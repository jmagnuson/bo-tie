use gap::advertise::service_class_uuid::Services;
use gap::advertise::service_data::ServiceData;
use std::option::Option;
use std::rc::Rc;
use std::time::Duration;

#[derive(Clone,Copy,Debug)]
pub enum AdvertiseError {
    AlreadyStarted,
    DataTooLarge,
    FeatureUnsupported,
    InternalError,
    TooManyAdvertisers,
    UnknownCode(jni::sys::jint)
}

impl AdvertiseError {
    fn from_raw(code: jni::sys::jint) -> Self {
        match code {
            1 => AdvertiseError::DataTooLarge,
            2 => AdvertiseError::TooManyAdvertisers,
            3 => AdvertiseError::AlreadyStarted,
            4 => AdvertiseError::InternalError,
            5 => AdvertiseError::FeatureUnsupported,
            _ => AdvertiseError::UnknownCode(code),
        }
    }
}

impl std::fmt::Display for AdvertiseError {
    fn fmt(&self, f: &mut std::fmt::Formatter ) -> Result<(), std::fmt::Error >{
        match self {
            AdvertiseError::DataTooLarge => {
                write!(f, "Advertise data is larger than 31 bytes")
            }
            AdvertiseError::TooManyAdvertisers => {
                write!(f, "No advertising instance available")
            }
            AdvertiseError::AlreadyStarted => {
                write!(f, "Advertising already started")
            }
            AdvertiseError::InternalError => {
                write!(f, "Internal error occurd")
            }
            AdvertiseError::FeatureUnsupported => {
                write!(f, "Advertising unsupported by this platform")
            }
            AdvertiseError::UnknownCode(val) => {
                write!(f, "Unknown error code: {}", val)
            }
        }
    }
}


#[derive(Clone)]
pub struct BluetoothLeAdvertiser<'a> {
    jenv: Rc<jni::JNIEnv<'a>>,
    object: jni::sys::jobject,
}

impl<'a> BluetoothLeAdvertiser<'a> {
    pub(crate) fn new(jenv: Rc<jni::JNIEnv<'a>>, object: jni::objects::JObject<'a> ) -> Self {
        BluetoothLeAdvertiser {
            jenv: jenv,
            object: object.into_inner(),
        }
    }

    pub fn start_advertising(
        &self,
        settings: AdvertiseSettings,
        data: AdvertisingData,
        callback: AdvertiseCallback )
    {
        self.jenv.call_method(
            jni::objects::JObject::from(self.object),
            "startAdvertising",
            format!("(L{};L{};L{};)V",
                bluetooth_le_class!("AdvertiseSettings"),
                bluetooth_le_class!("AdvertiseData"),
                bluetooth_le_class!("AdvertiseCallback")
            ),
            &[
                jni::objects::JValue::Object(jni::objects::JObject::from(settings.object)),
                jni::objects::JValue::Object(jni::objects::JObject::from(data.object)),
                jni::objects::JValue::Object(jni::objects::JObject::from(callback.object)),
            ])
        .expect("Start Advertise failed");
    }

    pub fn stop_advertising(&self, callback: AdvertiseCallback)
    {
        self.jenv.call_method(
            jni::objects::JObject::from(self.object),
            "stopAdvertising",
            format!("(L{};)V", bluetooth_le_class!("AdvertiseCallback")),
            &[jni::objects::JValue::Object(jni::objects::JObject::from(callback.object))])
        .expect("Stop Advertise failed");
    }
}

/// Mode returned from AdvertiseSettings::get_mode
#[derive(Clone,Copy,Debug,PartialEq)]
pub enum Mode {
    Balanced,
    LowLatency,
    LowPower,
}

/// Tx Power Level returned from AdvertiseSettings::get_tx_power_level
#[derive(Clone,Copy,Debug,PartialEq)]
pub enum TxPowerLevel {
    High,
    Low,
    Medium,
    UltraLow,
}

/// Settings for advertising
///
/// This can be created through AdvertiseSettingsBuilder
#[derive(Clone)]
pub struct AdvertiseSettings<'a> {
    jenv: Rc<jni::JNIEnv<'a>>,
    object: jni::sys::jobject,
    mode: Mode,
    connectable: bool,
    tx_power_level: TxPowerLevel,
    timeout: Duration,
}

impl<'a> AdvertiseSettings<'a> {

    /// Get the mode
    ///
    /// On android this is either balanced, low latency, or low power
    pub fn get_mode(&self) -> Mode {
        self.mode
    }

    pub fn is_connectable(&self) -> bool {
        self.connectable
    }

    pub fn get_timeout(&self) -> Duration {
        self.timeout
    }

    pub fn get_tx_power_level(&self) -> TxPowerLevel {
        self.tx_power_level
    }

    fn get_jint_const(jenv: &jni::JNIEnv, name: &'static str) -> jni::sys::jint {
        get_jvalue! {
            Int,
            jenv.get_static_field(
                bluetooth_le_class!("AdvertiseSettings"),
                name,
                "I"
            )
            .unwrap()
        }
    }

    fn j_get_mode(jenv: &jni::JNIEnv, object: jni::objects::JObject) -> Mode {
        let mode = get_jvalue!{
            Int,
            jenv.call_method(
                object,
                "getMode",
                "()I",
                &[])
            .unwrap()
        };

        if mode == Self::get_jint_const(jenv, "ADVERTISE_MODE_BALANCED") {
            Mode::Balanced
        }
        else if mode == Self::get_jint_const(jenv, "ADVERTISE_MODE_LOW_LATENCY") {
            Mode::LowLatency
        }
        else if mode == Self::get_jint_const(jenv, "ADVERTISE_MODE_LOW_POWER") {
            Mode::LowPower
        }
        else {
            panic!("Unknown advertise mode returned from android bluetooth driver")
        }
    }

    fn j_get_connectable_flag(jenv: &jni::JNIEnv, object: jni::objects::JObject) -> bool {
        let connectable = get_jvalue!{
            Bool,
            jenv.call_method(
                object,
                "isConnectable",
                "()Z",
                &[])
            .unwrap()
        };

        connectable != jni::sys::jboolean::default()
    }

    fn j_get_timeout(jenv: &jni::JNIEnv, object: jni::objects::JObject) -> Duration {
        let ms = get_jvalue!{
            Int,
            jenv.call_method(
                object,
                "getTimeout",
                "()I",
                &[])
            .unwrap()
        };

        Duration::from_millis(ms as u64)
    }

    fn j_get_tx_power_level(jenv: &jni::JNIEnv, object: jni::objects::JObject) -> TxPowerLevel {
        let power_level = get_jvalue! {
            Int,
            jenv.call_method(
                object,
                "getTxPowerLevel",
                "()I",
                &[])
            .unwrap()
        };

        if power_level == Self::get_jint_const(jenv, "ADVERTISE_TX_POWER_HIGH") {
            TxPowerLevel::High
        }
        else if power_level == Self::get_jint_const(jenv, "ADVERTISE_TX_POWER_LOW") {
            TxPowerLevel::Low
        }
        else if power_level == Self::get_jint_const(jenv, "ADVERTISE_TX_POWER_MEDIUM") {
            TxPowerLevel::Medium
        }
        else if power_level == Self::get_jint_const(jenv, "ADVERTISE_TX_POWER_ULTRA_LOW") {
            TxPowerLevel::UltraLow
        }
        else {
            panic!("Unknown advertise tx power level returned from android bluetooth driver")
        }
    }

    /// Object must be of the type
    /// [AdvertiseSettings](https://developer.android.com/reference/android/bluetooth/le/AdvertiseSettings)
    fn from_raw( jenv: Rc<jni::JNIEnv<'a>>, object: jni::objects::JObject ) -> Self {
        // piggy-back off some methods from AdvertiseSettingsBuilder
        Self {
            mode: AdvertiseSettings::j_get_mode(jenv.as_ref(), object),

            connectable: AdvertiseSettings::j_get_connectable_flag(jenv.as_ref(), object),

            timeout: AdvertiseSettings::j_get_timeout(jenv.as_ref(), object),

            tx_power_level: AdvertiseSettings::j_get_tx_power_level(jenv.as_ref(), object),

            jenv: jenv.clone(),
            object: object.into_inner(),
        }
    }
}

#[derive(Clone)]
pub struct AdvertiseSettingsBuilder<'a> {
    jenv: Rc<jni::JNIEnv<'a>>,
    object: jni::sys::jobject,
    mode: Option<Mode>,
    connectable: Option<bool>,
    timeout: Option<Duration>,
    tx_power_level: Option<TxPowerLevel>,
}

macro_rules! advertise_settings_builder_java_name{ () => { "AdvertiseSettings$Builder" }}

impl<'a> AdvertiseSettingsBuilder<'a> {

    pub fn new( jenv: Rc<jni::JNIEnv<'a>> ) -> Self {
        let object = jenv.new_object(
            bluetooth_le_class!(advertise_settings_builder_java_name!()),
            "()V",
            &[])
        .unwrap();

        AdvertiseSettingsBuilder {
            jenv: jenv.clone(),
            object: object.into_inner(),
            mode: None,
            connectable: None,
            timeout: None,
            tx_power_level: None,
        }
    }

    pub fn set_mode(self, mode: Mode ) -> Self {
        let jvalue_mode = self.jenv.get_static_field(
            bluetooth_le_class!("AdvertiseSettings"),
            match mode {
                Mode::Balanced => "ADVERTISE_MODE_BALANCED",
                Mode::LowLatency => "ADVERTISE_MODE_LOW_LATENCY",
                Mode::LowPower => "ADVERTISE_MODE_LOW_POWER",
            },
            "I")
        .unwrap();

        let object = get_jvalue!{
            Object,
            self.jenv.call_method(
                jni::objects::JObject::from(self.object),
                "setAdvertiseMode",
                format!("(I)L{};", bluetooth_le_class!(advertise_settings_builder_java_name!())),
                &[jvalue_mode])
            .unwrap()
        };

        let mut ret = self.clone();

        ret.object = object.into_inner();
        ret.mode = Some(mode);

        ret
    }

    pub fn set_connectable( self, val: bool ) -> Self {
        let object = get_jvalue!{
            Object,
            self.jenv.call_method(
                jni::objects::JObject::from(self.object),
                "setConnectable",
                format!("(Z)L{};", bluetooth_le_class!(advertise_settings_builder_java_name!())),
                &[jni::objects::JValue::Bool(val as jni::sys::jboolean)])
            .unwrap()
        };

        let mut ret = self.clone();

        ret.object = object.into_inner();
        ret.connectable = Some(val);

        ret
    }

    /// Set the advertising timeout in milliseconds
    ///
    /// The android api sets a hard limit at 180 seconds
    pub fn set_timeout( self, time: Duration ) -> Self {
        use jni::sys::jint;

        let ms = (time.as_secs() as jint) * 1000 + (time.subsec_millis() as jint);

        let object = get_jvalue!{
            Object,
            self.jenv.call_method(
                jni::objects::JObject::from(self.object),
                "setTimeout",
                format!("(I)L{};", bluetooth_le_class!(advertise_settings_builder_java_name!())),
                &[jni::objects::JValue::Int(ms)])
            .unwrap()
        };

        let mut ret = self.clone();

        ret.object = object.into_inner();
        ret.timeout = Some(time);

        ret
    }

    pub fn set_tx_power_level( self, level: TxPowerLevel ) -> Self {
        let jvalue_power_level = self.jenv.get_static_field(
            bluetooth_le_class!("AdvertiseSettings"),
            match level {
                TxPowerLevel::High => "ADVERTISE_TX_POWER_HIGH",
                TxPowerLevel::Low => "ADVERTISE_TX_POWER_LOW",
                TxPowerLevel::Medium => "ADVERTISE_TX_POWER_MEDIUM",
                TxPowerLevel::UltraLow => "ADVERTISE_TX_POWER_ULTRA_LOW",
            },
            "I")
        .unwrap();

        let object = get_jvalue!{
            Object,
            self.jenv.call_method(
                jni::objects::JObject::from(self.object),
                "setTxPowerLevel",
                format!("(I)L{};", bluetooth_le_class!(advertise_settings_builder_java_name!())),
                &[jvalue_power_level])
            .unwrap()
        };

        let mut ret = self.clone();

        ret.object = object.into_inner();
        ret.tx_power_level = Some(level);

        ret
    }

    fn get_mode<'b>(&self, advertise_settings: jni::objects::JObject<'b>) -> Mode {
        match self.mode {
            Some(mode) => mode,
            None => AdvertiseSettings::j_get_mode(self.jenv.as_ref(), advertise_settings),
        }
    }

    fn get_connectable_flag<'b>(&self, advertise_settings: jni::objects::JObject<'b> ) -> bool {
        match self.connectable {
            Some(connectable) => connectable,
            None => AdvertiseSettings::j_get_connectable_flag(self.jenv.as_ref(), advertise_settings),
        }
    }

    fn get_timeout<'b>(&self, advertise_settings: jni::objects::JObject<'b> ) -> Duration {
        match self.timeout {
            Some(duration) => duration,
            None => AdvertiseSettings::j_get_timeout(self.jenv.as_ref(), advertise_settings),
        }
    }

    fn get_tx_power_level<'b>(&self, advertise_settings: jni::objects::JObject<'b> ) -> TxPowerLevel {
        match self.tx_power_level {
            Some(level) => level,
            None => AdvertiseSettings::j_get_tx_power_level(self.jenv.as_ref(), advertise_settings),
        }
    }

    pub fn build(self) -> AdvertiseSettings<'a> {
        let object = get_jvalue! {
            Object,
            self.jenv.call_method(
                jni::objects::JObject::from(self.object),
                "build",
                format!("()L{};", bluetooth_le_class!("AdvertiseSettings")),
                &[])
            .unwrap()
        };

        AdvertiseSettings {
            mode: self.get_mode(object),
            connectable: self.get_connectable_flag(object),
            timeout: self.get_timeout(object),
            tx_power_level: self.get_tx_power_level(object),
            jenv: self.jenv.clone(),
            object: object.into_inner(),
        }
    }
}

#[derive(Clone)]
pub struct AdvertisingData<'a> {
    jenv: Rc<jni::JNIEnv<'a>>,
    object: jni::sys::jobject,
    device_name_included: bool,
    tx_power_level_included: bool,
    service_uuids: Services<u128>,
    service_data: Vec<ServiceData<u128>>,
}

impl<'a> AdvertisingData<'a> {
    pub fn is_device_name_included(&self) -> bool { self.device_name_included }

    pub fn is_tx_power_level_included(&self) -> bool { self.tx_power_level_included }

    pub fn get_service_uuids(&self) -> Services<u128> { self.service_uuids.clone() }

    pub fn get_service_data(&self) -> Vec<ServiceData<u128>> { self.service_data.clone() }
}

#[derive(Clone)]
pub struct AdvertisingDataBuilder<'a> {
    jenv: Rc<jni::JNIEnv<'a>>,
    object: jni::sys::jobject,
    include_device_name: bool,
    include_tx_power_level: bool,
    service_uuids: Services<u128>,
    service_data: Vec<ServiceData<u128>>,
}

macro_rules! advertise_data_builder_java_name{ () => { "AdvertiseData$Builder" }}

impl<'a> AdvertisingDataBuilder<'a> {

    /// Creates a new AdvertisingDataBuilder
    ///
    /// By default, the device name is not included, the tx power level is not included,
    /// no service UUIDs are included, and no service data is added.
    pub fn new( jenv: Rc<jni::JNIEnv<'a>> ) -> Self {
        let object = jenv.new_object(
            bluetooth_le_class!(advertise_data_builder_java_name!()),
            "()V",
            &[])
        .unwrap();

        AdvertisingDataBuilder {
            jenv: jenv.clone(),
            object: object.into_inner(),
            include_device_name: false,
            include_tx_power_level: false,
            service_uuids: ::gap::advertise::service_class_uuid::new_128(true),
            service_data: Vec::new(),
        }
    }

    pub fn include_device_name( self, val: bool ) -> Self {
        let object = get_jvalue!{
            Object,
            self.jenv.call_method(
                jni::objects::JObject::from(self.object),
                "setIncludeDeviceName",
                format!("(Z)L{};", bluetooth_le_class!(advertise_data_builder_java_name!())),
                &[jni::objects::JValue::Bool(val as jni::sys::jboolean)])
            .unwrap()
        };

        let mut ret = self.clone();

        ret.object = object.into_inner();
        ret.include_device_name = val;

        ret
    }

    pub fn include_tx_power_level( self, val: bool ) -> Self {

        let object = get_jvalue!{
            Object,
            self.jenv.call_method(
                jni::objects::JObject::from(self.object),
                "setIncludeTxPowerLevel",
                format!("(Z)L{};", bluetooth_le_class!(advertise_data_builder_java_name!())),
                &[jni::objects::JValue::Bool(val as jni::sys::jboolean)])
            .unwrap()
        };

        let mut ret = self.clone();

        ret.object = object.into_inner();
        ret.include_tx_power_level = val;

        ret
    }

    fn get_uuid_parcel( &'a self, uuid: u128 ) -> jni::objects::JObject<'a> {
        let lower = jni::objects::JValue::Long(uuid as jni::sys::jlong);
        let upper = jni::objects::JValue::Long((uuid >> 64) as jni::sys::jlong);

        let java_uuid = self.jenv.new_object(
            "java/util/UUID",
            "(JJ)V",
            &[upper, lower])
        .unwrap();

        self.jenv.new_object(
            "android/os/ParcelUuid",
            "(Ljava/util/UUID;)V",
            &[jni::objects::JValue::Object(java_uuid)])
        .unwrap()
    }

    pub fn add_service_data( self, service_data: ServiceData<u128> ) -> Self {

        let parcel = jni::objects::JValue::Object(self.get_uuid_parcel(service_data.get_uuid()));

        let bytes = jni::objects::JValue::Object(
            jni::objects::JObject::from(
                self.jenv.byte_array_from_slice(service_data.serialized_data.as_slice()).unwrap()
            )
        );

        let object = get_jvalue!{
            Object,
            self.jenv.call_method(
                jni::objects::JObject::from(self.object),
                "addServiceData",
                format!("(Landroid/os/ParcelUuid;[B)L{};",
                    bluetooth_le_class!(advertise_data_builder_java_name!())),
                &[parcel, bytes])
            .unwrap()
        };

        let mut ret = self.clone();

        ret.object = object.into_inner();
        ret.service_data.push(service_data);

        ret
    }

    pub fn add_service_uuid( self, service_uuids: Services<u128> ) -> Self {
        let mut object = jni::objects::JObject::from(self.object);

        let mut ret = self.clone();
        ret.service_uuids = service_uuids.clone();

        for service in service_uuids {
            let parcel = jni::objects::JValue::Object(self.get_uuid_parcel(service));

            object = get_jvalue!{
                Object,
                self.jenv.call_method(
                    jni::objects::JObject::from(self.object),
                    "addServiceUuid",
                    format!("(Landroid/os/ParcelUuid;)L{};",
                        bluetooth_le_class!(advertise_data_builder_java_name!())),
                    &[parcel])
                .unwrap()
            };
        }

        ret.object = object.into_inner();

        ret
    }

    pub fn build(self) -> AdvertisingData<'a> {
        let object = get_jvalue! {
            Object,
            self.jenv.call_method(
                jni::objects::JObject::from(self.object),
                "build",
                format!("()L{};", bluetooth_le_class!("AdvertiseData")),
                &[])
            .unwrap()
        };

        AdvertisingData {
            jenv: self.jenv.clone(),
            object: object.into_inner(),
            device_name_included: self.include_device_name,
            tx_power_level_included: self.include_tx_power_level,
            service_uuids: self.service_uuids,
            service_data: self.service_data,
        }
    }
}

/// This module is just for containg some things for AdvertiseCallback
mod callback {
    use std::sync::{Arc,Mutex};
    use super::{AdvertiseSettings, AdvertiseError};

    pub struct AdvertiseCallbackFunctions<'s,F,S>
    where F: Fn(AdvertiseError) + Send + ?Sized,
          S: Fn(AdvertiseSettings<'s>) + Send + ?Sized
    {
        pub on_failure: Option<Box<F>>,
        pub on_success: Option<Box<S>>,
        pub(super) pd: std::marker::PhantomData<&'s ()>
    }

    pub type HashMap<'s> = std::collections::HashMap<jni::sys::jint, AdvertiseCallbackFunctions<
        's,
        dyn Fn(AdvertiseError) + Send,
        dyn Fn(AdvertiseSettings<'s>) + Send
    >>;

    pub type Map<'s> = Arc<Mutex<HashMap<'s>>>;
}

/// This is a wrapper around a java class that is the actuall callback.
///
/// The class can be found at src/android/java/botie/AdvertiseCallback.java
pub struct AdvertiseCallback<'a>
{
    jenv: Rc<jni::JNIEnv<'a>>,
    /// This is an object that is created through reflection. Method calls of the class need to be
    /// made with reflection.
    object: jni::sys::jobject,

    #[cfg(feature = "android_test")]
    class: &'a jni::objects::GlobalRef,
}


impl<'a> AdvertiseCallback<'a>
{
    #[inline]
    fn get_map_lock() -> Box< std::ops::DerefMut< Target=callback::HashMap<'static> > + 'static >
    {
        use std::collections::HashMap;
        use std::sync::{Arc,Mutex};
        use self::callback::Map;

        lazy_static! {
            pub static ref MAP: Map<'static> = Arc::new(Mutex::new(HashMap::new()));
        }

        Box::new(MAP.lock().expect("AdvertiseCallback MAP poisoned!"))
    }

    fn new(
        jenv: Rc<jni::JNIEnv<'a>>,
        classloader: ::android::ClassLoader,
        builder: AdvertiseCallbackBuilder<'static>)
        -> Self
    {
        use android::load_classbytes::{DexBytesLoader, NativeMethod};
        use std::ffi::CString;
        use std::sync::Once;
        use jni::objects::GlobalRef;

        static CLASS_NAME: &'static str = "botie/AdvertiseCallback";
        static mut CLASS: Option<GlobalRef> = None;
        static ONCE: Once = Once::new();

        ONCE.call_once(|| {
            let class = DexBytesLoader::new(
                    &::android::classbytes::ADVERTISE_CALLBACK_CLASS_BYTES,
                    CLASS_NAME)
                .add_native_method (
                    unsafe { NativeMethod::new(
                        CString::new("onStartFailure").unwrap(),
                        CString::new("(I)V").unwrap(),
                        Self::on_start_failure as *const std::ffi::c_void,
                    ) })
                .add_native_method (
                    unsafe { NativeMethod::new(
                        CString::new("onStartSuccess").unwrap(),
                        CString::new("(Landroid/bluetooth/le/AdvertiseSettings;)V").unwrap(),
                        Self::on_start_success as *const std::ffi::c_void,
                    ) })
                .add_native_method (
                    unsafe { NativeMethod::new(
                        CString::new("cleanBotie").unwrap(),
                        CString::new("()V").unwrap(),
                        Self::clean as *const std::ffi::c_void,
                    ) })
                .load(jenv.as_ref(), classloader);

            unsafe {
                CLASS = Some(jenv.new_global_ref(class.into()).expect(
                    "Couldn't create global reference for AdvertiseCallback java class"));
            }
        });

        let object = get_jvalue! {
            Object,
            jenv.call_method(
                unsafe { CLASS.as_ref().unwrap().as_obj() },
                "newInstance",
                "()Ljava/lang/Object;",
                &[]
            )
            .unwrap()
        };

        if let Some(_) = Self::get_map_lock()
            .insert( Self::get_hash_code(jenv.as_ref(), object), builder.callbacks )
        {
            panic!("Attempted to insert a duplicate object hashCode (as key) into \
                callback::ADVERTISE_CALLBACK_MAP");
        }

        #[cfg(feature = "android_test")]
        macro_rules! new_self {
            () => {
                Self {
                    jenv: jenv.clone(),
                    object: object.into_inner(),
                    class: unsafe{ CLASS.as_ref().unwrap() },
                }
            }
        }

        #[cfg(not(feature = "android_test"))]
        macro_rules! new_self {
            () => {
                Self {
                    jenv: jenv.clone(),
                    object: object.into_inner(),
                }
            }
        }

        new_self!()
    }

    fn get_hash_code(jenv: &jni::JNIEnv, object: jni::objects::JObject ) -> jni::sys::jint {
        get_jvalue! {
            Int,
            jenv.call_method(
                object,
                "hashCode",
                "()I",
                &[]
            )
            .unwrap()
        }
    }

    extern "system" fn on_start_failure(
        jenv: jni::JNIEnv,
        object: jni::objects::JObject,
        code: jni::sys::jint)
    {
        if let Some(ref callback) = Self::get_map_lock()
            .get( &Self::get_hash_code(&jenv, object) )
            .expect("Cannot get object from AdvertiseCallback's static map")
            .on_failure
        {
            callback(AdvertiseError::from_raw(code));
        }
    }

    extern "system" fn on_start_success(
        jenv: jni::JNIEnv<'static>,
        object: jni::objects::JObject,
        advertise_settings: jni::objects::JObject )
    {
        let rc_env = Rc::new(jenv);

        if let Some(ref callback) = Self::get_map_lock()
            .get( &Self::get_hash_code(rc_env.as_ref(), object) )
            .expect("Cannot get object from AdvertiseCallback's static map")
            .on_success
        {
            callback( AdvertiseSettings::from_raw( rc_env, advertise_settings ) );
        }
    }

    /// This can only be used for cleaning up callback::ADVERTISE_CALLBACK_MAP.
    extern "system" fn clean( jenv : jni::JNIEnv, object: jni::objects::JObject ) {
        Self::get_map_lock().remove( &Self::get_hash_code(&jenv, object) )
            .expect("Cannot get object from AdvertiseCallback's static map");
    }
}

pub struct AdvertiseCallbackBuilder<'s> {
    callbacks: callback::AdvertiseCallbackFunctions<
        's,
        dyn Fn(AdvertiseError) + Send,
        dyn Fn(AdvertiseSettings<'s>) + Send >,
}

impl<'s> AdvertiseCallbackBuilder<'s>
{
    pub fn new() -> Self {
        AdvertiseCallbackBuilder {
            callbacks: callback::AdvertiseCallbackFunctions {
                on_failure: None,
                on_success: None,
                pd: std::marker::PhantomData
            }
        }
    }

    pub fn set_on_start_failure_callback<FailFun>(self, callback: FailFun) -> Self
    where FailFun: 'static + std::marker::Unsize< dyn Fn(AdvertiseError) + Send > + Sized
    {
        Self {
            callbacks: callback::AdvertiseCallbackFunctions {
                on_failure: Some(Box::new(callback)),
                on_success: self.callbacks.on_success,
                pd: std::marker::PhantomData,
            }
        }
    }

    pub fn set_on_start_success_callback<SuccessFun>(self, callback: SuccessFun) -> Self
    where SuccessFun: 'static + std::marker::Unsize< dyn Fn(AdvertiseSettings<'s>) + Send > + Sized
    {
        Self {
            callbacks: callback::AdvertiseCallbackFunctions {
                on_failure: self.callbacks.on_failure,
                on_success: Some(Box::new(callback) as Box<dyn Fn(AdvertiseSettings<'s>) + Send>),
                pd: std::marker::PhantomData,
            }
        }
    }

    pub fn build<'j>(self, jenv: Rc<jni::JNIEnv<'j>>, class_loader: ::android::ClassLoader ) -> AdvertiseCallback<'j>
    where Self: 'static
    {
        AdvertiseCallback::new(jenv, class_loader, self)
    }
}

#[cfg(feature = "android_test")]
mod tests {
    extern crate android_test;

    use jni::JNIEnv;
    use self::android_test::android_test;
    use std::rc::Rc;
    use super::*;

    #[android_test]
    fn get_advertise_settings_builder(env: JNIEnv) {
        let rc_env = Rc::new(env);
        AdvertiseSettingsBuilder::new(rc_env.clone());
    }

    #[android_test]
    fn advertise_settings_mode(env: JNIEnv) {
        let rc_env = Rc::new(env);

        let mode = Mode::Balanced;
        let settings = AdvertiseSettingsBuilder::new(rc_env.clone()).set_mode(mode).build();
        assert_eq!(settings.get_mode(), mode);

        let mode = Mode::LowPower;
        let settings = AdvertiseSettingsBuilder::new(rc_env.clone()).set_mode(mode).build();
        assert_eq!(settings.get_mode(), mode);
    }

    #[android_test]
    fn advertise_settings_connectable(env: JNIEnv) {
        let rc_env = Rc::new(env);

        let flag = false;
        let settings = AdvertiseSettingsBuilder::new(rc_env.clone()).set_connectable(flag).build();
        assert_eq!(settings.is_connectable(), flag);

        let flag = true;
        let settings = AdvertiseSettingsBuilder::new(rc_env.clone()).set_connectable(flag).build();
        assert_eq!(settings.is_connectable(), flag);
    }

    #[android_test]
    fn advertise_settings_timeout(env: JNIEnv) {
        use std::time::Duration;

        let rc_env = Rc::new(env);

        let timeout = Duration::from_secs(10);
        let settings = AdvertiseSettingsBuilder::new(rc_env.clone()).set_timeout(timeout).build();
        assert_eq!(settings.get_timeout(), timeout);

        let timeout = Duration::from_secs(25);
        let settings = AdvertiseSettingsBuilder::new(rc_env.clone()).set_timeout(timeout).build();
        assert_eq!(settings.get_timeout(), timeout);
    }

    #[android_test]
    fn advertise_settings_tx_power_level(env: JNIEnv) {
        let rc_env = Rc::new(env);

        // Default connectable flag
        AdvertiseSettingsBuilder::new(rc_env.clone()).build().get_timeout();

        let power_level = TxPowerLevel::Medium;
        let settings = AdvertiseSettingsBuilder::new(rc_env.clone()).set_tx_power_level(power_level).build();
        assert_eq!(settings.get_tx_power_level(), power_level);

        let power_level = TxPowerLevel::Low;
        let settings = AdvertiseSettingsBuilder::new(rc_env.clone()).set_tx_power_level(power_level).build();
        assert_eq!(settings.get_tx_power_level(), power_level);
    }

    #[android_test]
    fn get_advertise_data_builder(env: JNIEnv) {
        let rc_env = Rc::new(env);
        AdvertisingDataBuilder::new(rc_env.clone());
    }

    #[android_test]
    fn advertise_data_include_name(env: JNIEnv) {
        let rc_env = Rc::new(env);

        let data = AdvertisingDataBuilder::new(rc_env.clone()).include_device_name(true).build();
        assert!(data.is_device_name_included());
    }

    #[android_test]
    fn advertise_data_add_service_data(env: JNIEnv) {
        use gap::advertise::service_data;

        let rc_env = Rc::new(env);

        let raw_data = [5u32,4,3,2,1];

        let service_data = service_data::new_128(0x123456789, &raw_data)
            .expect("Couldn't create service data");

        let data = AdvertisingDataBuilder::new(rc_env.clone()).add_service_data(service_data).build();

        assert_eq!(
            data.get_service_data()
                .first().unwrap()
                .get_data::<[u32;5]>().expect("Couldn't deserialize data"),
            raw_data
        );
    }

    #[android_test]
    fn advertise_data_add_services(env: JNIEnv) {

        let rc_env = Rc::new(env);

        let services: Services<u16> = [
            0xd56u16,
            0xa30e4d4,
            0x29,
        ].iter()
        .cloned()
        .collect();

        let data = AdvertisingDataBuilder::new(rc_env.clone())
            .add_service_uuid(services.clone().into())
            .build();

        let to_cmp = data.get_service_uuids().into_iter().zip(services);

        for each in to_cmp {
            assert_eq!( each.0, each.1 as u128 );
        }
    }

    #[android_test]
    fn create_advertise_callback(env: JNIEnv ) {
        use android::ClassLoader;

        let rc_env = Rc::new(env);

        AdvertiseCallbackBuilder::new()
            .build( rc_env.clone(), ClassLoader::system( rc_env.as_ref() ));
    }

    #[android_test]
    fn advertise_callback_fail_callback(env:JNIEnv) {
        use android::ClassLoader;
        use jni::objects::{JObject,JValue};
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        let flag = Arc::new(AtomicBool::new(false));

        let flag_clone = flag.clone();

        let rc_env = Rc::new(env);

        let callback = AdvertiseCallbackBuilder::new()
            .set_on_start_failure_callback( move |_| flag_clone.store( true, Ordering::Relaxed ) )
            .build( rc_env.clone(), ClassLoader::system( rc_env.as_ref() ));

        let reflect_method_name = callback.jenv.new_string("onStartFailure").unwrap();

        let method_parameter_type = callback.jenv.new_object_array(
            1,
            "java/lang/Class",
            get_jvalue! {
                Object,
                callback.jenv.get_static_field(
                    "java/lang/Integer",
                    "TYPE",
                    "Ljava/lang/Class;"
                )
                .expect("Couldn't get field TYPE of Integer")
            }
        )
        .expect("Couldn't create class array");

        let on_start_failure_method = get_jvalue! {
            Object,
            callback.jenv.call_method(
                callback.class.into(),
                "getMethod",
                "(Ljava/lang/String;[Ljava/lang/Class;)Ljava/lang/reflect/Method;",
                &[  JValue::Object(reflect_method_name.into()),
                    JValue::Object(JObject::from(method_parameter_type))
                ]
            )
            .expect("Couldn't create reflect method")
        };

        let on_start_failure_arg = callback.jenv.new_object_array(
            1,
            "java/lang/Object",
            callback.jenv.new_object(
                "java/lang/Integer",
                "(I)V",
                &[JValue::Int(32)] // number doesn't matter
            )
            .expect("Couldn't create Integer object")
        )
        .expect("Couldn't create onStartFailure arg Array");

        callback.jenv.call_method(
            on_start_failure_method,
            "invoke",
            "(Ljava/lang/Object;[Ljava/lang/Object;)Ljava/lang/Object;",
            &[
                JValue::Object(callback.object.into()),
                JValue::Object(on_start_failure_arg.into())
            ]
        )
        .expect("Invoke failed");

        assert!( flag.load(Ordering::Relaxed) );
    }

    #[android_test]
    fn advertise_callback_success_callback<'r>( env: JNIEnv ) {
        use android::ClassLoader;
        use jni::objects::{JObject,JValue};
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        let flag = Arc::new(AtomicBool::new(false));

        let flag_clone = flag.clone();

        let rc_env = Rc::new(env);

        let callback = AdvertiseCallbackBuilder::new()
            .set_on_start_success_callback( move |_| flag_clone.store( true, Ordering::Relaxed ) )
            .build( rc_env.clone(), ClassLoader::system( rc_env.as_ref() ));

        let reflect_method_name = callback.jenv.new_string("onStartSuccess").unwrap();

        let method_parameter_type = callback.jenv.new_object_array(
            1,
            "java/lang/Class",
            rc_env.find_class(bluetooth_le_class!("AdvertiseSettings")).expect("Class not found").into(),
        )
        .expect("Couldn't create class array");

        let on_start_failure_method = get_jvalue! {
            Object,
            callback.jenv.call_method(
                callback.class.into(),
                "getMethod",
                "(Ljava/lang/String;[Ljava/lang/Class;)Ljava/lang/reflect/Method;",
                &[  JValue::Object(reflect_method_name.into()),
                    JValue::Object(JObject::from(method_parameter_type))
                ]
            )
            .expect("Couldn't create reflect method")
        };

        let on_start_success_arg = callback.jenv.new_object_array(
            1,
            "java/lang/Object",
            JObject::from(AdvertiseSettingsBuilder::new(rc_env.clone()).build().object)
        )
        .expect("Couldn't create onStartFailure arg Array");

        callback.jenv.call_method(
            on_start_failure_method,
            "invoke",
            "(Ljava/lang/Object;[Ljava/lang/Object;)Ljava/lang/Object;",
            &[
                JValue::Object(callback.object.into()),
                JValue::Object(on_start_success_arg.into())
            ]
        )
        .expect("Invoke failed");

        assert!( flag.load(Ordering::Relaxed) );
    }

    #[android_test(permissions=( "BLUETOOTH", "BLUETOOTH_ADMIN", "ACCESS_COARSE_LOCATION", ))]
    fn bluetooth_le_advertiser( env: JNIEnv ) {
        use android::BluetoothManager;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::thread;
        use std::time::Duration;

        let rc_env = Rc::new(env);

        let new_callback = || {
            let success_callback_flag = Arc::new(AtomicBool::default());
            let failure_callback_flag = Arc::new(AtomicBool::default());

            let success_callback_flag_clone = success_callback_flag.clone();
            let failure_callback_flag_clone = failure_callback_flag.clone();

            AdvertiseCallbackBuilder::new()
                .set_on_start_failure_callback(
                    move |_| failure_callback_flag_clone.store( true, Ordering::Relaxed ) )
                .set_on_start_success_callback(
                    move |_| success_callback_flag_clone.store( true, Ordering::Relaxed ) )
                .build( rc_env.clone(), ::android::ClassLoader::system(rc_env.as_ref()) )
        };

        let context = ::android::tests::get_target_context(rc_env.as_ref());
        let manager = BluetoothManager::new(rc_env.clone(), context);
        let adapter = manager.get_adapter();

        assert!( adapter.enable() );

        // wait for the adapter to be enabled
        while !adapter.is_enabled() { thread::sleep(Duration::from_millis(100)) }

        let advertiser = adapter.get_bluetooth_le_advertisier();

        let settings = AdvertiseSettingsBuilder::new(rc_env.clone())
            // The advertising timeout is disabled, but this test will turn off advertising
            .set_timeout(Duration::from_secs(0))
            // No need for high power (or any for that matter :P )
            .set_tx_power_level(TxPowerLevel::UltraLow)
            // No connecting shenanagins
            .set_connectable(false)
            .build();

        let data = AdvertisingDataBuilder::new(rc_env.clone()).build();

        let start_callback = new_callback();
        let stop_callback = new_callback();

        assert!( ! advertiser.object.is_null(), "Check if bluetooth is enabled" );
        assert!( ! settings.object.is_null() );
        assert!( ! data.object.is_null() );
        assert!( ! start_callback.object.is_null() );
        assert!( ! stop_callback.object.is_null() );

        advertiser.start_advertising(settings, data, start_callback);

        thread::sleep(Duration::from_millis(500));

        advertiser.stop_advertising(stop_callback);
    }
}
