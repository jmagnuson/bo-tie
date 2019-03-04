 use std::rc::Rc;
use std::time::Duration;

pub struct BluetoothLeScanner<'a> {
    jenv: Rc<jni::JNIEnv<'a>>,
    object: jni::sys::jobject,
}

impl<'a> BluetoothLeScanner<'a> {
    pub(crate) fn new(jenv: Rc<jni::JNIEnv<'a>>, object: jni::objects::JObject<'a> ) -> Self {
        BluetoothLeScanner {
            jenv: jenv.clone(),
            object: object.into_inner(),
        }
    }
}

/// This contains things related to the android class
/// [ScanSettings] (https://developer.android.com/reference/android/bluetooth/le/ScanSettings)
pub mod scan_settings {

    macro_rules! make {
        ( $( #[$mets:meta] )* pub enum $enum_name:ident {
            $( $( #[$sub_mets:meta] )* $enum:ident => $val:expr, )*
        }) => {
            $( #[$mets] )*
            pub enum $enum_name {
                $( $( #[$sub_mets] )* $enum ),*
            }

            impl<'a> $enum_name {
                pub(super) fn get_jval(&self, jenv: ::std::rc::Rc<jni::JNIEnv<'a>> )
                    -> ::jni::sys::jint
                {
                    match *self {
                        $( $enum_name::$enum => $val ),*
                    }
                }

                #[allow(dead_code)]
                pub(super) fn from_jint<'b>(
                    jenv: ::std::rc::Rc<::jni::JNIEnv>,
                    val: ::jni::sys::jint
                ) -> ::std::result::Result<Self,String>
                 {
                    match val {
                        $( $val => Ok($enum_name::$enum), )*
                        _ => Err(format!(concat!("No enum in '", stringify!($enum_name), "' associated with value '{}'"), val))
                    }
                }
            }
        };
    }

    make! {
        #[derive(Clone, Copy, Debug, PartialEq)]
        pub enum CallbackType {
            /// CALLBACK_TYPE_ALL_MATCHES
            AllMatches => 1,
            /// CALLBACK_TYPE_FIRST_MATCH
            FirstMatch => 2,
            /// CALLBACK_TYPE_MATCH_LOST
            MatchLost  => 3,
        }
    }

    make!{
        #[derive(Clone, Copy, Debug, PartialEq)]
        pub enum MatchMode {
            /// MATCH_MODE_AGGRESSIVE
            Aggressive => 1,
            /// MATCH_MODE_STICKY
            Sticky     => 2,
        }
    }

    make! {
        #[derive(Clone, Copy, Debug, PartialEq)]
        pub enum AdvertismentMatchNumber {
            /// MATCH_NUM_ONE_ADVERTISEMENT
            One => 1,
            /// MATCH_NUM_FEW_ADVERTISEMENT
            Few => 2,
            /// MATCH_NUM_MAX_ADVERTISEMENT
            Max => 3,
        }
    }

    make! {
        #[derive(Clone, Copy, Debug, PartialEq)]
        pub enum PhySupport {
            /// PHY_LE_ALL_SUPPORTED
            AllSupported => 0xFF,
        }
    }

    make! {
        #[derive(Clone, Copy, Debug, PartialEq)]
        pub enum ScanMode {
            /// SCAN_MODE_BALANCED
            Balanced      => 1,
            /// SCAN_MODE_LOW_LATENCY
            LowLatency    => 2,
            /// SCAN_MODE_LOW_POWER
            LowPower      => 0,
            /// SCAN_MODE_OPPORTUNISTIC
            Opportunistic => -1,
        }
    }
}

struct ScanSettings<'a> {
    jenv: Rc<jni::JNIEnv<'a>>,
    object: jni::sys::jobject,
    pub callback_type: scan_settings::CallbackType,
    pub only_legacy: bool,
    pub phy: scan_settings::PhySupport,
    pub report_delay: Duration,
    pub scan_mode: scan_settings::ScanMode,
}

#[derive(Clone)]
struct ScanSettingsBuilder<'a> {
    jenv: Rc<jni::JNIEnv<'a>>,
    object: jni::sys::jobject,
}

macro_rules! scan_settins_builder_java_name{ () => {"ScanSettings$Builder"} }

impl<'a> ScanSettingsBuilder<'a> {
    pub fn new(jenv: Rc<jni::JNIEnv<'a>> ) -> Self {

        let object = jenv.new_object(
            bluetooth_le_class!(scan_settins_builder_java_name!()),
            "()V",
            &[])
        .unwrap();

        ScanSettingsBuilder {
            jenv: jenv.clone(),
            object: object.into_inner(),
        }
    }

    pub fn set_callback_type( self, cb_type: scan_settings::CallbackType ) -> Self {
        let object = get_jvalue!{
            Object,
            self.jenv.call_method(
                jni::objects::JObject::from(self.object),
                "setCallbackType",
                format!("(I)L{};", bluetooth_le_class!(scan_settins_builder_java_name!())),
                &[cb_type.get_jval(self.jenv.clone()).into()])
            .unwrap()
        };

        let mut ret = self.clone();

        ret.object = object.into_inner();

        ret
    }

    pub fn set_only_legacy(self, only_legacy: bool) -> Self {
        let object = get_jvalue!{
            Object,
            self.jenv.call_method(
                jni::objects::JObject::from(self.object),
                "setLegacy",
                format!("(Z)L{};", bluetooth_le_class!(scan_settins_builder_java_name!())),
                &[only_legacy.clone().into()])
            .unwrap()
        };


        let mut ret = self.clone();

        ret.object = object.into_inner();

        ret
    }

    pub fn set_match_mode(self, match_mode: scan_settings::MatchMode ) -> Self {
        let object = get_jvalue!{
            Object,
            self.jenv.call_method(
                jni::objects::JObject::from(self.object),
                "setMatchMode",
                format!("(I)L{};", bluetooth_le_class!(scan_settins_builder_java_name!())),
                &[match_mode.get_jval(self.jenv.clone()).into()])
            .unwrap()
        };


        let mut ret = self.clone();

        ret.object = object.into_inner();

        ret
    }

    pub fn set_num_of_matches(self, num_of_matches: scan_settings::AdvertismentMatchNumber ) -> Self {
        let object = get_jvalue!{
            Object,
            self.jenv.call_method(
                jni::objects::JObject::from(self.object),
                "setNumOfMatches",
                format!("(I)L{};", bluetooth_le_class!(scan_settins_builder_java_name!())),
                &[num_of_matches.get_jval(self.jenv.clone()).into()])
            .unwrap()
        };

        let mut ret = self.clone();

        ret.object = object.into_inner();

        ret
    }

    /// This just returns self since there is only one enum in variant `scan_settings::PhySupport`
    pub fn set_phy(self, _phy: scan_settings::PhySupport) -> Self {
        self
    }

    /// The `delay` imput has a minimum resolution of 1 milliseconds.
    pub fn set_report_delay(self, delay: Duration ) -> Self {
        let ms = (delay.as_secs() as u32 * 1000 + delay.subsec_millis()) as jni::sys::jlong;

        let object = get_jvalue!{
            Object,
            self.jenv.call_method(
                jni::objects::JObject::from(self.object),
                "setReportDelay",
                format!("(J)L{};", bluetooth_le_class!(scan_settins_builder_java_name!())),
                &[ms.into()])
            .unwrap()
        };

        let mut ret = self.clone();

        ret.object = object.into_inner();

        ret
    }

    pub fn set_scan_mode(self, mode: scan_settings::ScanMode ) -> Self {
        let object = get_jvalue!{
            Object,
            self.jenv.call_method(
                jni::objects::JObject::from(self.object),
                "setScanMode",
                format!("(I)L{};", bluetooth_le_class!(scan_settins_builder_java_name!())),
                &[mode.get_jval(self.jenv.clone()).into()])
            .unwrap()
        };

        let mut ret = self.clone();

        ret.object = object.into_inner();

        ret
    }

    pub fn build(self) -> ScanSettings<'a> {
        let object = get_jvalue!{
            Object,
            self.jenv.call_method(
                jni::objects::JObject::from(self.object),
                "build",
                format!("()L{};", bluetooth_le_class!("ScanSettings")),
                &[])
            .unwrap()
        };

        ScanSettings {
            jenv: self.jenv.clone(),
            callback_type: self.get_callback_type(object),
            only_legacy: self.get_legacy(object),
            phy: scan_settings::PhySupport::AllSupported,
            report_delay: self.get_report_delay(object),
            scan_mode: self.get_scan_mode(object),
            object: object.into_inner(),
        }
    }

    fn get_callback_type(&self, settings: jni::objects::JObject<'a>) -> scan_settings::CallbackType {
        scan_settings::CallbackType::from_jint(
            self.jenv.clone(),
            get_jvalue!{
                Int,
                self.jenv.call_method(
                    jni::objects::JObject::from(settings),
                    "getCallbackType",
                    "()I",
                    &[])
                .unwrap()
            }
        ).unwrap()
    }

    fn get_legacy(&self, settings: jni::objects::JObject<'a>) -> bool {
        0 != get_jvalue!{
            Bool,
            self.jenv.call_method(
                jni::objects::JObject::from(settings),
                "getLegacy",
                "()Z",
                &[])
            .unwrap()
        }
    }

    fn get_report_delay(&self, settings: jni::objects::JObject<'a>) -> Duration {
        let delay = get_jvalue!{
            Long,
            self.jenv.call_method(
                jni::objects::JObject::from(settings),
                "getReportDelayMillis",
                "()J",
                &[])
            .unwrap()
        };

        Duration::from_millis(delay as u64)
    }

    fn get_scan_mode(&self, settings: jni::objects::JObject<'a>) -> scan_settings::ScanMode {
        scan_settings::ScanMode::from_jint(
            self.jenv.clone(),
            get_jvalue!{
                Int,
                self.jenv.call_method(
                    jni::objects::JObject::from(settings),
                    "getScanMode",
                    "()I",
                    &[])
                .unwrap()
            }
        ).unwrap()
    }
}

/// A result from scanning
///
/// To get the advertising data, call
/// [get_scan_record](./index.html#get_scan_record)
pub struct ScanResult<'a> {
    jenv: Rc<jni::JNIEnv<'a>>,
    object: jni::sys::jobject,
}

impl<'a> ScanResult<'a> {

    fn new( jenv: Rc<jni::JNIEnv<'a>>, object: jni::objects::JObject ) -> Self {
        Self {
            jenv: jenv,
            object: object.into_inner()
        }
    }

    pub fn get_device(&self) -> ::android::BluetoothDevice {
        let object = get_jvalue! {
            Object,
            self.jenv.call_method(
                jni::objects::JObject::from(self.object),
                "getDevice",
                format!("()L{};", bluetooth_class!("BluetoothDevice")),
                &[])
            .unwrap()
        };

        ::android::BluetoothDevice::new(self.jenv.clone(), object)
    }

    // Signal strength of advertiser in dB
    pub fn get_rssi(&self) -> i32 {
        get_jvalue! (
            Int,
            self.jenv.call_method(
                jni::objects::JObject::from(self.object),
                "getRssi",
                "()I",
                &[])
            .unwrap()
        ) as i32
    }

    pub fn get_scan_record(&self) -> ScanRecord {
        ScanRecord::new( self.jenv.clone(), get_jvalue! {
            Object,
            self.jenv.call_method(
                jni::objects::JObject::from(self.object),
                "getScanRecord",
                format!("()L{};", bluetooth_le_class!("ScanRecord")),
                &[])
            .unwrap()
        })
    }

    pub fn is_connectable(&self) -> bool {
        self.jenv.call_method(
            jni::objects::JObject::from(self.object),
            "isConnectable",
            "()Z",
            &[])
        .unwrap()
        .z()
        .unwrap()
    }
}

/// A Scan Record
///
/// A scan record can be converted into an interator to iterate over the returned advertise data
pub struct ScanRecord {
    bytes: Vec<u8>
}

impl ScanRecord
{
    /// Create a new ScanRecord from its java object
    fn new<'a>(jenv: Rc<jni::JNIEnv<'a>>, java_object: jni::objects::JObject ) -> Self {
        let java_bytes = get_jvalue! {
            Object,
            jenv.call_method(
                java_object,
                "getBytes",
                "()[B",
                &[])
            .unwrap()
        };

        Self {
            bytes: jenv.convert_byte_array( java_bytes.into_inner() ).unwrap()
        }
    }

    pub fn iter<'a, T>(&'a self) -> ScanRecordIter<'a, T> {
        ScanRecordIter {
            bytes: &self.bytes,
            pd: std::marker::PhantomData,
        }
    }
}

pub struct ScanRecordIter<'a, T> {
    bytes: &'a [u8],
    pd: std::marker::PhantomData<T>
}

/// An iterator over a received scan record
impl<'a, T> Iterator for ScanRecordIter<'a, T>
where T: ::gap::advertise::TryFromRaw
{
    type Item = Result< T, ::gap::advertise::Error >;

    fn next(&mut self) -> Option<Self::Item> {
        use ::gap::advertise::Error;

        self.bytes.split_first().and_then(|(first, rest)| {
            let len = *first as usize;

            if rest.len() >= len {

                self.bytes = &self.bytes[len..];

                Some(T::try_from_raw(&self.bytes[..len]))

            } else {
                // This should happen only if a length (any of them) value is bad

                self.bytes = &[];

                Some(Err(Error::IncorrectLength))
            }
        })
    }
}

pub enum ScanError {
    AlreadyStarted,
    ApplicationRegistrationFailed,
    FeatureUnsupported,
    InternalError,
    Unknown(jni::sys::jint),
}

impl ScanError {
    fn from_raw(code: jni::sys::jint) -> Self {
        match code {
            1 => ScanError::AlreadyStarted,
            2 => ScanError::ApplicationRegistrationFailed,
            3 => ScanError::InternalError,
            4 => ScanError::FeatureUnsupported,
            _ => ScanError::Unknown(code)
        }
    }
}

mod callback {
    use std::sync::{Arc,Mutex};
    use super::scan_settings::CallbackType;
    use super::{ScanError,ScanResult};

    pub struct ScanCallbackFunctions<'s, BR, F, R>
    where BR: Fn( Box<[ScanResult<'s>]> ) + Send + ?Sized,
          F:  Fn( ScanError ) + Send + ?Sized,
          R:  Fn( CallbackType, ScanResult<'s> ) + Send + ?Sized
    {
        pub batch_results: Option<Box<BR>>,
        pub scan_failed: Option<Box<F>>,
        pub scan_result: Option<Box<R>>,
        pub pd: std::marker::PhantomData<&'s ()>,
    }

    pub type HashMap<'s> = std::collections::HashMap< jni::sys::jint, ScanCallbackFunctions<
        's,
        dyn Fn( Box<[ScanResult<'s>]> ) + Send,
        dyn Fn( ScanError ) + Send,
        dyn Fn( CallbackType, ScanResult<'s> ) + Send
    >>;

    pub type Map<'s> = Arc<Mutex<HashMap<'s>>>;
}

pub struct ScanCallback<'a> {
    jenv: Rc<jni::JNIEnv<'a>>,
    object: jni::sys::jobject,

    #[cfg(feature = "android_test")]
    class: &'a jni::objects::GlobalRef,
}

impl<'a> ScanCallback<'a> {

    #[inline]
    fn get_map_lock() -> Box< std::ops::DerefMut< Target=callback::HashMap<'static> > + 'static >
    {
        use std::collections::HashMap;
        use std::sync::{Arc,Mutex};
        use self::callback::Map;

        lazy_static! {
            static ref MAP: Map<'static> = Arc::new(Mutex::new(HashMap::new()));
        }

        Box::new(MAP.lock().expect("ScanCallback MAP poisoned!"))
    }

    fn new(
        jenv: Rc<jni::JNIEnv<'a>>,
        classloader: ::android::ClassLoader,
        builder: ScanCallbackBuilder<'static>,
    ) -> Self {
        use android::load_classbytes::{DexBytesLoader, NativeMethod};
        use std::ffi::CString;
        use std::sync::Once;
        use jni::objects::GlobalRef;

        static CLASS_NAME: &'static str = "botie/ScanCallback";
        static mut CLASS: Option<GlobalRef> = None;
        static ONCE: Once = Once::new();

        ONCE.call_once(|| {
            let class = DexBytesLoader::new(
                    &::android::classbytes::SCAN_CALLBACK_CLASS_BYTES,
                    CLASS_NAME)
                .add_native_method (
                    unsafe { NativeMethod::new(
                        CString::new("onBatchScanResults").unwrap(),
                        CString::new("(Ljava/util/List;)V").unwrap(),
                        Self::on_batch_results as *const std::ffi::c_void,
                    ) })
                .add_native_method (
                    unsafe { NativeMethod::new(
                        CString::new("onScanFailed").unwrap(),
                        CString::new("(I)V").unwrap(),
                        Self::on_failure as *const std::ffi::c_void,
                    ) })
                .add_native_method (
                    unsafe { NativeMethod::new(
                        CString::new("onScanResult").unwrap(),
                        CString::new(format!("(IL{};)V", bluetooth_le_class!("ScanResult"))).unwrap(),
                        Self::on_result as *const std::ffi::c_void,
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

    extern "system" fn on_batch_results (
        jenv: jni::JNIEnv<'static>,
        object: jni::objects::JObject,
        scan_result_list: jni::objects::JObject,
    ) {
        if let Some(ref callback) = Self::get_map_lock()
            .get( &Self::get_hash_code( &jenv, object ) )
            .expect("Cannot get object from ScanCallback's static map")
            .batch_results
        {
            let rc_env = Rc::new(jenv);

            let array = get_jvalue! {
                Object,
                rc_env.call_method(
                    scan_result_list,
                    "toArray",
                    "()[Ljava/lang/Object;",
                    &[])
                .unwrap()
            }
            .into_inner();

            let array_len = rc_env.get_array_length(array).unwrap();

            let scan_results =  {0..array_len}.map( | index | {
                let element = rc_env.get_object_array_element(array, index).unwrap();

                ScanResult::new( rc_env.clone(), element)
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();

            callback(scan_results);
        }
    }

    extern "system" fn on_failure (
        jenv: jni::JNIEnv<'static>,
        object: jni::objects::JObject,
        err_code: jni::sys::jint,
    ) {
        if let Some(ref callback) = Self::get_map_lock()
            .get( &Self::get_hash_code( &jenv, object ) )
            .expect("Cannot get object from ScanCallback's static map")
            .scan_failed
        {
             callback( ScanError::from_raw(err_code) )
        }
    }

    extern "system" fn on_result (
        jenv: jni::JNIEnv<'static>,
        object: jni::objects::JObject,
        callback_type: jni::sys::jint,
        scan_result: jni::objects::JObject,
    ) {
        let rc_env = Rc::new(jenv);

        use self::scan_settings::CallbackType;

        if let Some(ref callback) = Self::get_map_lock()
            .get( &Self::get_hash_code( rc_env.as_ref(), object ) )
            .expect("Cannot get object from ScanCallback's static map")
            .scan_result
        {
            callback(
                CallbackType::from_jint(rc_env.clone(), callback_type)
                    .expect("Unknown scan callback type"),
                ScanResult::new( rc_env, scan_result)
            )
        }
    }

    extern "system" fn clean ( jenv: jni::JNIEnv, object: jni::objects::JObject) {
        Self::get_map_lock()
            .remove( &Self::get_hash_code( &jenv, object ) )
            .expect("Cannot get object from ScanCallback's static map");
    }
}

pub struct ScanCallbackBuilder<'s> {
    callbacks: callback::ScanCallbackFunctions<
        's,
        dyn Fn( Box<[ScanResult<'s>]> ) + Send,
        dyn Fn( ScanError ) + Send,
        dyn Fn( self::scan_settings::CallbackType, ScanResult<'s> ) + Send
    >,
}

impl<'s> ScanCallbackBuilder<'s> {
    fn new() -> Self {
        ScanCallbackBuilder {
            callbacks: callback::ScanCallbackFunctions {
                batch_results: None,
                scan_failed: None,
                scan_result: None,
                pd: std::marker::PhantomData,
            }
        }
    }

    pub fn set_on_batch_scan_results_callback<F>( self, callback: F ) -> Self
    where F: 'static + std::marker::Unsize< dyn Fn( Box<[ScanResult<'s>]> ) + Send > + Sized
    {
        Self {
            callbacks: callback::ScanCallbackFunctions {
                batch_results: Some(Box::new(callback) as Box<dyn Fn( Box<[ScanResult<'s>]> ) + Send> ),
                scan_failed: self.callbacks.scan_failed,
                scan_result: self.callbacks.scan_result,
                pd: std::marker::PhantomData,
            }
        }
    }

    pub fn set_on_scan_failed_callback<F>( self, callback: F ) -> Self
    where F: 'static + std::marker::Unsize< dyn Fn( ScanError ) + Send > + Sized
    {
        Self {
            callbacks: callback::ScanCallbackFunctions {
                batch_results: self.callbacks.batch_results,
                scan_failed: Some(Box::new(callback)),
                scan_result: self.callbacks.scan_result,
                pd: std::marker::PhantomData,
            }
        }
    }

    pub fn set_on_scan_result<F>( self, callback: F ) -> Self
    where F: 'static + std::marker::Unsize< dyn Fn( scan_settings::CallbackType, ScanResult<'s> ) + Send > + Sized
    {
        Self {
            callbacks: callback::ScanCallbackFunctions {
                batch_results: self.callbacks.batch_results,
                scan_failed: self.callbacks.scan_failed,
                scan_result: Some(Box::new(callback)),
                pd: std::marker::PhantomData,
            }
        }
    }

    pub fn build<'j>(self, jenv: Rc<jni::JNIEnv<'j>>, classloader: ::android::ClassLoader) -> ScanCallback<'j>
    where Self: 'static
    {
        ScanCallback::new(jenv, classloader, self)
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
    fn get_scan_settings_builder(jenv: JNIEnv) {
        let rc_jenv = Rc::new(jenv);

        ScanSettingsBuilder::new(rc_jenv);
    }

    #[android_test]
    fn scan_settings_set_only_legacy(jenv: JNIEnv) {
        let rc_jenv = Rc::new(jenv);

        let scan_settings = ScanSettingsBuilder::new(rc_jenv.clone()).set_only_legacy(true).build();

        assert!(scan_settings.only_legacy);
    }

    #[android_test]
    fn scan_settings_set_match_mode(jenv: JNIEnv) {
        use self::scan_settings::MatchMode;

        let rc_jenv = Rc::new(jenv);

        ScanSettingsBuilder::new(rc_jenv.clone()).set_match_mode(MatchMode::Aggressive).build();
        ScanSettingsBuilder::new(rc_jenv.clone()).set_match_mode(MatchMode::Sticky).build();
    }

    #[android_test]
    fn scan_settings_set_num_of_matches(jenv: JNIEnv) {
        use self::scan_settings::AdvertismentMatchNumber;

        let rc_jenv = Rc::new(jenv);

        ScanSettingsBuilder::new(rc_jenv.clone()).set_num_of_matches(AdvertismentMatchNumber::One).build();
        ScanSettingsBuilder::new(rc_jenv.clone()).set_num_of_matches(AdvertismentMatchNumber::Few).build();
        ScanSettingsBuilder::new(rc_jenv.clone()).set_num_of_matches(AdvertismentMatchNumber::Max).build();
    }

    #[android_test]
    fn scan_settings_set_phy(jenv: JNIEnv) {
        let rc_jenv = Rc::new(jenv);

        ScanSettingsBuilder::new(rc_jenv.clone()).set_phy(self::scan_settings::PhySupport::AllSupported).build();
    }

    #[android_test]
    fn scan_settings_set_report_delay(jenv: JNIEnv) {
        let rc_jenv = Rc::new(jenv);

        let dur_1 = Duration::from_secs(2);
        let dur_2 = Duration::from_millis(500);

        let settings_1 = ScanSettingsBuilder::new(rc_jenv.clone()).set_report_delay(dur_1).build();
        let settings_2 = ScanSettingsBuilder::new(rc_jenv.clone()).set_report_delay(dur_2).build();

        assert_eq!(settings_1.report_delay, dur_1);
        assert_eq!(settings_2.report_delay, dur_2);
    }

    #[android_test]
    fn scan_settings_set_scan_mode(jenv: JNIEnv) {
        use self::scan_settings::ScanMode;

        let rc_jenv = Rc::new(jenv);

        ScanSettingsBuilder::new(rc_jenv.clone()).set_scan_mode(ScanMode::Balanced).build();
        ScanSettingsBuilder::new(rc_jenv.clone()).set_scan_mode(ScanMode::LowLatency).build();
        ScanSettingsBuilder::new(rc_jenv.clone()).set_scan_mode(ScanMode::LowPower).build();
        ScanSettingsBuilder::new(rc_jenv.clone()).set_scan_mode(ScanMode::Opportunistic).build();
    }

    #[android_test]
    fn create_scan_callback(jenv: JNIEnv) {
        use android::ClassLoader;

        let rc_env = Rc::new(jenv);

        ScanCallbackBuilder::new().build(rc_env.clone(), ClassLoader::system(rc_env.as_ref()));
    }

    #[android_test]
    fn scan_on_batch_results_callback(jenv: JNIEnv) {
        use android::ClassLoader;
        use jni::objects::{JValue,JObject};
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool,Ordering};

        let flag = Arc::new(AtomicBool::new(false));
        let flag_clone = flag.clone();

        let rc_env = Rc::new(jenv);

        let callback = ScanCallbackBuilder::new()
            .set_on_batch_scan_results_callback(move |_| flag_clone.store(true, Ordering::Relaxed))
            .build(rc_env.clone(), ClassLoader::system(rc_env.as_ref()));

        let reflect_method_name = rc_env.new_string("onBatchScanResults").unwrap();

        let method_parameter_type = rc_env.new_object_array(
            1,
            "java/lang/Class",
            rc_env.find_class("java/util/List").expect("Class not found").into())
        .expect("Couldn't create class array");

        let on_start_failure_method = get_jvalue! {
            Object,
            rc_env.call_method(
                callback.class.into(),
                "getMethod",
                "(Ljava/lang/String;[Ljava/lang/Class;)Ljava/lang/reflect/Method;",
                &[  JValue::Object(reflect_method_name.into()),
                    JValue::Object(JObject::from(method_parameter_type))
                ]
            )
            .expect("Couldn't create reflect method")
        };

        let on_batch_results_input = get_jvalue!{
            Object,
            rc_env.call_static_method(
                "java/util/Collections",
                "emptyList",
                "()Ljava/util/List;",
                &[])
            .unwrap()
        };

        let on_batch_results_arg = rc_env.new_object_array(
            1,
            "java/lang/Object",
            on_batch_results_input
        )
        .expect("Couldn't create onBatchScanResults arg Array");

        rc_env.call_method(
            on_start_failure_method,
            "invoke",
            "(Ljava/lang/Object;[Ljava/lang/Object;)Ljava/lang/Object;",
            &[
                JValue::Object(callback.object.into()),
                JValue::Object(on_batch_results_arg.into())
            ]
        )
        .expect("Invoke failed");


        assert!( flag.load(Ordering::Relaxed) );
    }

    #[android_test]
    fn scan_scan_failure_callback(jenv: JNIEnv) {
        use android::ClassLoader;
        use jni::objects::{JValue,JObject};
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool,Ordering};

        let flag = Arc::new(AtomicBool::new(false));
        let flag_clone = flag.clone();

        let rc_env = Rc::new(jenv);

        let callback = ScanCallbackBuilder::new()
            .set_on_scan_failed_callback( move |_| flag_clone.store(true, Ordering::Relaxed) )
            .build(rc_env.clone(), ClassLoader::system(rc_env.as_ref()));

        let reflect_method_name = rc_env.new_string("onScanFailed").unwrap();

        let method_parameter_type = rc_env.new_object_array(
            1,
            "java/lang/Class",
            get_jvalue! {
                Object,
                rc_env.get_static_field(
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
            rc_env.call_method(
                callback.class.into(),
                "getMethod",
                "(Ljava/lang/String;[Ljava/lang/Class;)Ljava/lang/reflect/Method;",
                &[  JValue::Object(reflect_method_name.into()),
                    JValue::Object(JObject::from(method_parameter_type))
                ]
            )
            .expect("Couldn't create reflect method")
        };

        let on_start_failure_arg = rc_env.new_object_array(
            1,
            "java/lang/Object",
            rc_env.new_object(
                "java/lang/Integer",
                "(I)V",
                &[JValue::Int(1)] // the value for 'already started'
            )
            .expect("Couldn't create Integer object")
        )
        .expect("Couldn't create onStartFailure arg Array");

        rc_env.call_method(
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

    /// # Note
    /// There is no way to construct the type android.bluetooth.le.ScanResult. This means that this
    /// test allocates a ScanResult object to call the java callback method `onScanResult` with.
    /// Thus the callback set to the scan result must ignore the 2nd input as it is unsafe to use.
    #[android_test]
    fn scan_on_scan_result_callback(jenv: JNIEnv) {
        use android::ClassLoader;
        use jni::objects::{JValue,JObject};
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool,Ordering};

        let flag = Arc::new(AtomicBool::new(false));
        let flag_clone = flag.clone();

        let rc_env = Rc::new(jenv);

        let callback = ScanCallbackBuilder::new()
            .set_on_scan_result(move |_,_| flag_clone.store(true, Ordering::Relaxed))
            .build(rc_env.clone(), ClassLoader::system(rc_env.as_ref()));

        let reflect_method_name = rc_env.new_string("onScanResult").unwrap();

        let method_parameters_types = rc_env.new_object_array(
            2,
            "java/lang/Class",
            rc_env.find_class("java/lang/Object").expect("Class not found").into() )
        .expect("Couldn't create class array");

        rc_env.set_object_array_element(
            method_parameters_types,
            0,
            get_jvalue! {
                Object,
                rc_env.get_static_field(
                    "java/lang/Integer",
                    "TYPE",
                    "Ljava/lang/Class;"
                )
                .expect("Couldn't get field TYPE of Integer")
            }
        )
        .expect("Integer class not added");

        rc_env.set_object_array_element(
            method_parameters_types,
            1,
            rc_env.find_class(bluetooth_le_class!("ScanResult")).expect("Class not found").into()
        )
        .expect("ScanResult class not added");

        let on_start_failure_method = get_jvalue! {
            Object,
            rc_env.call_method(
                callback.class.into(),
                "getMethod",
                "(Ljava/lang/String;[Ljava/lang/Class;)Ljava/lang/reflect/Method;",
                &[  JValue::Object(reflect_method_name.into()),
                    JValue::Object(JObject::from(method_parameters_types))
                ]
            )
            .expect("Couldn't create reflect method")
        };

        let on_scan_result_args = rc_env.new_object_array(
            2,
            "java/lang/Object",
            rc_env.new_object("java/lang/Object", "()V", &[]).unwrap()
        )
        .expect("Couldn't create onScanResult arg Array");

        rc_env.set_object_array_element(
            on_scan_result_args,
            0,
            rc_env.new_object(
                "java/lang/Integer",
                "(I)V",
                &[JValue::Int(1)] // the value for 'callback type all matches'
            )
            .expect("Couldn't create Integer object")
        )
        .expect("Integer not added to arg array");

        rc_env.set_object_array_element(
            on_scan_result_args,
            1,
            // this is why the closure assigned as the callback cannot use any inputs in this test
            rc_env.alloc_object(bluetooth_le_class!("ScanResult")).unwrap()
        )
        .expect("Allocated ScanResult not added to arg array");

        rc_env.call_method(
            on_start_failure_method,
            "invoke",
            "(Ljava/lang/Object;[Ljava/lang/Object;)Ljava/lang/Object;",
            &[
                JValue::Object(callback.object.into()),
                JValue::Object(on_scan_result_args.into())
            ]
        )
        .expect("Invoke failed");


        assert!( flag.load(Ordering::Relaxed) );
    }

    #[android_test]
    fn scan_result_get_device(jenv: JNIEnv) {
        let rc_env = Rc::new(jenv);

        let alloc_obj = rc_env.alloc_object(bluetooth_le_class!("ScanResult")).unwrap();

        let dummy_scan_result = ScanResult::new(rc_env.clone(), alloc_obj);

        // test that this doesn't panic
        dummy_scan_result.get_device();
    }

    #[android_test]
    fn scan_result_get_rssi(jenv: JNIEnv) {
        let rc_env = Rc::new(jenv);

        let alloc_obj = rc_env.alloc_object(bluetooth_le_class!("ScanResult")).unwrap();

        let dummy_scan_result = ScanResult::new(rc_env.clone(), alloc_obj);

        // test that this doesn't panic
        dummy_scan_result.get_rssi();
    }

    /// This test will never work because trying to create a scan record from an allocated object
    /// will never work
    #[android_test(ignore)]
    fn scan_result_get_scan_record(jenv: JNIEnv) {
        let rc_env = Rc::new(jenv);

        let alloc_obj = rc_env.alloc_object(bluetooth_le_class!("ScanResult")).unwrap();

        let dummy_scan_result = ScanResult::new(rc_env.clone(), alloc_obj);

        // test that this doesn't panic
        dummy_scan_result.get_scan_record();
    }

    #[android_test]
    fn scan_result_is_connectable(jenv: JNIEnv) {
        let rc_env = Rc::new(jenv);

        let alloc_obj = rc_env.alloc_object(bluetooth_le_class!("ScanResult")).unwrap();

        let dummy_scan_result = ScanResult::new(rc_env.clone(), alloc_obj);

        // test that this doesn't panic
        dummy_scan_result.is_connectable();
    }
}
