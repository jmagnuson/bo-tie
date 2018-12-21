use gap::advertise::service_class_uuid::Services;
use std::option::Option;
use std::rc::Rc;
use std::time::Duration;

#[derive(Clone)]
pub struct BluetoothLEAdvertiser<'a> {
    jenv: Rc<jni::JNIEnv<'a>>,
    object: jni::sys::jobject,
}

impl<'a> BluetoothLEAdvertiser<'a> {
    pub(super) fn new(jenv: Rc<jni::JNIEnv<'a>>, object: jni::objects::JObject<'a> ) -> Self {
        BluetoothLEAdvertiser {
            jenv: jenv,
            object: object.into_inner(),
        }
    }

    pub fn start_advertising( &self, settings: AdvertiseSettings, data: AdvertisingData, callback: AdvertiseCallback ) {
        self.jenv.call_method(
            jni::objects::JObject::from(self.object),
            "startAdvertising",
            format!("(L{};L{};L{};)",
                bluetooth_le_class!("AdvertiseSettings"),
                bluetooth_le_class!("AdvertiseData"),
                bluetooth_le_class!("AdvertiseCallback")
            ),
            &[
                jni::objects::JValue::Object(jni::objects::JObject::from(settings.object)),
                jni::objects::JValue::Object(jni::objects::JObject::from(data.object)),
                jni::objects::JValue::Object(jni::objects::JObject::from(callback.object)),
            ])
        .unwrap();
    }

    pub fn stop_advertising(&self, callback: AdvertiseCallback) {
        self.jenv.call_method(
            jni::objects::JObject::from(self.object),
            "stopAdvertising",
            format!("(L{};)", bluetooth_le_class!("AdvertiseCallback")),
            &[jni::objects::JValue::Object(jni::objects::JObject::from(callback.object))])
        .unwrap();
    }
}

/// Mode returned from AdvertiseSettings::get_mode
#[derive(Clone,Copy,Debug)]
pub enum Mode {
    Balanced,
    LowLatency,
    LowPower,
}

/// Tx Power Level returned from AdvertiseSettings::get_tx_power_level
#[derive(Clone,Copy,Debug)]
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
    mode: super::Mode,
    connectable: bool,
    tx_power_level: super::TxPowerLevel,
    timeout: Duration,
}

impl<'a> AdvertiseSettings<'a> {

    /// Get the mode
    ///
    /// On android this is either balanced, low latency, or low power
    pub fn get_mode(&self) -> super::Mode {
        self.mode
    }

    pub fn is_connectable(&self) -> bool {
        self.connectable
    }

    pub fn get_timeout(&self) -> Duration {
        self.timeout
    }

    pub fn get_tx_power_level(&self) -> super::TxPowerLevel {
        self.tx_power_level
    }
}

#[derive(Clone)]
pub struct AdvertiseSettingsBuilder<'a> {
    jenv: Rc<jni::JNIEnv<'a>>,
    object: jni::sys::jobject,
    mode: Option<super::Mode>,
    connectable: Option<bool>,
    timeout: Option<Duration>,
    tx_power_level: Option<super::TxPowerLevel>,
}

macro_rules! advertise_settings_builder_java_name{ () => { "AdvertiseSettings$Builder" }}

impl<'a> AdvertiseSettingsBuilder<'a> {

    pub fn new( jenv: Rc<jni::JNIEnv<'a>> ) -> Self {
        let object = jenv.new_object(
            bluetooth_le_class!(advertise_settings_builder_java_name!()),
            "()",
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

    fn get_const(&self, name: &str) -> jni::objects::JValue {
        self.jenv.get_static_field(
            bluetooth_le_class!(advertise_settings_builder_java_name!()),
            name,
            "I")
        .unwrap()
    }

    pub fn set_advertise_mode(self, mode: Mode ) -> Self {
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
    /// This will limit the advertising to this time. However, advertising may be less than the
    /// amount provided as the parameter depending on the (android) os/device.
    pub fn set_timeout_millis( self, time: Duration ) -> Self {
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

    pub fn set_tx_power_level( self, level: super::TxPowerLevel ) -> Self {
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

    fn get_jint_const<'b>(&self, name: &'static str) -> jni::sys::jint {
        get_jvalue! {
            Int,
            self.jenv.get_static_field(
                bluetooth_le_class!("AdvertiseSettings"),
                name,
                "I"
            )
            .unwrap()
        }
    }

    fn get_mode<'b>(&self, advertise_settings: jni::objects::JObject<'b>) -> super::Mode {
        match self.mode {
            Some(mode) => mode,
            None => {
                let mode = get_jvalue!{
                    Int,
                    self.jenv.call_method(
                        advertise_settings,
                        "getMode",
                        "()I",
                        &[])
                    .unwrap()
                };

                if mode == self.get_jint_const("ADVERTISE_MODE_BALANCED") {
                    Mode::Balanced
                }
                else if mode == self.get_jint_const("ADVERTISE_MODE_LOW_LATENCY") {
                    Mode::LowLatency
                }
                else if mode == self.get_jint_const("ADVERTISE_MODE_LOW_POWER") {
                    Mode::LowPower
                }
                else {
                    panic!("Unknown advertise mode returned from android bluetooth driver")
                }
            }
        }
    }

    fn get_connectable<'b>(&self, advertise_settings: jni::objects::JObject<'b> ) -> bool {
        match self.connectable {
            Some(connectable) => connectable,
            None => {
                let connectable = get_jvalue!{
                    Bool,
                    self.jenv.call_method(
                        advertise_settings,
                        "getConnectable",
                        "()Z",
                        &[])
                    .unwrap()
                };

                connectable != jni::sys::jboolean::default()
            }
        }
    }

    fn get_timeout<'b>(&self, advertise_settings: jni::objects::JObject<'b> ) -> Duration {
        match self.timeout {
            Some(duration) => duration,
            None => {
                let ms = get_jvalue!{
                    Int,
                    self.jenv.call_method(
                        advertise_settings,
                        "getTimeout",
                        "()I",
                        &[])
                    .unwrap()
                };

                Duration::from_millis(ms as u64)
            }
        }
    }

    fn get_tx_power_level<'b>(&self, advertise_settings: jni::objects::JObject<'b> ) -> super::TxPowerLevel
    {
        match self.tx_power_level {
            Some(level) => level,
            None => {
                let power_level = get_jvalue! {
                    Int,
                    self.jenv.call_method(
                        advertise_settings,
                        "getTxPowerLevel",
                        "()I",
                        &[])
                    .unwrap()
                };

                if power_level == self.get_jint_const("ADVERTISE_TX_POWER_HIGH") {
                    TxPowerLevel::High
                }
                else if power_level == self.get_jint_const("ADVERTISE_TX_POWER_LOW") {
                    TxPowerLevel::Low
                }
                else if power_level == self.get_jint_const("ADVERTISE_TX_POWER_MEDIUM") {
                    TxPowerLevel::Medium
                }
                else if power_level == self.get_jint_const("ADVERTISE_TX_POWER_ULTRA_LOW") {
                    TxPowerLevel::UltraLow
                }
                else {
                    panic!("Unknown advertise tx power level returned from android bluetooth driver")
                }
            }
        }
    }

    pub fn build(self) -> super::AdvertiseSettings<'a> {
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
            connectable: self.get_connectable(object),
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
}

#[derive(Clone)]
pub struct AdvertisingDataBuilder<'a> {
    jenv: Rc<jni::JNIEnv<'a>>,
    object: jni::sys::jobject,
    services: Services<u128>,
    include_device_name: bool,
    include_tx_power_level: bool,
}

macro_rules! advertise_data_builder_java_name{ () => { "AdvertiseSettings$Builder" }}

impl<'a> AdvertisingDataBuilder<'a> {
    pub fn new( jenv: Rc<jni::JNIEnv<'a>> ) -> Self {
        let object = jenv.new_object(
            bluetooth_le_class!(advertise_data_builder_java_name!()),
            "()",
            &[])
        .unwrap();

        AdvertisingDataBuilder {
            jenv: jenv.clone(),
            object: object.into_inner(),
            services: ::gap::advertise::service_class_uuid::new_128(false),
            include_device_name: false,
            include_tx_power_level: false,
        }
    }

    pub fn set_include_device_name( self, val: bool ) -> Self {
        let object = get_jvalue!{
            Object,
            self.jenv.call_method(
                jni::objects::JObject::from(self.object),
                "setIncludeDeviceName",
                format!("(Z)L{};", advertise_data_builder_java_name!()),
                &[jni::objects::JValue::Bool(val as jni::sys::jboolean)])
            .unwrap()
        };

         let mut ret = self.clone();

        ret.object = object.into_inner();
        ret.include_device_name = val;

        ret
    }

    pub fn set_include_tx_power_level( self, val: bool ) -> Self {
        let object = get_jvalue!{
            Object,
            self.jenv.call_method(
                jni::objects::JObject::from(self.object),
                "setIncludeTxPowerLevel",
                format!("(Z);{};", advertise_data_builder_java_name!()),
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
            "(I,I)",
            &[upper, lower])
        .unwrap();

        self.jenv.new_object(
            "android/os/ParcelUuid",
            "(Landroid/os/ParcelUuid;)",
            &[jni::objects::JValue::Object(java_uuid)])
        .unwrap()
    }

    /// This will not add duplicate service data, if the uuid is already in the list of uuids to
    /// advertise then just a copy of self is returned.
    pub fn add_service_data( self, uuid: u128 ) -> Self {
        let mut ret = self.clone();

        if ret.services.insert_uuid(uuid) {
            let object = get_jvalue!{
                Object,
                self.jenv.call_method(
                    jni::objects::JObject::from(self.object),
                    "addServiceUuid",
                    "(Landroid/os/ParcelUuid;)",
                    &[jni::objects::JValue::Object(self.get_uuid_parcel(uuid))])
                .unwrap()
            };

            ret.object = object.into_inner();
        }

        ret
    }
}

pub struct AdvertiseCallback<'a> {
    jenv: Rc<jni::JNIEnv<'a>>,
    object: jni::sys::jobject,
}

impl<'a> AdvertiseCallback<'a> {
    fn new( jenv: Rc<jni::JNIEnv<'a>>, callback: jni::objects::JObject<'a> ) -> Self {
        AdvertiseCallback {
            jenv: jenv.clone(),
            object: callback.into_inner(),
        }
    }
}
