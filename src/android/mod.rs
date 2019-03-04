macro_rules! jvalue_panic { ($msg:expr) => {
    panic!("Incorrect JValue, Expected {} ({}:{},{})", $msg, file!(), line!(), column!())
} }

macro_rules! get_jvalue { ($type:ident, $to_get:expr ) => {
    if let ::jni::objects::JValue::$type(v) = $to_get {
        v
    } else {
        jvalue_panic!(stringify!($type))
    }
}}

macro_rules! bluetooth_class { ( $name:expr ) => { concat!("android/bluetooth/", $name) } }
macro_rules! bluetooth_le_class { ( $name:expr) => { bluetooth_class!(concat!("le/", $name)) } }

mod classbytes;

pub mod le;
pub mod gatt;

use std::collections::HashSet;
use std::cmp::{
    Eq,
    PartialEq,
};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

trait MakeJavaUUID {
    fn make_java_uuid<'a>(&self, jenv: &'a jni::JNIEnv<'a>) -> jni::objects::JObject<'a>;
    fn from_java_uuid<'a>(jenv: Rc<jni::JNIEnv<'a>>, uuid: jni::objects::JObject<'a>) -> Self;
}

impl MakeJavaUUID for ::UUID {
    fn make_java_uuid<'a>(&self, jenv: &'a jni::JNIEnv<'a>) -> jni::objects::JObject<'a> {
        let lower = jni::objects::JValue::Long( <u128>::from(*self) as i64 );
        let upper = jni::objects::JValue::Long( (<u128>::from(*self) >> 64) as i64 );

        jenv.new_object(
            "java/util/UUID",
            "(JJ)V",
            &[upper, lower]
        )
        .unwrap()
    }

    fn from_java_uuid<'a>(jenv: Rc<jni::JNIEnv<'a>>, juuid: jni::objects::JObject<'a>) -> Self {
        let lower = get_jvalue!{
            Long,
            jenv.call_method(
                juuid,
                "getLeastSignificantBits",
                "()J",
                &[]
            ).unwrap()
        };

        let upper = get_jvalue!{
            Long,
            jenv.call_method(
                juuid,
                "getMostSignificantBits",
                "()J",
                &[]
            ).unwrap()
        };

        // The double cast on lower is done because jlong is logically converted to a u128.
        // This doesn't need to be done for upper as the shift removes the logically added bits.
        ::UUID::from( ((upper as u128) << 64) | (lower as u64 as u128) )
    }
}

pub struct AndroidBluetooth<'a> {
    env: Rc<jni::JNIEnv<'a>>,
    adapter: BluetoothAdapter<'a>,
    devices: HashSet<BluetoothDevice<'a>>
}

impl<'a> AndroidBluetooth<'a> {
    pub fn new( jenv: Rc<jni::JNIEnv<'a>>, context: jni::objects::JObject<'a> ) -> Self {

        let bluetooth_manager = BluetoothManager::<'a>::new(jenv.clone(), context);

        AndroidBluetooth {
            env: jenv.clone(),
            adapter: bluetooth_manager.get_adapter(),
            devices: HashSet::new(),
        }
    }
}

/// A wrapper around a java ClassLoader object
pub struct ClassLoader<'a> {
    class_loader: jni::objects::JObject<'a>,
}

impl<'a> ClassLoader<'a> {

    /// Create a ClassLoader containing the jvm system class loader
    pub fn system(env: &'a jni::JNIEnv<'a>) -> Self {
        let class_loader = get_jvalue! {
            Object,
            env.call_static_method(
                "java/lang/ClassLoader",
                "getSystemClassLoader",
                "()Ljava/lang/ClassLoader;",
                &[])
            .unwrap()
        };

        ClassLoader {
            class_loader: class_loader
        }
    }

    /// Create a ClassLoader containig the classloader of the input `class`
    ///
    /// The input `class` must be a
    /// [java.lang.Class](https://developer.android.com/reference/java/lang/Class)
    /// object.
    pub fn delegate( env: &'a jni::JNIEnv<'a>, class: jni::objects::JClass<'a> ) -> Self{
        let class_loader = get_jvalue! {
            Object,
            env.call_static_method(
                class,
                "getClassLoader",
                "()Ljava/lang/ClassLoader;",
                &[])
            .unwrap()
        };

        ClassLoader {
            class_loader: class_loader
        }
    }

    pub fn into_inner(self) -> jni::objects::JObject<'a> {
        self.class_loader
    }
}

mod load_classbytes {
    use std::ffi::c_void;
    use std::ffi::CString;
    use android::ClassLoader;

    pub struct NativeMethod
    {
        /// The java method name
        method_name: CString,
        /// The signature of the method name
        method_signature: CString,
        /// A raw pointer to a rust function to register the java method with
        rust_function_ptr: *const c_void,
    }

    impl NativeMethod {
        /// Create a new NativeMethod
        ///
        /// Three things are needed, the name of the *java* method name, the jni signature of that
        /// method, and the rust function to be called by that method.
        ///
        /// # WARNING
        /// The rust function must be in the formation of a jni c function. This function must also
        /// be marked with the qualifier `extern "system"`. Because these restrictions cannot be
        /// enforced, the function is labeled unsafe.
        ///
        /// You'll get a "process crash" if you dont match the name, use an incorrect signature, or
        /// use an invalid pointer to a extern "system" function.
        pub unsafe fn new(name: CString, sig: CString, fn_ptr: *const c_void ) -> NativeMethod
        {
            NativeMethod {
                method_name: name,
                method_signature: sig,
                rust_function_ptr: fn_ptr,
            }
        }
    }

    /// Dynamically load delvik (dex) formatted classbytes into the JVM
    ///
    /// This should used wherever jni is not enough and a java class is required.
    ///
    /// Any native methods in the loaded class will need to be registered in order to work. To do
    /// that just use the add_native_method functions.
    pub struct DexBytesLoader<'a> {
        bytes: &'a [u8],
        class_name: &'a str,
        methods: Vec<NativeMethod>,
    }

    impl<'a> DexBytesLoader<'a> {

        pub fn new( dex_bytes: &'a [u8], class_name: &'a str ) -> Self {
            DexBytesLoader {
                bytes: dex_bytes,
                class_name: class_name,
                methods: Vec::new(),
            }
        }

        /// add a rust function to register with a native method
        pub fn add_native_method( mut self, native: NativeMethod ) -> Self {
            self.methods.push(native);
            self
        }

        /// Load the dex bytes into the jvm and register any native methods
        pub fn load(
            self,
            jenv: &'a jni::JNIEnv<'a>,
            class_loader: ClassLoader<'a> )
            -> jni::objects::JClass<'a>
        {
            use jni::objects::{JValue,JClass,JObject};
            use std::ffi::c_void;
            use std::os::raw::c_char;

            let class = jenv.get_object_class(class_loader.into_inner()).unwrap();

            let class_loader = jenv.call_method(
                class.into(),
                "getClassLoader",
                "()Ljava/lang/ClassLoader;",
                &[])
            .unwrap();

            let java_bytes = JObject::from( jenv.byte_array_from_slice(self.bytes).unwrap() );

            let class_byte_buffer = jenv.call_static_method(
                "java/nio/ByteBuffer",
                "wrap",
                "([B)Ljava/nio/ByteBuffer;",
                &[JValue::Object(java_bytes)]
            )
            .unwrap();

            let dex_class_loader = jenv.new_object(
                "dalvik/system/InMemoryDexClassLoader",
                "(Ljava/nio/ByteBuffer;Ljava/lang/ClassLoader;)V",
                &[class_byte_buffer, class_loader]
            )
            .unwrap();

            let class_name = jenv.new_string(self.class_name).unwrap().into();

            let loaded_class = JClass::from(
                get_jvalue! {
                    Object,
                    jenv.call_method(
                        dex_class_loader,
                        "loadClass",
                        "(Ljava/lang/String;)Ljava/lang/Class;",
                        &[JValue::Object(class_name)]
                    )
                    .unwrap()
                }
            );

            // For some reason dynamic memory doest work, idk why, so I did this.
            self.methods.into_iter().for_each( |native| {

                let jni_native = jni::sys::JNINativeMethod {
                    name: native.method_name.as_ptr() as *mut c_char,
                    signature: native.method_signature.as_ptr() as *mut c_char,
                    fnPtr: native.rust_function_ptr as *mut c_void ,
                };

                unsafe {

                    (**jenv.get_native_interface()).RegisterNatives.unwrap()(
                        jenv.get_native_interface(),
                        loaded_class.into_inner(),
                        &jni_native as *const _,
                        1
                    )
                };
            });

            loaded_class
        }
    }
}


#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phy {
    // LE 1M
    OneMbs,
    // LE 2M
    TwoMbs,
    // LE Coded
    Coded,
}

impl Phy {
    fn from_raw(raw: jni::sys::jint) -> Result<Self, jni::sys::jint> {
        match raw {
            // BluetoothDevice.PHY_LE_1M
            1 => Ok(Phy::OneMbs),
            // BluetoothDevice.PHY_LE_2M
            2 => Ok(Phy::TwoMbs),
            // BluetoothDevice.PHY_LE_CODED
            3 => Ok(Phy::Coded),
            // Unknown
            _ => Err(raw)
        }
    }

    fn val(&self) -> jni::sys::jint {
        match *self {
            Phy::OneMbs => 1,
            Phy::TwoMbs => 2,
            Phy::Coded  => 3,
        }
    }
}

pub enum PhyCodedType {
    Coded2M,
    Coded8M,
}

impl PhyCodedType {
    fn val_from_opt( val: Option<Self>) -> jni::sys::jint
    {
        match val {
            None => 0,
            Some(coded_type) => match coded_type {
                PhyCodedType::Coded2M => 1,
                PhyCodedType::Coded8M => 2,
            }
        }
    }
}

/// Bluetooth device identifier
#[derive(Clone)]
pub struct BluetoothDevice<'a> {
    jenv: Rc<jni::JNIEnv<'a>>,
    object: jni::sys::jobject,
}

impl<'a> BluetoothDevice<'a> {
    fn new( jenv: Rc<jni::JNIEnv<'a>>, object: jni::objects::JObject<'a> ) -> Self {
        BluetoothDevice {
            jenv: jenv.clone(),
            object: object.into_inner(),
        }
    }
}

impl<'a> PartialEq for BluetoothDevice<'a> {
    fn eq(&self, other: &BluetoothDevice) -> bool {

        let is_equal = get_jvalue!( Bool,
            self.jenv.call_method(
                jni::objects::JObject::from(self.object),
                "equals",
                "(Ljava/lang/Object;)Z",
                &[jni::objects::JValue::Object(jni::objects::JObject::from(other.object))])
            .unwrap()
        );

        // convert number to boolean
        is_equal != jni::sys::jboolean::default()
    }
}

impl<'a> Eq for BluetoothDevice<'a> {}

impl<'a> Hash for BluetoothDevice<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {

        // really only safe to use hashCode method
        get_jvalue!( Int,
            self.jenv.call_method(
                jni::objects::JObject::from(self.object),
                "hashCode",
                "()I",
                &[])
            .unwrap()
        )
        .hash(state);
    }
}

/// Bluetooth Manager
#[derive(Clone)]
pub struct BluetoothManager<'a> {
    jenv: Rc<jni::JNIEnv<'a>>,
    object: jni::sys::jobject,
}

impl <'a> BluetoothManager<'a> {
    pub fn new( jenv: Rc<jni::JNIEnv<'a>>, context: jni::objects::JObject<'a> ) -> Self {

        let bluetooth_service_field = jenv.get_static_field(
            "android/content/Context",
            "BLUETOOTH_SERVICE",
            "Ljava/lang/String;")
        .unwrap();

        let object = get_jvalue!(
            Object,
            jenv.call_method(
                context,
                "getSystemService",
                "(Ljava/lang/String;)Ljava/lang/Object;",
                &[bluetooth_service_field])
            .unwrap());

        BluetoothManager {
            jenv: jenv.clone(),
            object: object.into_inner(),
        }
    }

    pub fn get_adapter(&self) -> BluetoothAdapter<'a> {
        let object = get_jvalue!( Object,
            self.jenv.call_method(
                jni::objects::JObject::from(self.object),
                "getAdapter",
                format!("()L{};", bluetooth_class!("BluetoothAdapter")),
                &[])
            .unwrap());

        BluetoothAdapter {
            jenv: self.jenv.clone(),
            object: object.into_inner()
        }
    }
}

/// Bluetooth Adapter
#[derive(Clone)]
pub struct BluetoothAdapter<'a> {
    jenv: Rc<jni::JNIEnv<'a>>,
    object: jni::sys::jobject,
}

impl<'a> BluetoothAdapter<'a> {

    /// Disable bluetooth on the device
    ///
    /// Only user action should be used to drive this
    pub fn disable(&self) -> bool {
        get_jvalue!(Bool,
            self.jenv.call_method(
                jni::objects::JObject::from(self.object),
                "disable",
                "()Z",
                &[])
            .unwrap()
        ) != 0
    }

    /// Enable bluetooth on the device
    ///
    /// only user action should be used to drive this
    pub fn enable(&self) -> bool {
        get_jvalue!(Bool,
            self.jenv.call_method(
                jni::objects::JObject::from(self.object),
                "enable",
                "()Z",
                &[])
            .unwrap()
        ) != 0
    }

    /// Get the bluetooth device address
    pub fn get_address(&self) -> ::BluetoothDeviceAddress {
        let obj_addr = jni::objects::JString::from(
            get_jvalue!(Object,
                self.jenv.call_method(
                    jni::objects::JObject::from(self.object),
                    "getAddress",
                    "()Ljava/lang/String;",
                    &[])
                .unwrap()
            )
        );

        let addr_str: String = self.jenv.get_string(obj_addr)
            .expect("Couldn't get address as string")
            .into();

        ::bluetooth_address_from_string(&addr_str).expect("Address Incorrect")
    }

    /// Get the bluetooth le advertiser object
    pub fn get_bluetooth_le_advertisier(&'a self) -> le::advertise::BluetoothLeAdvertiser<'a> {
        let object = get_jvalue!( Object,
            self.jenv.call_method(
                jni::objects::JObject::from(self.object),
                "getBluetoothLeAdvertiser",
                format!("()L{};", bluetooth_le_class!("BluetoothLeAdvertiser")),
                &[])
            .unwrap()
        );

        le::advertise::BluetoothLeAdvertiser::new(self.jenv.clone(), object)
    }

    /// TODO, needs to be implemented
    pub fn get_bluetooth_le_scanner(&'a self) -> le::scan::BluetoothLeScanner<'a> {
        let object = get_jvalue!( Object,
            self.jenv.call_method(
                jni::objects::JObject::from(self.object),
                "getBluetoothLeScanner",
                format!("()L{};", bluetooth_le_class!("BluetoothLeScanner")),
                &[])
            .unwrap()
        );

        le::scan::BluetoothLeScanner::new(self.jenv.clone(), object)
    }

    pub fn get_bonded_devices(&'a self) -> Vec<BluetoothDevice<'a>> {

        let has_next = | set: jni::objects::JObject<'a> | -> bool {
            0 != get_jvalue!(Bool, self.jenv.call_method( set, "hasNext", "()Z", &[]).unwrap())
        };

        let jset = get_jvalue!(Object,
            self.jenv.call_method(
                jni::objects::JObject::from(self.object),
                "getBondedDevices",
                "()Ljava/util/Set;",
                &[])
            .unwrap()
        );

        let jitr = get_jvalue!(Object,
            self.jenv.call_method(
                jset,
                "iterator",
                "()Ljava/util/Iterator;",
                &[])
            .unwrap()
        );

        let mut devices = Vec::default();

        while has_next(jitr) {

            let j_bluetooth_device = get_jvalue!(
                Object,
                self.jenv.call_method(
                    jitr,
                    "next",
                    format!("()L{};", bluetooth_le_class!("BluetoothDevice")),
                    &[])
                .unwrap()
            );

            devices.push( BluetoothDevice::new(self.jenv.clone(), j_bluetooth_device) )
        }

        devices
    }

    pub fn get_le_maximum_advertising_data_length(&'a self) -> Option<usize> {
        let len = get_jvalue!(Int,
            self.jenv.call_method(
                jni::objects::JObject::from(self.object),
                "getLeMaximumAdvertisingDataLength",
                "()I",
                &[])
            .unwrap()
        ) as usize;

        if len != 0 { Some(len) } else { None }
    }

    pub fn get_name(&self) -> String {
        self.jenv.get_string(
            get_jvalue!(Object,
                self.jenv.call_method(
                    jni::objects::JObject::from(self.object),
                    "getName",
                    "()Ljava/lang/String;",
                    &[])
                .unwrap()
            ).into()
        ).expect("Couldn't get string name").into()
    }

    pub fn is_le_2m_phy_supported(&'a self) -> bool {
        get_jvalue!(Bool,
            self.jenv.call_method(
                jni::objects::JObject::from(self.object),
                "isLe2MPhySupported",
                "()Z",
                &[])
            .unwrap()
        ) != 0
    }

    pub fn is_le_coded_phy_supported(&'a self) -> bool {
        get_jvalue!(Bool,
            self.jenv.call_method(
                jni::objects::JObject::from(self.object),
                "isLeCodedPhySupported",
                "()Z",
                &[])
            .unwrap()
        ) != 0
    }

    pub fn is_periodic_advertising_supported(&'a self) -> bool {
        get_jvalue!(Bool,
            self.jenv.call_method(
                jni::objects::JObject::from(self.object),
                "isLePeriodicAdvertisingSupported",
                "()Z",
                &[])
            .unwrap()
        ) != 0
    }

    /// Returns an error if the name couldn't be made into a java string
    pub fn set_name(&'a self, name: &str) -> jni::errors::Result<bool>{
        let jni_str = self.jenv.new_string(name)?.into_inner();

        Ok( get_jvalue!(Bool,
            self.jenv.call_method(
                jni::objects::JObject::from(self.object),
                "setName",
                "(Ljava/lang/String;)Z",
                &[jni::objects::JValue::Object(jni::objects::JObject::from(jni_str))]
            )
            .unwrap()
        ) != 0 )
    }

    pub fn is_enabled(&self) -> bool {
        get_jvalue!(Bool,
            self.jenv.call_method(
                jni::objects::JObject::from(self.object),
                "isEnabled",
                "()Z",
                &[])
            .unwrap()
        ) != 0
    }

    pub fn get_remote_device(&self, address: ::BluetoothDeviceAddress) -> BluetoothDevice {
        let address_string = ::bluetooth_address_into_string(address);

        let j_string = self.jenv.new_string(address_string).unwrap();

        let object = get_jvalue!{
            Object,
            self.jenv.call_method(
                self.object.into(),
                "getRemoteDevice",
                format!("(Ljava/lang/String;){}", bluetooth_le_class!("BluetoothDevice")),
                &[jni::objects::JValue::Object(j_string.into())])
            .unwrap()
        };

        BluetoothDevice {
            jenv: self.jenv.clone(),
            object: object.into_inner()
        }
    }
}

#[cfg(feature = "android_test")]
mod tests {
    extern crate android_test;

    use jni::JNIEnv;
    use jni::objects::{JObject,JValue};
    use self::android_test::android_test;
    use super::*;

    /// Get the target context
    ///
    /// Calls androidx.test.platform.app.InstrumentationRegistry.getTargetContext()
    pub fn get_target_context<'a>(env: &'a JNIEnv<'a>) -> JObject<'a> {
        let instrumentor = match env.call_static_method(
            "androidx/test/platform/app/InstrumentationRegistry",
            "getInstrumentation",
            "()Landroid/app/Instrumentation;",
            &[]
        ) {
            Ok(JValue::Object(obj)) => obj,
            Err(err) => panic!("Couldn't get instrumentor: {}", err),
            _ => panic!("Returned value wasn't an object"),
        };

        match env.call_method(
            instrumentor,
            "getTargetContext",
            "()Landroid/content/Context;",
            &[]
        ) {
            Ok(JValue::Object(obj)) => obj,
            Err(err) => panic!("Couldn't get instrumentor: {}", err),
            _ => panic!("Returned value wasn't an object"),
        }
    }

    /// Just to make sure that the rest of the tests are not tripping on themselves just because
    /// the function get_context isn't working.
    #[android_test]
    fn get_context_test(env: JNIEnv) {
        get_target_context(&env);
    }

    #[android_test( permissions=( "BLUETOOTH", "BLUETOOTH_ADMIN", "ACCESS_COARSE_LOCATION", ))]
    fn get_bluetooth(env: JNIEnv) {
        let renv = Rc::new(env);
        AndroidBluetooth::new( renv.clone(), get_target_context(renv.as_ref()));
    }

    #[android_test]
    fn get_bluetooth_manager(env: JNIEnv) {
        let rc_jenv = Rc::new(env);
        let context = get_target_context(rc_jenv.as_ref());
        BluetoothManager::new(rc_jenv.clone(), context);
    }

    #[android_test]
    fn get_bluetooth_adapter(env: JNIEnv) {
        let rc_jenv = Rc::new(env);
        let context = get_target_context(rc_jenv.as_ref());
        let manager = BluetoothManager::new(rc_jenv.clone(), context);
        manager.get_adapter();
    }

    #[android_test ( permissions=( "BLUETOOTH_ADMIN" ))]
    fn enable_disable(env: JNIEnv ) {
        let rc_jenv = Rc::new(env);
        let context = get_target_context(rc_jenv.as_ref());
        let manager = BluetoothManager::new(rc_jenv.clone(), context);
        let adapter = manager.get_adapter();

        if adapter.is_enabled() {
            assert!(adapter.disable());
        } else {
            assert!(adapter.enable());
            assert!(adapter.disable());
        }

        assert!(adapter.enable());
    }

    #[android_test (permissions=( "BLUETOOTH" ))]
    fn get_address(env: JNIEnv) {
        let rc_jenv = Rc::new(env);
        let context = get_target_context(rc_jenv.as_ref());
        let manager = BluetoothManager::new(rc_jenv.clone(), context);
        let adapter = manager.get_adapter();
        adapter.get_address();
    }

    #[android_test]
    fn get_bluetooth_advertiser(env: JNIEnv) {
        let rc_jenv = Rc::new(env);
        let context = get_target_context(rc_jenv.as_ref());
        let manager = BluetoothManager::new(rc_jenv.clone(), context);
        let adapter = manager.get_adapter();
        adapter.get_bluetooth_le_advertisier();
    }

    #[android_test (ignore)]
    fn get_bluetooth_scanner(env: JNIEnv) {
        let rc_jenv = Rc::new(env);
        let context = get_target_context(rc_jenv.as_ref());
        let manager = BluetoothManager::new(rc_jenv.clone(), context);
        let adapter = manager.get_adapter();
        adapter.get_bluetooth_le_scanner();
    }

    #[android_test (permissions=( "BLUETOOTH" ))]
    fn get_bonded_devices(env: JNIEnv) {
        let rc_jenv = Rc::new(env);
        let context = get_target_context(rc_jenv.as_ref());
        let manager = BluetoothManager::new(rc_jenv.clone(), context);
        let adapter = manager.get_adapter();
        adapter.get_bonded_devices();
    }

    #[android_test]
    fn get_le_extended_adv_max_len(env: JNIEnv) {
        let rc_jenv = Rc::new(env);
        let context = get_target_context(rc_jenv.as_ref());
        let manager = BluetoothManager::new(rc_jenv.clone(), context);
        let adapter = manager.get_adapter();
        adapter.get_le_maximum_advertising_data_length();
    }

    #[android_test (permissions = ( "BLUETOOTH", "BLUETOOTH_ADMIN" ))]
    fn get_set_name(env: JNIEnv) {
        let rc_jenv = Rc::new(env);
        let context = get_target_context(rc_jenv.as_ref());
        let manager = BluetoothManager::new(rc_jenv.clone(), context);
        let adapter = manager.get_adapter();
        let name = adapter.get_name();
        adapter.set_name(&name).expect("Couldn't set name");
    }

    #[android_test]
    fn le_supported_phy(env: JNIEnv) {
        let rc_jenv = Rc::new(env);
        let context = get_target_context(rc_jenv.as_ref());
        let manager = BluetoothManager::new(rc_jenv.clone(), context);
        let adapter = manager.get_adapter();
        adapter.is_le_2m_phy_supported();
        adapter.is_le_coded_phy_supported();
    }

    #[android_test]
    fn periodic_adv_supported(env: JNIEnv) {
        let rc_jenv = Rc::new(env);
        let context = get_target_context(rc_jenv.as_ref());
        let manager = BluetoothManager::new(rc_jenv.clone(), context);
        let adapter = manager.get_adapter();
        adapter.is_periodic_advertising_supported();
    }
}
