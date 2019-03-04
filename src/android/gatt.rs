use jni::objects::JObject;
use jni::sys::jint;
use std::marker::Unsize;
use std::rc::Rc;
use super::Phy;

pub enum ConnectionPriority {
    Balanced,
    High,
    LowPower,
}

impl ConnectionPriority {
    fn val(&self) -> i32 {
        match *self {
            ConnectionPriority::Balanced => 0,
            ConnectionPriority::High => 1,
            ConnectionPriority::LowPower => 2,
        }
    }
}

pub struct BluetoothGatt<'a> {
    jenv: Rc<jni::JNIEnv<'a>>,
    object: jni::sys::jobject,
}

impl<'a> BluetoothGatt<'a> {
    const GATT_SUCCESS: jni::sys::jint = 0;

    pub fn new(jenv: Rc<jni::JNIEnv<'a>>, object: JObject<'a> ) -> Self {
        Self {
            jenv: jenv,
            object: object.into_inner(),
        }
    }

    /// This is equivalent to the `connect` android api method
    pub fn reconnect(&self) -> bool {
        self.jenv.call_method(
            self.object.into(),
            "connect",
            "()Z",
            &[]
        ).unwrap()
        .z()
        .unwrap()
    }

    pub fn disconnect(&self) {
        self.jenv.call_method(
            self.object.into(),
            "disconnect",
            "()V",
            &[]
        ).unwrap();
    }

    /// This is called through the implemention of Drop
    fn close(&self) {
        self.jenv.call_method(
            self.object.into(),
            "close",
            "()V",
            &[]
        ).unwrap();
    }

    pub fn discover_services(&self) -> bool {
        self.jenv.call_method (
            self.object.into(),
            "discoverServices",
            "()Z",
            &[]
        ).unwrap()
        .z()
        .unwrap()
    }

    /// TODO see if this should be made into an async method, or be a
    /// step of a higher level asyn
    pub fn execute_reliable_write(&self) -> bool {
        self.jenv.call_method (
            self.object.into(),
            "executeReliableWrite",
            "()Z",
            &[]
        ).unwrap()
        .z()
        .unwrap()
    }

    /// Get the service if it exists on the remote device
    pub fn get_service(&'a self, uuid: ::UUID) -> Result<BluetoothGattService<'a>,()> {
        use android::MakeJavaUUID;

        let object = get_jvalue!(
            Object,
            self.jenv.call_method(
                self.object.into(),
                "getService",
                format!("(Ljava.util.UUID;)L{};", bluetooth_class!("BluetoothGattService")),
                &[jni::objects::JValue::Object(uuid.make_java_uuid(self.jenv.as_ref()))])
            .unwrap()
        );

        if !object.is_null() {
            Ok(BluetoothGattService::from_obj(self.jenv.clone(), object))
        } else {
            Err(())
        }
    }

    /// Get all services on a remote device
    pub fn get_services(&'a self) -> Box<[BluetoothGattService<'a>]> {
        let list = get_jvalue!{
            Object,
            self.jenv.call_method(
                self.object.into(),
                "getServices",
                "()Ljava/util/List;",
                &[])
            .unwrap()
        };

        let list_size = get_jvalue! (
            Int,
            self.jenv.call_method(
                list,
                "size",
                "()I",
                &[])
            .unwrap()
        );

        (0..list_size).map(|index| {
            let object = get_jvalue!{
                Object,
                self.jenv.call_method(
                    list,
                    "get",
                    "(I)Ljava/lang/Object;",
                    &[jni::objects::JValue::Int(index)])
                .unwrap()
            };

            BluetoothGattService::from_obj(self.jenv.clone(), object)
        })
        .collect::<Vec<_>>()
        .into_boxed_slice()
    }

    /// TODO see if this should be made into an async method, or be a
    /// step of a higher level asyn
    pub fn read_characteristic(&self, characteristic: &BluetoothGattCharacteristic) -> bool {
        self.jenv.call_method(
            self.object.into(),
            "readCharacteristic",
            format!("(L{};)Z", bluetooth_class!("BluetoothGattCharacteristic")),
            &[jni::objects::JValue::Object(characteristic.object.into())])
        .unwrap()
        .z()
        .unwrap()
    }

    /// TODO see if this should be made into an async method, or be a
    /// step of a higher level asyn
    pub fn read_descriptor(&self, descriptor: &BluetoothGattDescriptor) -> bool {
        self.jenv.call_method(
            self.object.into(),
            "readDescriptor",
            format!("(L{};)Z", bluetooth_class!("BluetoothGattDescriptor")),
            &[jni::objects::JValue::Object(descriptor.object.into())])
        .unwrap()
        .z()
        .unwrap()
    }

    /// TODO see if this should be made into an async method, or be a
    /// step of a higher level asyn
    pub fn read_phy(&self) {
        self.jenv.call_method(
            self.object.into(),
            "readPhy",
            "()V",
            &[])
        .unwrap();
    }

    /// TODO see if this should be made into an async method, or be a
    /// step of a higher level asyn
    pub fn request_connection_priority(&self, cp: ConnectionPriority) -> bool {
        self.jenv.call_method(
            self.object.into(),
            "requestConnectionPriority",
            "(I)Z",
            &[jni::objects::JValue::Int(cp.val() as jni::sys::jint)])
        .unwrap()
        .z()
        .unwrap()
    }

    pub fn request_mtu(&self, mtu: jni::sys::jint) -> bool {
        self.jenv.call_method(
            self.object.into(),
            "requestMtu",
            "(I)Z",
            &[jni::objects::JValue::Int(mtu)])
        .unwrap()
        .z()
        .unwrap()
    }

    pub fn set_characteristic_notification(&self, characteristic: BluetoothGattCharacteristic, enable: bool) -> bool{
        self.jenv.call_method(
            self.object.into(),
            "setCharacteristicNotification",
            format!("(L{};Z)Z", bluetooth_class!("BluetoothGattCharacteristic")),
            &[
                jni::objects::JValue::Object(characteristic.object.into()),
                jni::objects::JValue::Bool(enable as jni::sys::jboolean)
            ])
        .unwrap()
        .z()
        .unwrap()
    }

    pub fn set_preferred_phy<C>(&self, tx_phy: ::android::Phy, rx_phy: ::android::Phy, coded_type: C)
        where C: Into<Option<::android::PhyCodedType>>
    {
        self.jenv.call_method (
            self.object.into(),
            "setPreferredPhy",
            "(III)V",
            &[
                jni::objects::JValue::Int(tx_phy.val()),
                jni::objects::JValue::Int(rx_phy.val()),
                jni::objects::JValue::Int(::android::PhyCodedType::val_from_opt(coded_type.into())),
            ])
        .unwrap();
    }

    pub fn write_characteristic(&self, characteristic: BluetoothGattCharacteristic) -> bool {
        self.jenv.call_method (
            self.object.into(),
            "writeCharacteristic",
            format!("(L{};)Z", bluetooth_class!("BluetoothGattCharacteristic")),
            &[jni::objects::JValue::Object(characteristic.object.into())])
        .unwrap()
        .z()
        .unwrap()
    }

    pub fn write_descriptor(&self, descriptor: BluetoothGattDescriptor) -> bool {
        self.jenv.call_method (
            self.object.into(),
            "writeDescriptor",
            format!("(L{};)Z", bluetooth_class!("BluetoothGattDescriptor")),
            &[jni::objects::JValue::Object(descriptor.object.into())])
        .unwrap()
        .z()
        .unwrap()
    }
}

impl<'a> Drop for BluetoothGatt<'a> {
    /// This will close the gatt client
    fn drop(&mut self) {
        self.close()
    }
}

pub struct BluetoothGattService<'a> {
    jenv: Rc<jni::JNIEnv<'a>>,
    object: jni::sys::jobject,
}

impl <'a> BluetoothGattService<'a> {
    const SERVICE_TYPE_PRIMARY: jni::sys::jint = 0;
    const SERVICE_TYPE_SECONDARY: jni::sys::jint = 1;

    fn from_obj(jenv: Rc<jni::JNIEnv<'a>>, object: JObject<'a> ) -> Self {
        Self {
            jenv: jenv,
            object: object.into_inner(),
        }
    }

    pub fn new( jenv: Rc<jni::JNIEnv<'a>>, uuid: ::UUID, is_primary: bool ) -> Self {
        use android::MakeJavaUUID;

        let object = jenv.new_object(
            bluetooth_class!("BluetoothGattService"),
            "(Ljava/util/UUID;I)V",
            &[
                jni::objects::JValue::Object(uuid.make_java_uuid(jenv.as_ref())),
                jni::objects::JValue::Int(
                    if is_primary {
                        Self::SERVICE_TYPE_PRIMARY
                    } else {
                        Self::SERVICE_TYPE_SECONDARY
                    }
                )
            ]
        ).unwrap();

        Self {
            jenv: jenv.clone(),
            object: object.into_inner()
        }
    }

    pub fn add_characteristic(&self, characteristic: BluetoothGattCharacteristic ) -> bool {
        self.jenv.call_method (
            self.object.into(),
            "addCharacteristic",
            format!("(L{};)Z", bluetooth_class!("BluetoothGattCharacteristic")),
            &[jni::objects::JValue::Object(characteristic.object.into())]
        ).unwrap()
        .z()
        .unwrap()
    }

    /// This is equivalent to the android api method `addService`
    pub fn include_service(&self, service: BluetoothGattService) -> bool {
        self.jenv.call_method (
            self.object.into(),
            "addService",
            format!("(L{};)Z", bluetooth_class!("BluetoothGattService")),
            &[jni::objects::JValue::Object(service.object.into())]
        ).unwrap()
        .z()
        .unwrap()
    }

    pub fn get_characteristic(&'a self, uuid: ::UUID) -> Option<BluetoothGattCharacteristic<'a>> {
        use android::MakeJavaUUID;

        let object = get_jvalue! {
            Object,
            self.jenv.call_method(
                self.object.into(),
                "getCharacteristic",
                format!("(Ljava/util/UUID;)L{};", bluetooth_class!("BluetoothGattCharacteristic")),
                &[jni::objects::JValue::Object(uuid.make_java_uuid(self.jenv.as_ref()))]
            ).unwrap()
        };

        if !object.is_null() {
            Some( BluetoothGattCharacteristic::from_obj(self.jenv.clone(), object) )
        } else {
            None
        }

    }

    pub fn get_characteristics(&'a self) -> Box<[BluetoothGattCharacteristic<'a>]> {
        let list = get_jvalue! {
            Object,
            self.jenv.call_method(
                self.object.into(),
                "getCharacteristics",
                "()Ljava/util/List;",
                &[]
            ).unwrap()
        };

        let list_size = get_jvalue! {
            Int,
            self.jenv.call_method(
                list,
                "size",
                "()I",
                &[]
            ).unwrap()
        };

        (0..list_size).map(|index| {
            let object = get_jvalue! {
                Object,
                self.jenv.call_method(
                    list,
                    "get",
                    "(I)Ljava/lang/Object;",
                    &[jni::objects::JValue::Int(index)]
                ).unwrap()
            };

            BluetoothGattCharacteristic::from_obj(self.jenv.clone(), object)
        })
        .collect::<Vec<_>>()
        .into_boxed_slice()
    }

    pub fn get_included_services(&'a self) -> Box<[BluetoothGattService<'a>]> {
        let list = get_jvalue! {
            Object,
            self.jenv.call_method(
                self.object.into(),
                "getIncludedServices",
                "()Ljava/util/List;",
                &[]
            ).unwrap()
        };

        let list_size = get_jvalue! {
            Int,
            self.jenv.call_method(
                list,
                "size",
                "()I",
                &[]
            ).unwrap()
        };

        (0..list_size).map(|index| {
            let object = get_jvalue! {
                Object,
                self.jenv.call_method(
                    list,
                    "get",
                    "(I)Ljava/lang/Object;",
                    &[jni::objects::JValue::Int(index)]
                ).unwrap()
            };

            BluetoothGattService::from_obj(self.jenv.clone(), object)
        })
        .collect::<Vec<_>>()
        .into_boxed_slice()
    }

    // Replacement for the method `getType`
    pub fn is_primary(&self) -> bool {
        Self::SERVICE_TYPE_PRIMARY == get_jvalue! {
            Int,
            self.jenv.call_method(
                self.object.into(),
                "getType",
                "()I",
                &[]
            )
            .unwrap()
        }
    }

    pub fn get_uuid(&self) -> ::UUID {
        use ::android::MakeJavaUUID;

        let juuid = get_jvalue! {
            Object,
            self.jenv.call_method(
                self.object.into(),
                "getUuid",
                "()Ljava/util/UUID;",
                &[]
            )
            .unwrap()
        };

        ::UUID::from_java_uuid(self.jenv.clone(), juuid)
    }
}

#[derive(Debug,Clone,Copy,PartialEq,Eq,PartialOrd,Ord)]
pub enum CharacteristicProperties {
    Broadcast,
    Read,
    WriteWithoutResponse,
    Write,
    Notify,
    Indicate,
    AuthenticatedSignedWrites,
    ExtendedProperties,
}

impl CharacteristicProperties {
    fn val(&self) -> jni::sys::jint {
        match *self {
            CharacteristicProperties::Broadcast => 0x1,
            CharacteristicProperties::Read => 0x2,
            CharacteristicProperties::WriteWithoutResponse => 0x4,
            CharacteristicProperties::Write => 0x8,
            CharacteristicProperties::Notify => 0x10,
            CharacteristicProperties::Indicate => 0x20,
            CharacteristicProperties::AuthenticatedSignedWrites => 0x40,
            CharacteristicProperties::ExtendedProperties => 0x80,
        }
    }

    fn from_val(val: jni::sys::jint) -> Self {
        match val {
            0x1  => CharacteristicProperties::Broadcast,
            0x2  => CharacteristicProperties::Read,
            0x4  => CharacteristicProperties::WriteWithoutResponse,
            0x8  => CharacteristicProperties::Write,
            0x10 => CharacteristicProperties::Notify,
            0x20 => CharacteristicProperties::Indicate,
            0x40 => CharacteristicProperties::AuthenticatedSignedWrites,
            0x80 => CharacteristicProperties::ExtendedProperties,
            _ => panic!("Unknown val")
        }
    }

    fn from_bit_field(field: jni::sys::jint ) -> Box<[CharacteristicProperties]> {
        (0..std::mem::size_of::<jni::sys::jint>() * 8)
            .map(|s| field & (1 << s) )
            .filter_map(|f| if f != 0 { Some(CharacteristicProperties::from_val(f)) } else { None })
            .collect::<Vec<_>>()
            .into_boxed_slice()
    }
}

#[derive(Debug,Clone,Copy,PartialEq,Eq,PartialOrd,Ord)]
pub enum Permissions {
    Read,
    ReadEncrypted,
    /// Encrypted read with man in the middle protection
    ReadEncryptedMITM,
    Write,
    WriteEncrypted,
    /// Encrypted write with man in the middle protection
    WriteEncryptedMITM,
    WriteSigned,
    /// Signed write with man in the middle protection
    WriteSignedMITM,
}

impl Permissions {
    fn val(&self) -> jni::sys::jint {
        match *self {
            Permissions::Read => 0x1,
            Permissions::ReadEncrypted => 0x2,
            Permissions::ReadEncryptedMITM => 0x4,
            Permissions::Write => 0x10,
            Permissions::WriteEncrypted => 0x20,
            Permissions::WriteEncryptedMITM => 0x40,
            Permissions::WriteSigned => 0x80,
            Permissions::WriteSignedMITM => 0x100,
        }
    }

    fn from_val(val: jni::sys::jint) -> Self {
        match val {
            0x1   => Permissions::Read,
            0x2   => Permissions::ReadEncrypted,
            0x4   => Permissions::ReadEncryptedMITM,
            0x10  => Permissions::Write,
            0x20  => Permissions::WriteEncrypted,
            0x40  => Permissions::WriteEncryptedMITM,
            0x80  => Permissions::WriteSigned,
            0x100 => Permissions::WriteSignedMITM,
            _ => panic!("Unknown val")
        }
    }

    fn from_bit_field(field: jni::sys::jint ) -> Box<[Permissions]> {
        (0..(std::mem::size_of::<jni::sys::jint>() * 8))
            .map(|s| field & (1 << s) )
            .filter_map(|f| if f != 0 { Some(Permissions::from_val(f)) } else { None })
            .collect::<Vec<_>>()
            .into_boxed_slice()
    }
}

pub struct BluetoothGattCharacteristic<'a> {
    jenv: Rc<jni::JNIEnv<'a>>,
    object: jni::sys::jobject,
}

impl<'a> BluetoothGattCharacteristic<'a> {

    fn from_obj(jenv: Rc<jni::JNIEnv<'a>>, object: JObject<'a> ) -> Self {
        Self {
            jenv: jenv,
            object: object.into_inner(),
        }
    }

    pub fn new( jenv: Rc<jni::JNIEnv<'a>>,
        uuid: ::UUID,
        properties: &[CharacteristicProperties],
        permissions: &[Permissions]) -> Self
    {
        use android::MakeJavaUUID;

        let prop_bit_field = properties.iter().fold(0, |bf, prop| bf | prop.val() );

        let perm_bit_field = permissions.iter().fold(0, |bf, perm| bf | perm.val() );

        let object = jenv.new_object(
            bluetooth_class!("BluetoothGattCharacteristic"),
            "(Ljava/util/UUID;II)V",
            &[
                jni::objects::JValue::Object(uuid.make_java_uuid(jenv.as_ref())),
                jni::objects::JValue::Int(prop_bit_field),
                jni::objects::JValue::Int(perm_bit_field),
            ]
        ).unwrap();

        Self {
            jenv: jenv.clone(),
            object: object.into_inner(),
        }
    }

    pub fn get_properties(&self) -> Box<[CharacteristicProperties]> {
        let bit_field = get_jvalue!{
            Int,
            self.jenv.call_method(
                self.object.into(),
                "getProperties",
                "()I",
                &[]
            ).unwrap()
        };

        CharacteristicProperties::from_bit_field(bit_field)
    }

    pub fn get_permissions(&self) -> Box<[Permissions]> {
        let bit_field = get_jvalue!{
            Int,
            self.jenv.call_method(
                self.object.into(),
                "getPermissions",
                "()I",
                &[]
            ).unwrap()
        };

        Permissions::from_bit_field(bit_field)
    }

    pub fn get_uuid(&self) -> ::UUID {
        let juuid = get_jvalue!{
            Object,
            self.jenv.call_method(
                self.object.into(),
                "getUuid",
                "()Ljava/util/UUID;",
                &[]
            ).unwrap()
        };

        <::UUID as ::android::MakeJavaUUID>::from_java_uuid(self.jenv.clone(), juuid)
    }

    pub fn add_descriptor(&self, descriptor: BluetoothGattDescriptor) -> bool {
        self.jenv.call_method(
            self.object.into(),
            "addDescriptor",
            format!("(L{};)Z", bluetooth_class!("BluetoothGattDescriptor")),
            &[jni::objects::JValue::Object(descriptor.object.into())]
        ).unwrap()
        .z()
        .unwrap()
    }

    pub fn get_descriptor(&self, uuid: ::UUID) -> Option<BluetoothGattDescriptor> {
        use android::MakeJavaUUID;

        let object = get_jvalue! {
            Object,
            self.jenv.call_method(
                self.object.into(),
                "getDescriptor",
                format!("(Ljava/util/UUID;)L{};", bluetooth_class!("BluetoothGattDescriptor")),
                &[jni::objects::JValue::Object(uuid.make_java_uuid(self.jenv.as_ref()))]
            ).unwrap()
        };

        if !object.is_null() {
            Some(BluetoothGattDescriptor::from_obj(self.jenv.clone(), object))
        } else {
            None
        }
    }
}

pub struct BluetoothGattDescriptor<'a> {
    jenv: Rc<jni::JNIEnv<'a>>,
    object: jni::sys::jobject,
}

impl<'a> BluetoothGattDescriptor<'a> {

    fn from_obj(jenv: Rc<jni::JNIEnv<'a>>, object: JObject<'a> ) -> Self {
        Self {
            jenv: jenv,
            object: object.into_inner(),
        }
    }

    pub fn new(jenv: Rc<jni::JNIEnv<'a>>, uuid: ::UUID, permissions: &[Permissions]) -> Self {
        use android::MakeJavaUUID;

        let bit_field = permissions.iter().fold(0, |bf, perm| bf | perm.val() );

        let object = jenv.new_object(
            bluetooth_class!("BluetoothGattDescriptor"),
            "(Ljava/util/UUID;I)V",
            &[
                jni::objects::JValue::Object(uuid.make_java_uuid(jenv.as_ref())),
                jni::objects::JValue::Int(bit_field),
            ]
        ).unwrap();

        Self{
            jenv: jenv.clone(),
            object: object.into_inner(),
        }
    }

    pub fn get_permissions(&self) -> Box<[Permissions]> {
        let bit_field = get_jvalue!{
            Int,
            self.jenv.call_method(
                self.object.into(),
                "getPermissions",
                "()I",
                &[]
            ).unwrap()
        };

        Permissions::from_bit_field(bit_field)
    }

    pub fn get_uuid(&self) -> ::UUID {
        let juuid = get_jvalue!{
            Object,
            self.jenv.call_method(
                self.object.into(),
                "getUuid",
                "()Ljava/util/UUID;",
                &[]
            ).unwrap()
        };

        <::UUID as ::android::MakeJavaUUID>::from_java_uuid(self.jenv.clone(), juuid)
    }

    pub fn get_value(&self) -> Box<[u8]> {
        let bytes = get_jvalue! {
            Object,
            self.jenv.call_method(
                self.object.into(),
                "getValue",
                "()[B",
                &[]
            ).unwrap()
        };

        self.jenv.convert_byte_array(bytes.into_inner()).unwrap().into_boxed_slice()
    }

    pub fn set_value(&self, bytes: &[u8]) -> bool {
        let jbytes = self.jenv.byte_array_from_slice(bytes).unwrap();

        self.jenv.call_method(
            self.object.into(),
            "setValue",
            "([B)Z",
            &[jni::objects::JValue::Object(jbytes.into())]
        ).unwrap()
        .z()
        .unwrap()
    }
}

pub enum ConnectionState {
    Disconnected,
    Connected,
}

impl ConnectionState {
    fn from_raw( raw: jint) -> Result<ConnectionState, jint> {
        match raw {
            // BluetoothProfile.STATE_DISCONNECTED
            0 => Ok(ConnectionState::Disconnected),
            // BluetoothProfile.STATE_CONNECTED
            2 => Ok(ConnectionState::Connected),
            _ => Err(raw)
        }
    }
}

#[macro_use]
mod callback {
    use android::Phy;
    use std::sync::{Arc,Mutex};
    use super::{
        BluetoothGatt,
        BluetoothGattCharacteristic,
        BluetoothGattDescriptor,
        ConnectionState
    };

    macro_rules! err_fn {
        () => { Box<dyn Fn() + Send> }
    }

    macro_rules! on_characteristic_changed {
        ($l:lifetime) => {Box<dyn Fn( BluetoothGatt<$l>, BluetoothGattCharacteristic<$l>) + Send>}
    }

    macro_rules! on_characteristic_read {
        ($l:lifetime) => {( Box<dyn Fn( BluetoothGatt<$l>, BluetoothGattCharacteristic<$l>) + Send>, err_fn!() )}
    }

    macro_rules! on_characteristic_write {
        ($l:lifetime) => {( Box<dyn Fn( BluetoothGatt<$l>, BluetoothGattCharacteristic<$l>) + Send>, err_fn!() )}
    }

    macro_rules! on_connection_state_change {
        ($l:lifetime) => {( Box<dyn Fn( BluetoothGatt<$l>, ConnectionState ) + Send>, err_fn!() )}
    }

    macro_rules! on_descriptor_read {
        ($l:lifetime) => {( Box<dyn Fn( BluetoothGatt<$l>, BluetoothGattDescriptor<$l> ) + Send>, err_fn!() )}
    }

    macro_rules! on_descriptor_write {
        ($l:lifetime) => {( Box<dyn Fn( BluetoothGatt<$l>, BluetoothGattDescriptor<$l> ) + Send>, err_fn!() )}
    }

    macro_rules! on_mtu_changed {
        ($l:lifetime) => {( Box<dyn Fn( BluetoothGatt<$l>, usize ) + Send>, err_fn!())}
    }

    macro_rules! on_phy_read {
        ($l:lifetime) => {( Box<dyn Fn(BluetoothGatt<$l>, Phy, Phy ) + Send>, err_fn!() )}
    }

    macro_rules! on_phy_update {
        ($l:lifetime) => {( Box<dyn Fn(BluetoothGatt<$l>, Phy, Phy ) + Send>, err_fn!() )}
    }

    macro_rules! on_read_remote_rssi {
        ($l:lifetime) => {( Box<dyn Fn(BluetoothGatt<$l>, i32 ) + Send>, err_fn!() )}
    }

    macro_rules! on_reliable_write_completed {
        ($l:lifetime) => {( Box<dyn Fn(BluetoothGatt<$l>) + Send>, err_fn!() )}
    }

    macro_rules! on_services_discovered {
        ($l:lifetime) => {( Box<dyn Fn(BluetoothGatt<$l>) + Send>, err_fn!() )}
    }

    pub struct GattCallbacks<'s> {
        pub on_characteristic_changed: Option<on_characteristic_changed!('s)>,
        pub on_characteristic_read: Option<on_characteristic_read!('s)>,
        pub on_characteristic_write: Option<on_characteristic_write!('s)>,
        pub on_connection_state_change: Option<on_connection_state_change!('s)>,
        pub on_descriptor_read: Option<on_descriptor_read!('s)>,
        pub on_descriptor_write: Option<on_descriptor_write!('s)>,
        pub on_mtu_changed: Option<on_mtu_changed!('s)>,
        pub on_phy_read: Option<on_phy_read!('s)>,
        pub on_phy_update: Option<on_phy_update!('s)>,
        pub on_read_remote_rssi: Option<on_read_remote_rssi!('s)>,
        pub on_reliable_write_completed: Option<on_reliable_write_completed!('s)>,
        pub on_services_discovered: Option<on_services_discovered!('s)>,
    }

    pub type HashMap<'s> = std::collections::HashMap<jni::sys::jint, GattCallbacks<'s>>;

    pub type Map<'s> = Arc<Mutex<self::HashMap<'s>>>;
}

struct BluetoothGattCallback<'a> {
    jenv: Rc<jni::JNIEnv<'a>>,
    object: jni::sys::jobject,

    #[cfg(feature = "android_test")]
    class: &'a jni::objects::GlobalRef,
}

impl<'a> BluetoothGattCallback<'a> {

    const GET_FROM_MAP_ERR_MSG: &'static str = "Cannot get object from BluetoothGattCallback's static map";

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

    fn get_hash_code(jenv: &jni::JNIEnv, object: JObject ) -> jni::sys::jint {
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

    fn new( jenv: Rc<jni::JNIEnv<'a>>,
            classloader: ::android::ClassLoader,
            builder: BluetoothGattCallbackBuilder<'static> ) -> Self
    {
        use android::load_classbytes::{DexBytesLoader, NativeMethod};
        use std::ffi::CString;
        use std::sync::Once;
        use jni::objects::GlobalRef;

        static CLASS_NAME: &'static str = "botie/BluetoothGattCallback";
        static mut CLASS: Option<GlobalRef> = None;
        static ONCE: Once = Once::new();

        ONCE.call_once(|| {
            let class = DexBytesLoader::new(
                    &::android::classbytes::GATT_CALLBACK_CLASS_BYTES,
                    CLASS_NAME)
                .add_native_method(
                    unsafe { NativeMethod::new(
                        CString::new("onCharacteristicChanged").unwrap(),
                        CString::new( format!("(L{};L{};)V",
                            bluetooth_class!("BluetoothGatt"),
                            bluetooth_class!("BluetoothGattCharacteristic"))).unwrap(),
                        Self::characteristic_changed as *const std::ffi::c_void,
                    ) })
                .add_native_method(
                    unsafe{ NativeMethod::new(
                        CString::new("onCharacteristicRead").unwrap(),
                        CString::new( format!("(L{};L{};I)V",
                            bluetooth_class!("BluetoothGatt"),
                            bluetooth_class!("BluetoothGattCharacteristic"))).unwrap(),
                        Self::characteristic_read as *const std::ffi::c_void,
                    )})
                .add_native_method(
                    unsafe{ NativeMethod::new(
                        CString::new("onCharacteristicWrite").unwrap(),
                        CString::new( format!("(L{};L{};I)V",
                            bluetooth_class!("BluetoothGatt"),
                            bluetooth_class!("BluetoothGattCharacteristic"))).unwrap(),
                        Self::characteristic_write as *const std::ffi::c_void,
                    )})
                .add_native_method(
                    unsafe { NativeMethod::new(
                        CString::new("onConnectionStateChange").unwrap(),
                        CString::new( format!("(L{};II)V",
                            bluetooth_class!("BluetoothGatt"))).unwrap(),
                        Self::connection_state_change as *const std::ffi::c_void,
                    )})
                .add_native_method(
                    unsafe{ NativeMethod::new(
                        CString::new("onDescriptorRead").unwrap(),
                        CString::new( format!("(L{};L{};I)V",
                            bluetooth_class!("BluetoothGatt"),
                            bluetooth_class!("BluetoothGattDescriptor"))).unwrap(),
                        Self::descriptor_read as *const std::ffi::c_void,
                    )})
                .add_native_method(
                    unsafe { NativeMethod::new(
                        CString::new("onDescriptorWrite").unwrap(),
                        CString::new( format!("(L{};L{};I)V",
                            bluetooth_class!("BluetoothGatt"),
                            bluetooth_class!("BluetoothGattDescriptor"))).unwrap(),
                        Self::descriptor_write as *const std::ffi::c_void,
                    )})
                .add_native_method(
                    unsafe { NativeMethod::new(
                        CString::new("onMtuChanged").unwrap(),
                        CString::new( format!("(L{};II)V",
                            bluetooth_class!("BluetoothGatt"))).unwrap(),
                        Self::mtu_changed as *const std::ffi::c_void,
                    )})
                .add_native_method(
                    unsafe { NativeMethod::new(
                        CString::new("onPhyRead").unwrap(),
                        CString::new( format!("(L{};III)V",
                            bluetooth_class!("BluetoothGatt"))).unwrap(),
                        Self::phy_read as *const std::ffi::c_void,
                    )})
                .add_native_method(
                    unsafe { NativeMethod::new(
                        CString::new("onPhyUpdate").unwrap(),
                        CString::new( format!("(L{};III)V",
                            bluetooth_class!("BluetoothGatt"))).unwrap(),
                        Self::phy_update as *const std::ffi::c_void,
                    )})
                .add_native_method(
                    unsafe { NativeMethod::new(
                        CString::new("onReadRemoteRssi").unwrap(),
                        CString::new( format!("(L{};II)V",
                            bluetooth_class!("BluetoothGatt"))).unwrap(),
                        Self::read_remote_rssi as *const std::ffi::c_void,
                    )})
                .add_native_method(
                    unsafe { NativeMethod::new(
                        CString::new("onReliableWriteCompleted").unwrap(),
                        CString::new( format!("(L{};I)V",
                            bluetooth_class!("BluetoothGatt"))).unwrap(),
                        Self::reliable_write_completed as *const std::ffi::c_void,
                    )})
                .add_native_method(
                    unsafe { NativeMethod::new(
                        CString::new("onServicesDiscovered").unwrap(),
                        CString::new( format!("(L{};I)V",
                            bluetooth_class!("BluetoothGatt"))).unwrap(),
                        Self::services_discovered as *const std::ffi::c_void,
                    )})
                .add_native_method(
                    unsafe { NativeMethod::new(
                        CString::new("cleanBotie").unwrap(),
                        CString::new("()V").unwrap(),
                        Self::clean as *const std::ffi::c_void,
                    ) })
                .load(&jenv, classloader);

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
            .insert( Self::get_hash_code(&jenv, object), builder.callbacks )
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

    extern "system" fn characteristic_changed(
        jenv: jni::JNIEnv<'static>,
        object: JObject<'static>,
        gatt_object: JObject<'static>,
        gatt_characteristic_object: JObject<'static>)
    {
        let rc_env = Rc::new(jenv);

        if let Some(ref callback) = Self::get_map_lock()
            .get( &Self::get_hash_code(rc_env.as_ref(), object) )
            .expect(Self::GET_FROM_MAP_ERR_MSG)
            .on_characteristic_changed
        {
            callback(
                BluetoothGatt::new(rc_env.clone(), gatt_object),
                BluetoothGattCharacteristic::from_obj(rc_env.clone(), gatt_characteristic_object),
            );
        }
    }

    extern "system" fn characteristic_read(
        jenv: jni::JNIEnv<'static>,
        object: JObject<'static>,
        gatt_object: JObject<'static>,
        gatt_characteristic_object: JObject<'static>,
        status: jint)
    {
        let rc_env = Rc::new(jenv);

        if let Some((ref on_success, ref on_fail)) = Self::get_map_lock()
            .get( &Self::get_hash_code(rc_env.as_ref(), object))
            .expect(Self::GET_FROM_MAP_ERR_MSG)
            .on_characteristic_read
        {
            if BluetoothGatt::GATT_SUCCESS == status {
                on_success(
                    BluetoothGatt::new(rc_env.clone(), gatt_object),
                    BluetoothGattCharacteristic::from_obj(rc_env.clone(), gatt_characteristic_object)
                );
            } else {
                on_fail()
            }
        }
    }

    extern "system" fn characteristic_write(
        jenv: jni::JNIEnv<'static>,
        object: JObject<'static>,
        gatt_object: JObject<'static>,
        gatt_characteristic_object: JObject<'static>,
        status: jint)
    {
        let rc_env = Rc::new(jenv);

        if let Some((ref on_success, ref on_fail)) = Self::get_map_lock()
            .get( &Self::get_hash_code(rc_env.as_ref(), object))
            .expect(Self::GET_FROM_MAP_ERR_MSG)
            .on_characteristic_write
        {
            if BluetoothGatt::GATT_SUCCESS == status {
                on_success(
                    BluetoothGatt::new(rc_env.clone(), gatt_object),
                    BluetoothGattCharacteristic::from_obj(rc_env.clone(), gatt_characteristic_object)
                )
            } else {
                on_fail()
            }
        }
    }

    extern "system" fn connection_state_change(
        jenv: jni::JNIEnv<'static>,
        object: JObject<'static>,
        gatt_object: JObject<'static>,
        status: jint,
        new_state: jint)
    {
        let rc_env = Rc::new(jenv);

        if let Some((ref on_success, ref on_fail)) = Self::get_map_lock()
            .get( &Self::get_hash_code(rc_env.as_ref(), object))
            .expect(Self::GET_FROM_MAP_ERR_MSG)
            .on_connection_state_change
        {
            if BluetoothGatt::GATT_SUCCESS == status {
                on_success(
                    BluetoothGatt::new(rc_env.clone(), gatt_object),
                    ConnectionState::from_raw(new_state).expect("Unknown Connection State")
                )
            } else {
                on_fail()
            }
        }
    }

    extern "system" fn descriptor_read(
        jenv: jni::JNIEnv<'static>,
        object: JObject<'static>,
        gatt_object: JObject<'static>,
        gatt_descriptor_object: JObject<'static>,
        status: jint)
    {
        let rc_env = Rc::new(jenv);

        if let Some((ref on_success, ref on_fail)) = Self::get_map_lock()
            .get( &Self::get_hash_code(rc_env.as_ref(), object))
            .expect(Self::GET_FROM_MAP_ERR_MSG)
            .on_descriptor_read
        {
            if BluetoothGatt::GATT_SUCCESS == status {
                on_success(
                    BluetoothGatt::new(rc_env.clone(), gatt_object),
                    BluetoothGattDescriptor::from_obj(rc_env.clone(), gatt_descriptor_object)
                )
            } else {
                on_fail()
            }
        }
    }

    extern "system" fn descriptor_write(
        jenv: jni::JNIEnv<'static>,
        object: JObject<'static>,
        gatt_object: JObject<'static>,
        gatt_descriptor_object: JObject<'static>,
        status: jint)
    {
        let rc_env = Rc::new(jenv);

        if let Some((ref on_success, ref on_fail)) = Self::get_map_lock()
            .get( &Self::get_hash_code(rc_env.as_ref(), object))
            .expect(Self::GET_FROM_MAP_ERR_MSG)
            .on_descriptor_write
        {
            if BluetoothGatt::GATT_SUCCESS == status {
                on_success(
                    BluetoothGatt::new(rc_env.clone(), gatt_object),
                    BluetoothGattDescriptor::from_obj(rc_env.clone(), gatt_descriptor_object)
                )
            } else {
                on_fail()
            }
        }
    }

    extern "system" fn mtu_changed(
        jenv: jni::JNIEnv<'static>,
        object: JObject<'static>,
        gatt_object: JObject<'static>,
        mtu: jint,
        status: jint)
    {
        let rc_env = Rc::new(jenv);

        if let Some((ref on_success, ref on_fail)) = Self::get_map_lock()
            .get(&Self::get_hash_code(rc_env.as_ref(), object))
            .expect(Self::GET_FROM_MAP_ERR_MSG)
            .on_mtu_changed
        {
            if BluetoothGatt::GATT_SUCCESS == status {
                on_success(
                    BluetoothGatt::new(rc_env.clone(), gatt_object),
                    mtu as usize
                )
            } else {
                on_fail()
            }
        }
    }

    extern "system" fn phy_read(
        jenv: jni::JNIEnv<'static>,
        object: JObject<'static>,
        gatt_object: JObject<'static>,
        tx_phy: jint,
        rx_phy: jint,
        status: jint)
    {
        let rc_env = Rc::new(jenv);

        if let Some((ref on_success, ref on_fail)) = Self::get_map_lock()
            .get(&Self::get_hash_code(rc_env.as_ref(), object))
            .expect(Self::GET_FROM_MAP_ERR_MSG)
            .on_phy_read
        {
            if BluetoothGatt::GATT_SUCCESS == status {
                on_success(
                    BluetoothGatt::new(rc_env.clone(), object),
                    Phy::from_raw(tx_phy).expect("Cannot get Phy"),
                    Phy::from_raw(rx_phy).expect("Cannot get Phy"),
                )
            } else {
                on_fail()
            }
        }
    }

    extern "system" fn phy_update(
        jenv: jni::JNIEnv<'static>,
        object: JObject<'static>,
        gatt_object: JObject<'static>,
        tx_phy: jint,
        rx_phy: jint,
        status: jint)
    {
        let rc_env = Rc::new(jenv);

        if let Some((ref on_success, ref on_fail)) = Self::get_map_lock()
            .get(&Self::get_hash_code(rc_env.as_ref(), object))
            .expect(Self::GET_FROM_MAP_ERR_MSG)
            .on_phy_update
        {
            if BluetoothGatt::GATT_SUCCESS == status {
                on_success(
                    BluetoothGatt::new(rc_env.clone(), gatt_object),
                    Phy::from_raw(tx_phy).expect("Cannot get Phy"),
                    Phy::from_raw(rx_phy).expect("Cannot get Phy")
                )
            } else {
                on_fail()
            }
        }
    }

    extern "system" fn read_remote_rssi(
        jenv: jni::JNIEnv<'static>,
        object: JObject<'static>,
        gatt_object: JObject<'static>,
        rssi: jint,
        status: jint)
    {
        let rc_env = Rc::new(jenv);

        if let Some((ref on_success, ref on_fail)) = Self::get_map_lock()
            .get(&Self::get_hash_code(rc_env.as_ref(), object))
            .expect(Self::GET_FROM_MAP_ERR_MSG)
            .on_read_remote_rssi
        {
            if BluetoothGatt::GATT_SUCCESS == status {
                on_success(
                    BluetoothGatt::new(rc_env.clone(), gatt_object),
                    rssi as i32
                )
            } else {
                on_fail()
            }
        }
    }

    extern "system" fn reliable_write_completed(
        jenv: jni::JNIEnv<'static>,
        object: JObject<'static>,
        gatt_object: JObject<'static>,
        status: jint)
    {
        let rc_env = Rc::new(jenv);

        if let Some((ref on_success, ref on_fail)) = Self::get_map_lock()
            .get(&Self::get_hash_code(rc_env.as_ref(), object))
            .expect(Self::GET_FROM_MAP_ERR_MSG)
            .on_reliable_write_completed
        {
            if BluetoothGatt::GATT_SUCCESS == status {
                on_success(BluetoothGatt::new(rc_env.clone(), gatt_object))
            } else {
                on_fail()
            }
        }
    }

    extern "system" fn services_discovered(
        jenv: jni::JNIEnv<'static>,
        object: JObject<'static>,
        gatt_object: JObject<'static>,
        status: jint)
    {
        let rc_env = Rc::new(jenv);

        if let Some((ref on_success, ref on_fail)) = Self::get_map_lock()
            .get(&Self::get_hash_code(rc_env.as_ref(), object))
            .expect(Self::GET_FROM_MAP_ERR_MSG)
            .on_services_discovered
        {
            if BluetoothGatt::GATT_SUCCESS == status {
                on_success(BluetoothGatt::new(rc_env.clone(), gatt_object))
            } else {
                on_fail()
            }
        }
    }

    extern "system" fn clean( jenv : jni::JNIEnv, object: JObject ) {
        Self::get_map_lock().remove( &Self::get_hash_code(&jenv, object) )
            .expect("Cannot get object from AdvertiseCallback's static map");
    }
}

struct BluetoothGattCallbackBuilder<'s> {
    callbacks: callback::GattCallbacks<'s>
}

impl<'s> BluetoothGattCallbackBuilder<'s> {
    fn new() -> Self {
        Self {
            callbacks: callback::GattCallbacks {
                on_characteristic_changed: None,
                on_characteristic_read: None,
                on_characteristic_write: None,
                on_connection_state_change: None,
                on_descriptor_read: None,
                on_descriptor_write: None,
                on_mtu_changed: None,
                on_phy_read: None,
                on_phy_update: None,
                on_read_remote_rssi: None,
                on_reliable_write_completed: None,
                on_services_discovered: None,
            }
        }
    }

    fn set_on_characteristic_changed<F>(self, callback: F) -> Self
    where F: 'static + Unsize<dyn Fn( BluetoothGatt<'s>, BluetoothGattCharacteristic<'s>) + Send> + Sized
    {
        let mut ret = self;

        ret.callbacks.on_characteristic_changed =
            Some(Box::new(callback) as Box<dyn Fn(BluetoothGatt<'s>, BluetoothGattCharacteristic<'s>) + Send>);

        ret
    }

    fn set_on_caracteristic_read<F,E>(self, on_success: F, on_fail: E) -> Self
    where F: 'static + Unsize<dyn Fn(BluetoothGatt<'s>, BluetoothGattCharacteristic<'s>)+ Send> + Sized,
          E: 'static + Unsize<dyn Fn() + Send> + Sized
    {
        let success = Box::new(on_success) as Box<dyn Fn(BluetoothGatt<'s>, BluetoothGattCharacteristic<'s>) + Send>;

        let error = Box::new(on_fail) as Box<dyn Fn() + Send>;

        let mut ret = self;

        ret.callbacks.on_characteristic_read = Some( (success, error) );

        ret
    }

    fn set_on_characteristic_write<F,E>(self, on_success: F, on_fail: E) -> Self
    where F: 'static + Unsize<dyn Fn(BluetoothGatt<'s>, BluetoothGattCharacteristic<'s>) + Send> + Sized,
          E: 'static + Unsize<dyn Fn() + Send> + Sized
    {
        let success = Box::new(on_success) as Box<dyn Fn(BluetoothGatt<'s>, BluetoothGattCharacteristic<'s>) + Send>;

        let error = Box::new(on_fail) as Box<dyn Fn() + Send>;

        let mut ret = self;

        ret.callbacks.on_characteristic_write = Some( (success, error) );

        ret
    }

    fn set_on_connection_state_change<F,E>(self, on_success: F, on_fail: E) -> Self
    where F: 'static + Unsize<dyn Fn(BluetoothGatt<'s>, ConnectionState) + Send> + Sized,
          E: 'static + Unsize<dyn Fn() + Send> + Sized
    {
        let success = Box::new(on_success) as Box<dyn Fn(BluetoothGatt<'s>, ConnectionState) + Send>;

        let error = Box::new(on_fail) as Box<dyn Fn() + Send>;

        let mut ret = self;

        ret.callbacks.on_connection_state_change = Some( (success, error) );

        ret
    }

    fn set_on_descriptor_read<F,E>(self, on_success: F, on_fail: E) -> Self
    where F: 'static + Unsize<dyn Fn(BluetoothGatt<'s>, BluetoothGattDescriptor<'s>) + Send> + Sized,
          E: 'static + Unsize<dyn Fn() + Send> + Sized
    {
        let success = Box::new(on_success) as Box<dyn Fn(BluetoothGatt<'s>, BluetoothGattDescriptor<'s>) + Send>;

        let error = Box::new(on_fail) as Box<dyn Fn() + Send>;

        let mut ret = self;

        ret.callbacks.on_descriptor_read = Some( (success, error) );

        ret
    }

    fn set_on_descriptor_write<F,E>(self, on_success: F, on_fail: E) -> Self
    where F: 'static + Unsize<dyn Fn(BluetoothGatt<'s>, BluetoothGattDescriptor<'s>) + Send> + Sized,
          E: 'static + Unsize<dyn Fn() + Send> + Sized
    {
        let success = Box::new(on_success) as Box<dyn Fn(BluetoothGatt<'s>, BluetoothGattDescriptor<'s>) + Send>;

        let error = Box::new(on_fail) as Box<dyn Fn() + Send>;

        let mut ret = self;

        ret.callbacks.on_descriptor_write = Some( (success, error) );

        ret
    }

    fn set_on_mtu_changed<F,E>(self, on_success: F, on_fail: E) -> Self
    where F: 'static + Unsize<dyn Fn(BluetoothGatt<'s>, usize ) + Send> + Sized,
          E: 'static + Unsize<dyn Fn() + Send> + Sized,
    {
        let success = Box::new(on_success) as Box<dyn Fn(BluetoothGatt<'s>, usize ) + Send>;

        let error = Box::new(on_fail) as Box<dyn Fn() + Send>;

        let mut ret = self;

        ret.callbacks.on_mtu_changed = Some( (success, error) );

        ret
    }

    fn set_on_phy_read<F,E>(self, on_success: F, on_fail: E) -> Self
    where F: 'static + Unsize<dyn Fn(BluetoothGatt<'s>, Phy, Phy) + Send> + Sized,
          E: 'static + Unsize<dyn Fn() + Send> + Sized,
    {
        let success = Box::new(on_success) as Box<dyn Fn(BluetoothGatt<'s>, Phy, Phy) + Send>;

        let error = Box::new(on_fail) as Box<dyn Fn() + Send>;

        let mut ret = self;

        ret.callbacks.on_phy_read = Some( (success, error) );

        ret
    }

    fn set_on_phy_update<F,E>(self, on_success: F, on_fail: E) -> Self
    where F: 'static + Unsize<dyn Fn(BluetoothGatt<'s>, Phy, Phy) + Send> + Sized,
          E: 'static + Unsize<dyn Fn() + Send> + Sized,
    {
        let success = Box::new(on_success) as Box<dyn Fn(BluetoothGatt<'s>, Phy, Phy) + Send>;

        let error = Box::new(on_fail) as Box<dyn Fn() + Send>;

        let mut ret = self;

        ret.callbacks.on_phy_update = Some( (success, error) );

        ret
    }

    fn set_on_read_remote_rssi<F,E>(self, on_success: F, on_fail: E) -> Self
    where F: 'static + Unsize<dyn Fn(BluetoothGatt<'s>, i32 ) + Send> + Sized,
          E: 'static + Unsize<dyn Fn() + Send> + Sized,
    {
        let success = Box::new(on_success) as Box<dyn Fn(BluetoothGatt<'s>, i32 ) + Send>;

        let error = Box::new(on_fail) as Box<dyn Fn() + Send>;

        let mut ret = self;

        ret.callbacks.on_read_remote_rssi = Some( (success, error) );

        ret
    }

    fn set_on_reliable_write_completed<F,E>(self, on_success: F, on_fail: E) -> Self
    where F: 'static + Unsize<dyn Fn(BluetoothGatt<'s>) + Send> + Sized,
          E: 'static + Unsize<dyn Fn() + Send>
    {
        let success = Box::new(on_success) as Box<dyn Fn(BluetoothGatt<'s>) + Send>;

        let error = Box::new(on_fail) as Box<dyn Fn() + Send>;

        let mut ret = self;

        ret.callbacks.on_reliable_write_completed = Some( (success, error) );

        ret
    }

    fn set_on_services_discovered<F,E>(self, on_success: F, on_fail: E) -> Self
    where F: 'static + Unsize<dyn Fn(BluetoothGatt<'s>) + Send> + Sized,
          E: 'static + Unsize<dyn Fn() + Send>
    {
        let success = Box::new(on_success) as Box<dyn Fn(BluetoothGatt<'s>) + Send>;

        let error = Box::new(on_fail) as Box<dyn Fn() + Send>;

        let mut ret = self;

        ret.callbacks.on_services_discovered = Some( (success, error) );

        ret
    }

    fn build<'a>(self, jenv: Rc<jni::JNIEnv<'a>>, classloader: ::android::ClassLoader) -> BluetoothGattCallback<'a>
    where Self: 'static
    {
        BluetoothGattCallback::new(jenv, classloader, self)
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
    fn bluetooth_gatt_callback_initialize(jenv: JNIEnv) {
        use android::ClassLoader;

        let rc_env = Rc::new(jenv);

        BluetoothGattCallbackBuilder::new()
            .build(rc_env.clone(), ClassLoader::system(rc_env.as_ref()));
    }

    /// A closure to forget the first parameter in a callback set for BluetoothGattCallback.
    ///
    /// Many of the tests create a java gatt object by just calling the jni::JNIEnv method
    /// alloc_object, and then construct a rust BluetoothGatt wrapper from that. The problem is
    /// that when the BluletoothGatt object is dropped, the java method
    /// ["close"](https://developer.android.com/reference/android/bluetooth/BluetoothGatt.html#close())
    /// is called, which will cause a 'process crash' to occur. So this macro will "forget" the
    /// BluetoothGatt object so that the method "drop" isn't called.
    macro_rules! forget_gatt {
        ( move | $gatt_arg:ident $(, $args:pat )* | $( $inner:tt )* ) => {
            move | $gatt_arg: self::BluetoothGatt $(, $args )* | {
                { $($inner)* };
                ::std::mem::forget($gatt_arg);
            }
        };
        ( move | _ $(, $args:pat )* | $( $inner:tt )* ) => {
            { forget_gatt!(move | ___g $(, $args )* | $( $inner )* ) }
        }
    }

    macro_rules! callback_test {
        (
            $rc_env:ident,
            $callback:ident,
            $method_name:expr,
            $method_parameter_class_objects:expr,
            $method_parameters:expr $(,)*
        ) =>
        {
            let reflected_method_name = $rc_env.new_string($method_name).unwrap();

            let method_parameters_types = $rc_env.new_object_array(
                $method_parameter_class_objects.len() as jni::sys::jsize,
                "java/lang/Class",
                $rc_env.find_class("java/lang/Class").unwrap().into(),
            )
            .expect("Coudln't create array");

            $method_parameter_class_objects.iter()
                .zip( 0..($method_parameter_class_objects.len() as jni::sys::jsize) )
                .for_each(|(class, index): (&jni::objects::JObject, _)| {
                    $rc_env.set_object_array_element(
                        method_parameters_types,
                        index,
                        *class,
                    )
                    .expect("Failed to set parameter type array element");
                });

            let reflected_method = get_jvalue! {
                Object,
                $rc_env.call_method(
                    $callback.class.into(),
                    "getMethod",
                    "(Ljava/lang/String;[Ljava/lang/Class;)Ljava/lang/reflect/Method;",
                    &[  jni::objects::JValue::Object(reflected_method_name.into()),
                        jni::objects::JValue::Object(JObject::from(method_parameters_types))
                    ]
                )
                .expect("Couldn't create reflect method")
            };

            let method_parameters = $rc_env.new_object_array(
                $method_parameters.len() as jni::sys::jsize,
                "java/lang/Object",
                $rc_env.new_object("java/lang/Object", "()V", &[]).unwrap()
            )
            .unwrap();

            $method_parameters.iter()
                .zip( 0..($method_parameters.len() as jni::sys::jsize))
                .for_each(|(object, index)|{
                    $rc_env.set_object_array_element(
                        method_parameters,
                        index,
                        *object,
                    )
                    .expect("Failed to set parameter array element");
                });

            $rc_env.call_method(
                reflected_method,
                "invoke",
                "(Ljava/lang/Object;[Ljava/lang/Object;)Ljava/lang/Object;",
                &[
                    jni::objects::JValue::Object($callback.object.into()),
                    jni::objects::JValue::Object(method_parameters.into())
                ]
            )
            .expect("Method invoke failed");
        }
    }

    #[android_test]
    fn bluetooth_gatt_callback_set_on_characteristic_changed(jenv: JNIEnv) {
        use android::ClassLoader;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        let rc_env = Rc::new(jenv);

        let flag = Arc::new(AtomicBool::new(false));

        let flag_clone = flag.clone();

        let callback = BluetoothGattCallbackBuilder::new()
            .set_on_characteristic_changed(
                forget_gatt!(move |_,_| flag_clone.store(true, Ordering::Relaxed))
            )
            .build(rc_env.clone(), ClassLoader::system(rc_env.as_ref()));

        callback_test!(
            rc_env,
            callback,
            "onCharacteristicChanged",
            &[
                jni::objects::JObject::from(rc_env.find_class(bluetooth_class!("BluetoothGatt")).unwrap()),
                rc_env.find_class(bluetooth_class!("BluetoothGattCharacteristic")).unwrap().into(),
            ],
            &[
                rc_env.alloc_object(bluetooth_class!("BluetoothGatt")).unwrap(),
                rc_env.alloc_object(bluetooth_class!("BluetoothGattCharacteristic")).unwrap(),
            ]
        );

        assert!( flag.load(Ordering::Relaxed) );
    }

    #[android_test]
    fn bluetooth_gatt_callback_set_on_caracteristic_read(jenv: JNIEnv) {
        use android::ClassLoader;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        let rc_env = Rc::new(jenv);

        let flag1 = Arc::new(AtomicBool::new(false));
        let flag2 = Arc::new(AtomicBool::new(false));

        let flag1_clone = flag1.clone();
        let flag2_clone = flag2.clone();

        let callback = BluetoothGattCallbackBuilder::new()
            .set_on_caracteristic_read(
                forget_gatt!( move |_,_| flag1_clone.store(true, Ordering::Relaxed) ),
                move || flag2_clone.store(true, Ordering::Relaxed) )
            .build(rc_env.clone(), ClassLoader::system(rc_env.as_ref()));

        callback_test!(
            rc_env,
            callback,
            "onCharacteristicRead",
            &[
                rc_env.find_class(bluetooth_class!("BluetoothGatt")).unwrap().into(),
                rc_env.find_class(bluetooth_class!("BluetoothGattCharacteristic")).unwrap().into(),
                get_jvalue! {
                    Object,
                    callback.jenv.get_static_field(
                        "java/lang/Integer",
                        "TYPE",
                        "Ljava/lang/Class;"
                    )
                    .unwrap()
                }
            ],
            &[
                rc_env.alloc_object(bluetooth_class!("BluetoothGatt")).unwrap(),
                rc_env.alloc_object(bluetooth_class!("BluetoothGattCharacteristic")).unwrap(),
                rc_env.new_object(
                    "java/lang/Integer",
                    "(I)V",
                    &[jni::objects::JValue::Int(BluetoothGatt::GATT_SUCCESS)]
                )
                .unwrap()
            ]
        );

        callback_test!(
            rc_env,
            callback,
            "onCharacteristicRead",
            &[
                rc_env.find_class(bluetooth_class!("BluetoothGatt")).unwrap().into(),
                rc_env.find_class(bluetooth_class!("BluetoothGattCharacteristic")).unwrap().into(),
                get_jvalue! {
                    Object,
                    callback.jenv.get_static_field(
                        "java/lang/Integer",
                        "TYPE",
                        "Ljava/lang/Class;"
                    )
                    .unwrap()
                }
            ],
            &[
                rc_env.alloc_object(bluetooth_class!("BluetoothGatt")).unwrap(),
                rc_env.alloc_object(bluetooth_class!("BluetoothGattCharacteristic")).unwrap(),
                rc_env.new_object(
                    "java/lang/Integer",
                    "(I)V",
                    &[jni::objects::JValue::Int(-1)]
                )
                .unwrap()
            ]
        );

        assert!( flag1.load(Ordering::Relaxed) );
        assert!( flag2.load(Ordering::Relaxed) );
    }

    #[android_test]
    fn bluetooth_gatt_callback_set_on_caracteristic_write(jenv: JNIEnv) {
        use android::ClassLoader;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        let rc_env = Rc::new(jenv);

        let flag1 = Arc::new(AtomicBool::new(false));
        let flag2 = Arc::new(AtomicBool::new(false));

        let flag1_clone = flag1.clone();
        let flag2_clone = flag2.clone();

        let callback = BluetoothGattCallbackBuilder::new()
            .set_on_characteristic_write(
                forget_gatt!( move |_,_| flag1_clone.store(true, Ordering::Relaxed) ),
                move || flag2_clone.store(true, Ordering::Relaxed) )
            .build(rc_env.clone(), ClassLoader::system(rc_env.as_ref()));

        callback_test!(
            rc_env,
            callback,
            "onCharacteristicWrite",
            &[
                rc_env.find_class(bluetooth_class!("BluetoothGatt")).unwrap().into(),
                rc_env.find_class(bluetooth_class!("BluetoothGattCharacteristic")).unwrap().into(),
                get_jvalue! {
                    Object,
                    callback.jenv.get_static_field(
                        "java/lang/Integer",
                        "TYPE",
                        "Ljava/lang/Class;"
                    )
                    .unwrap()
                }
            ],
            &[
                rc_env.alloc_object(bluetooth_class!("BluetoothGatt")).unwrap(),
                rc_env.alloc_object(bluetooth_class!("BluetoothGattCharacteristic")).unwrap(),
                rc_env.new_object(
                    "java/lang/Integer",
                    "(I)V",
                    &[jni::objects::JValue::Int(BluetoothGatt::GATT_SUCCESS)]
                )
                .unwrap()
            ]
        );

        callback_test!(
            rc_env,
            callback,
            "onCharacteristicWrite",
            &[
                rc_env.find_class(bluetooth_class!("BluetoothGatt")).unwrap().into(),
                rc_env.find_class(bluetooth_class!("BluetoothGattCharacteristic")).unwrap().into(),
                get_jvalue! {
                    Object,
                    callback.jenv.get_static_field(
                        "java/lang/Integer",
                        "TYPE",
                        "Ljava/lang/Class;"
                    )
                    .unwrap()
                }
            ],
            &[
                rc_env.alloc_object(bluetooth_class!("BluetoothGatt")).unwrap(),
                rc_env.alloc_object(bluetooth_class!("BluetoothGattCharacteristic")).unwrap(),
                rc_env.new_object(
                    "java/lang/Integer",
                    "(I)V",
                    &[jni::objects::JValue::Int(-1)]
                )
                .unwrap()
            ]
        );

        assert!( flag1.load(Ordering::Relaxed) );
        assert!( flag2.load(Ordering::Relaxed) );
    }

    #[android_test]
    fn bluetooth_gatt_callback_set_on_characteristic_state_changed(jenv: JNIEnv) {
        use android::ClassLoader;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        let rc_env = Rc::new(jenv);

        let flag1 = Arc::new(AtomicBool::new(false));
        let flag2 = Arc::new(AtomicBool::new(false));

        let flag1_clone = flag1.clone();
        let flag2_clone = flag2.clone();

        let callback = BluetoothGattCallbackBuilder::new()
            .set_on_connection_state_change(
                forget_gatt!( move |_,_| flag1_clone.store(true, Ordering::Relaxed) ),
                move || flag2_clone.store(true, Ordering::Relaxed) )
            .build(rc_env.clone(), ClassLoader::system(rc_env.as_ref()));

        callback_test!(
            rc_env,
            callback,
            "onConnectionStateChange",
            &[
                rc_env.find_class(bluetooth_class!("BluetoothGatt")).unwrap().into(),
                get_jvalue! {
                    Object,
                    callback.jenv.get_static_field(
                        "java/lang/Integer",
                        "TYPE",
                        "Ljava/lang/Class;"
                    )
                    .unwrap()
                },
                get_jvalue! {
                    Object,
                    callback.jenv.get_static_field(
                        "java/lang/Integer",
                        "TYPE",
                        "Ljava/lang/Class;"
                    )
                    .unwrap()
                }
            ],
            &[
                rc_env.alloc_object(bluetooth_class!("BluetoothGatt")).unwrap(),
                rc_env.new_object(
                    "java/lang/Integer",
                    "(I)V",
                    &[jni::objects::JValue::Int(BluetoothGatt::GATT_SUCCESS)]
                )
                .unwrap(),
                rc_env.new_object(
                    "java/lang/Integer",
                    "(I)V",
                    &[jni::objects::JValue::Int(0)]
                )
                .unwrap()
            ]
        );

        callback_test!(
            rc_env,
            callback,
            "onConnectionStateChange",
            &[
                rc_env.find_class(bluetooth_class!("BluetoothGatt")).unwrap().into(),
                get_jvalue! {
                    Object,
                    callback.jenv.get_static_field(
                        "java/lang/Integer",
                        "TYPE",
                        "Ljava/lang/Class;"
                    )
                    .unwrap()
                },
                get_jvalue! {
                    Object,
                    callback.jenv.get_static_field(
                        "java/lang/Integer",
                        "TYPE",
                        "Ljava/lang/Class;"
                    )
                    .unwrap()
                }
            ],
            &[
                rc_env.alloc_object(bluetooth_class!("BluetoothGatt")).unwrap(),
                rc_env.new_object(
                    "java/lang/Integer",
                    "(I)V",
                    &[jni::objects::JValue::Int(-1)]
                )
                .unwrap(),
                rc_env.new_object(
                    "java/lang/Integer",
                    "(I)V",
                    &[jni::objects::JValue::Int(0)]
                )
                .unwrap()
            ]
        );

        assert!( flag1.load(Ordering::Relaxed) );
        assert!( flag2.load(Ordering::Relaxed) );
    }

    #[android_test]
    fn bluetooth_gatt_callback_set_on_descriptor_read(jenv: JNIEnv) {
        use android::ClassLoader;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        let rc_env = Rc::new(jenv);

        let flag1 = Arc::new(AtomicBool::new(false));
        let flag2 = Arc::new(AtomicBool::new(false));

        let flag1_clone = flag1.clone();
        let flag2_clone = flag2.clone();

        let callback = BluetoothGattCallbackBuilder::new()
            .set_on_descriptor_read(
                forget_gatt!( move |_,_| flag1_clone.store(true, Ordering::Relaxed) ),
                move || flag2_clone.store(true, Ordering::Relaxed) )
            .build(rc_env.clone(), ClassLoader::system(rc_env.as_ref()));

        callback_test!(
            rc_env,
            callback,
            "onDescriptorRead",
            &[
                rc_env.find_class(bluetooth_class!("BluetoothGatt")).unwrap().into(),
                rc_env.find_class(bluetooth_class!("BluetoothGattDescriptor")).unwrap().into(),
                get_jvalue! {
                    Object,
                    callback.jenv.get_static_field(
                        "java/lang/Integer",
                        "TYPE",
                        "Ljava/lang/Class;"
                    )
                    .unwrap()
                }
            ],
            &[
                rc_env.alloc_object(bluetooth_class!("BluetoothGatt")).unwrap(),
                rc_env.alloc_object(bluetooth_class!("BluetoothGattDescriptor")).unwrap(),
                rc_env.new_object(
                    "java/lang/Integer",
                    "(I)V",
                    &[jni::objects::JValue::Int(BluetoothGatt::GATT_SUCCESS)]
                )
                .unwrap()
            ]
        );

        callback_test!(
            rc_env,
            callback,
            "onDescriptorRead",
            &[
                rc_env.find_class(bluetooth_class!("BluetoothGatt")).unwrap().into(),
                rc_env.find_class(bluetooth_class!("BluetoothGattDescriptor")).unwrap().into(),
                get_jvalue! {
                    Object,
                    callback.jenv.get_static_field(
                        "java/lang/Integer",
                        "TYPE",
                        "Ljava/lang/Class;"
                    )
                    .unwrap()
                }
            ],
            &[
                rc_env.alloc_object(bluetooth_class!("BluetoothGatt")).unwrap(),
                rc_env.alloc_object(bluetooth_class!("BluetoothGattDescriptor")).unwrap(),
                rc_env.new_object(
                    "java/lang/Integer",
                    "(I)V",
                    &[jni::objects::JValue::Int(-1)]
                )
                .unwrap()
            ]
        );

        assert!( flag1.load(Ordering::Relaxed) );
        assert!( flag2.load(Ordering::Relaxed) );
    }

    #[android_test]
    fn bluetooth_gatt_callback_set_on_descriptor_write(jenv: JNIEnv) {
        use android::ClassLoader;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        let rc_env = Rc::new(jenv);

        let flag1 = Arc::new(AtomicBool::new(false));
        let flag2 = Arc::new(AtomicBool::new(false));

        let flag1_clone = flag1.clone();
        let flag2_clone = flag2.clone();

        let callback = BluetoothGattCallbackBuilder::new()
            .set_on_descriptor_write(
                forget_gatt!( move |_,_| flag1_clone.store(true, Ordering::Relaxed) ),
                move || flag2_clone.store(true, Ordering::Relaxed) )
            .build(rc_env.clone(), ClassLoader::system(rc_env.as_ref()));

        callback_test!(
            rc_env,
            callback,
            "onDescriptorWrite",
            &[
                rc_env.find_class(bluetooth_class!("BluetoothGatt")).unwrap().into(),
                rc_env.find_class(bluetooth_class!("BluetoothGattDescriptor")).unwrap().into(),
                get_jvalue! {
                    Object,
                    callback.jenv.get_static_field(
                        "java/lang/Integer",
                        "TYPE",
                        "Ljava/lang/Class;"
                    )
                    .unwrap()
                }
            ],
            &[
                rc_env.alloc_object(bluetooth_class!("BluetoothGatt")).unwrap(),
                rc_env.alloc_object(bluetooth_class!("BluetoothGattDescriptor")).unwrap(),
                rc_env.new_object(
                    "java/lang/Integer",
                    "(I)V",
                    &[jni::objects::JValue::Int(BluetoothGatt::GATT_SUCCESS)]
                )
                .unwrap()
            ]
        );

        callback_test!(
            rc_env,
            callback,
            "onDescriptorWrite",
            &[
                rc_env.find_class(bluetooth_class!("BluetoothGatt")).unwrap().into(),
                rc_env.find_class(bluetooth_class!("BluetoothGattDescriptor")).unwrap().into(),
                get_jvalue! {
                    Object,
                    callback.jenv.get_static_field(
                        "java/lang/Integer",
                        "TYPE",
                        "Ljava/lang/Class;"
                    )
                    .unwrap()
                }
            ],
            &[
                rc_env.alloc_object(bluetooth_class!("BluetoothGatt")).unwrap(),
                rc_env.alloc_object(bluetooth_class!("BluetoothGattDescriptor")).unwrap(),
                rc_env.new_object(
                    "java/lang/Integer",
                    "(I)V",
                    &[jni::objects::JValue::Int(-1)]
                )
                .unwrap()
            ]
        );

        assert!( flag1.load(Ordering::Relaxed) );
        assert!( flag2.load(Ordering::Relaxed) );
    }

    #[android_test]
    fn bluetooth_gatt_callback_set_on_mtu_changed(jenv: JNIEnv) {
        use android::ClassLoader;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        let rc_env = Rc::new(jenv);

        let flag1 = Arc::new(AtomicBool::new(false));
        let flag2 = Arc::new(AtomicBool::new(false));

        let flag1_clone = flag1.clone();
        let flag2_clone = flag2.clone();

        let callback = BluetoothGattCallbackBuilder::new()
            .set_on_mtu_changed(
                forget_gatt!( move |_,_| flag1_clone.store(true, Ordering::Relaxed) ),
                move || flag2_clone.store(true, Ordering::Relaxed) )
            .build(rc_env.clone(), ClassLoader::system(rc_env.as_ref()));

        callback_test!(
            rc_env,
            callback,
            "onMtuChanged",
            &[
                rc_env.find_class(bluetooth_class!("BluetoothGatt")).unwrap().into(),
                get_jvalue! {
                    Object,
                    callback.jenv.get_static_field(
                        "java/lang/Integer",
                        "TYPE",
                        "Ljava/lang/Class;"
                    )
                    .unwrap()
                },
                get_jvalue! {
                    Object,
                    callback.jenv.get_static_field(
                        "java/lang/Integer",
                        "TYPE",
                        "Ljava/lang/Class;"
                    )
                    .unwrap()
                }
            ],
            &[
                rc_env.alloc_object(bluetooth_class!("BluetoothGatt")).unwrap(),
                rc_env.new_object(
                    "java/lang/Integer",
                    "(I)V",
                    &[jni::objects::JValue::Int(32)]
                )
                .unwrap(),
                rc_env.new_object(
                    "java/lang/Integer",
                    "(I)V",
                    &[jni::objects::JValue::Int(BluetoothGatt::GATT_SUCCESS)]
                )
                .unwrap()
            ]
        );

        callback_test!(
            rc_env,
            callback,
            "onMtuChanged",
            &[
                rc_env.find_class(bluetooth_class!("BluetoothGatt")).unwrap().into(),
                get_jvalue! {
                    Object,
                    callback.jenv.get_static_field(
                        "java/lang/Integer",
                        "TYPE",
                        "Ljava/lang/Class;"
                    )
                    .unwrap()
                },
                get_jvalue! {
                    Object,
                    callback.jenv.get_static_field(
                        "java/lang/Integer",
                        "TYPE",
                        "Ljava/lang/Class;"
                    )
                    .unwrap()
                }
            ],
            &[
                rc_env.alloc_object(bluetooth_class!("BluetoothGatt")).unwrap(),
                rc_env.new_object(
                    "java/lang/Integer",
                    "(I)V",
                    &[jni::objects::JValue::Int(32)]
                )
                .unwrap(),
                rc_env.new_object(
                    "java/lang/Integer",
                    "(I)V",
                    &[jni::objects::JValue::Int(-1)]
                )
                .unwrap()
            ]
        );

        assert!( flag1.load(Ordering::Relaxed) );
        assert!( flag2.load(Ordering::Relaxed) );
    }

    #[android_test]
    fn bluetooth_gatt_callback_set_on_phy_read(jenv: JNIEnv) {
        use android::ClassLoader;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        let rc_env = Rc::new(jenv);

        let flag1 = Arc::new(AtomicBool::new(false));
        let flag2 = Arc::new(AtomicBool::new(false));

        let flag1_clone = flag1.clone();
        let flag2_clone = flag2.clone();

        let callback = BluetoothGattCallbackBuilder::new()
            .set_on_phy_read(
                forget_gatt!( move |_,_,_| flag1_clone.store(true, Ordering::Relaxed) ),
                move || flag2_clone.store(true, Ordering::Relaxed) )
            .build(rc_env.clone(), ClassLoader::system(rc_env.as_ref()));

        callback_test!(
            rc_env,
            callback,
            "onPhyRead",
            &[
                rc_env.find_class(bluetooth_class!("BluetoothGatt")).unwrap().into(),
                get_jvalue! {
                    Object,
                    callback.jenv.get_static_field(
                        "java/lang/Integer",
                        "TYPE",
                        "Ljava/lang/Class;"
                    )
                    .unwrap()
                },
                get_jvalue! {
                    Object,
                    callback.jenv.get_static_field(
                        "java/lang/Integer",
                        "TYPE",
                        "Ljava/lang/Class;"
                    )
                    .unwrap()
                },
                get_jvalue! {
                    Object,
                    callback.jenv.get_static_field(
                        "java/lang/Integer",
                        "TYPE",
                        "Ljava/lang/Class;"
                    )
                    .unwrap()
                }
            ],
            &[
                rc_env.alloc_object(bluetooth_class!("BluetoothGatt")).unwrap(),
                rc_env.new_object(
                    "java/lang/Integer",
                    "(I)V",
                    &[jni::objects::JValue::Int(1)]
                )
                .unwrap(),
                rc_env.new_object(
                    "java/lang/Integer",
                    "(I)V",
                    &[jni::objects::JValue::Int(1)]
                )
                .unwrap(),
                rc_env.new_object(
                    "java/lang/Integer",
                    "(I)V",
                    &[jni::objects::JValue::Int(BluetoothGatt::GATT_SUCCESS)]
                )
                .unwrap()
            ]
        );

        callback_test!(
            rc_env,
            callback,
            "onPhyRead",
            &[
                rc_env.find_class(bluetooth_class!("BluetoothGatt")).unwrap().into(),
                get_jvalue! {
                    Object,
                    callback.jenv.get_static_field(
                        "java/lang/Integer",
                        "TYPE",
                        "Ljava/lang/Class;"
                    )
                    .unwrap()
                },
                get_jvalue! {
                    Object,
                    callback.jenv.get_static_field(
                        "java/lang/Integer",
                        "TYPE",
                        "Ljava/lang/Class;"
                    )
                    .unwrap()
                },
                get_jvalue! {
                    Object,
                    callback.jenv.get_static_field(
                        "java/lang/Integer",
                        "TYPE",
                        "Ljava/lang/Class;"
                    )
                    .unwrap()
                }
            ],
            &[
                rc_env.alloc_object(bluetooth_class!("BluetoothGatt")).unwrap(),
                rc_env.new_object(
                    "java/lang/Integer",
                    "(I)V",
                    &[jni::objects::JValue::Int(1)]
                )
                .unwrap(),
                rc_env.new_object(
                    "java/lang/Integer",
                    "(I)V",
                    &[jni::objects::JValue::Int(1)]
                )
                .unwrap(),
                rc_env.new_object(
                    "java/lang/Integer",
                    "(I)V",
                    &[jni::objects::JValue::Int(-1)]
                )
                .unwrap()
            ]
        );

        assert!( flag1.load(Ordering::Relaxed) );
        assert!( flag2.load(Ordering::Relaxed) );
    }

    #[android_test]
    fn bluetooth_gatt_callback_set_on_phy_update(jenv: JNIEnv) {
        use android::ClassLoader;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        let rc_env = Rc::new(jenv);

        let flag1 = Arc::new(AtomicBool::new(false));
        let flag2 = Arc::new(AtomicBool::new(false));

        let flag1_clone = flag1.clone();
        let flag2_clone = flag2.clone();

        let callback = BluetoothGattCallbackBuilder::new()
            .set_on_phy_update(
                forget_gatt!( move |_,_,_| flag1_clone.store(true, Ordering::Relaxed) ),
                move || flag2_clone.store(true, Ordering::Relaxed) )
            .build(rc_env.clone(), ClassLoader::system(rc_env.as_ref()));

        callback_test!(
            rc_env,
            callback,
            "onPhyUpdate",
            &[
                rc_env.find_class(bluetooth_class!("BluetoothGatt")).unwrap().into(),
                get_jvalue! {
                    Object,
                    callback.jenv.get_static_field(
                        "java/lang/Integer",
                        "TYPE",
                        "Ljava/lang/Class;"
                    )
                    .unwrap()
                },
                get_jvalue! {
                    Object,
                    callback.jenv.get_static_field(
                        "java/lang/Integer",
                        "TYPE",
                        "Ljava/lang/Class;"
                    )
                    .unwrap()
                },
                get_jvalue! {
                    Object,
                    callback.jenv.get_static_field(
                        "java/lang/Integer",
                        "TYPE",
                        "Ljava/lang/Class;"
                    )
                    .unwrap()
                }
            ],
            &[
                rc_env.alloc_object(bluetooth_class!("BluetoothGatt")).unwrap(),
                rc_env.new_object(
                    "java/lang/Integer",
                    "(I)V",
                    &[jni::objects::JValue::Int(1)]
                )
                .unwrap(),
                rc_env.new_object(
                    "java/lang/Integer",
                    "(I)V",
                    &[jni::objects::JValue::Int(1)]
                )
                .unwrap(),
                rc_env.new_object(
                    "java/lang/Integer",
                    "(I)V",
                    &[jni::objects::JValue::Int(BluetoothGatt::GATT_SUCCESS)]
                )
                .unwrap()
            ]
        );

        callback_test!(
            rc_env,
            callback,
            "onPhyUpdate",
            &[
                rc_env.find_class(bluetooth_class!("BluetoothGatt")).unwrap().into(),
                get_jvalue! {
                    Object,
                    callback.jenv.get_static_field(
                        "java/lang/Integer",
                        "TYPE",
                        "Ljava/lang/Class;"
                    )
                    .unwrap()
                },
                get_jvalue! {
                    Object,
                    callback.jenv.get_static_field(
                        "java/lang/Integer",
                        "TYPE",
                        "Ljava/lang/Class;"
                    )
                    .unwrap()
                },
                get_jvalue! {
                    Object,
                    callback.jenv.get_static_field(
                        "java/lang/Integer",
                        "TYPE",
                        "Ljava/lang/Class;"
                    )
                    .unwrap()
                }
            ],
            &[
                rc_env.alloc_object(bluetooth_class!("BluetoothGatt")).unwrap(),
                rc_env.new_object(
                    "java/lang/Integer",
                    "(I)V",
                    &[jni::objects::JValue::Int(1)]
                )
                .unwrap(),
                rc_env.new_object(
                    "java/lang/Integer",
                    "(I)V",
                    &[jni::objects::JValue::Int(1)]
                )
                .unwrap(),
                rc_env.new_object(
                    "java/lang/Integer",
                    "(I)V",
                    &[jni::objects::JValue::Int(-1)]
                )
                .unwrap()
            ]
        );

        assert!( flag1.load(Ordering::Relaxed) );
        assert!( flag2.load(Ordering::Relaxed) );
    }

    #[android_test]
    fn bluetooth_gatt_callback_set_on_read_remote_rssi(jenv: JNIEnv) {
        use android::ClassLoader;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        let rc_env = Rc::new(jenv);

        let flag1 = Arc::new(AtomicBool::new(false));
        let flag2 = Arc::new(AtomicBool::new(false));

        let flag1_clone = flag1.clone();
        let flag2_clone = flag2.clone();

        let callback = BluetoothGattCallbackBuilder::new()
            .set_on_read_remote_rssi(
                forget_gatt!( move |_,_| flag1_clone.store(true, Ordering::Relaxed) ),
                move || flag2_clone.store(true, Ordering::Relaxed) )
            .build(rc_env.clone(), ClassLoader::system(rc_env.as_ref()));

        callback_test!(
            rc_env,
            callback,
            "onReadRemoteRssi",
            &[
                rc_env.find_class(bluetooth_class!("BluetoothGatt")).unwrap().into(),
                get_jvalue! {
                    Object,
                    callback.jenv.get_static_field(
                        "java/lang/Integer",
                        "TYPE",
                        "Ljava/lang/Class;"
                    )
                    .unwrap()
                },
                get_jvalue! {
                    Object,
                    callback.jenv.get_static_field(
                        "java/lang/Integer",
                        "TYPE",
                        "Ljava/lang/Class;"
                    )
                    .unwrap()
                }
            ],
            &[
                rc_env.alloc_object(bluetooth_class!("BluetoothGatt")).unwrap(),
                rc_env.new_object(
                    "java/lang/Integer",
                    "(I)V",
                    &[jni::objects::JValue::Int(-11)]
                )
                .unwrap(),
                rc_env.new_object(
                    "java/lang/Integer",
                    "(I)V",
                    &[jni::objects::JValue::Int(BluetoothGatt::GATT_SUCCESS)]
                )
                .unwrap()
            ]
        );

        callback_test!(
            rc_env,
            callback,
            "onReadRemoteRssi",
            &[
                rc_env.find_class(bluetooth_class!("BluetoothGatt")).unwrap().into(),
                get_jvalue! {
                    Object,
                    callback.jenv.get_static_field(
                        "java/lang/Integer",
                        "TYPE",
                        "Ljava/lang/Class;"
                    )
                    .unwrap()
                },
                get_jvalue! {
                    Object,
                    callback.jenv.get_static_field(
                        "java/lang/Integer",
                        "TYPE",
                        "Ljava/lang/Class;"
                    )
                    .unwrap()
                }
            ],
            &[
                rc_env.alloc_object(bluetooth_class!("BluetoothGatt")).unwrap(),
                rc_env.new_object(
                    "java/lang/Integer",
                    "(I)V",
                    &[jni::objects::JValue::Int(-11)]
                )
                .unwrap(),
                rc_env.new_object(
                    "java/lang/Integer",
                    "(I)V",
                    &[jni::objects::JValue::Int(-1)]
                )
                .unwrap()
            ]
        );

        assert!( flag1.load(Ordering::Relaxed) );
        assert!( flag2.load(Ordering::Relaxed) );
    }

    #[android_test]
    fn bluetooth_gatt_callback_set_on_reliable_write_completed(jenv: JNIEnv) {
        use android::ClassLoader;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        let rc_env = Rc::new(jenv);

        let flag1 = Arc::new(AtomicBool::new(false));
        let flag2 = Arc::new(AtomicBool::new(false));

        let flag1_clone = flag1.clone();
        let flag2_clone = flag2.clone();

        let callback = BluetoothGattCallbackBuilder::new()
            .set_on_reliable_write_completed(
                forget_gatt!( move |_| flag1_clone.store(true, Ordering::Relaxed) ),
                move || flag2_clone.store(true, Ordering::Relaxed) )
            .build(rc_env.clone(), ClassLoader::system(rc_env.as_ref()));

        callback_test!(
            rc_env,
            callback,
            "onReliableWriteCompleted",
            &[
                rc_env.find_class(bluetooth_class!("BluetoothGatt")).unwrap().into(),
                get_jvalue! {
                    Object,
                    callback.jenv.get_static_field(
                        "java/lang/Integer",
                        "TYPE",
                        "Ljava/lang/Class;"
                    )
                    .unwrap()
                },
            ],
            &[
                rc_env.alloc_object(bluetooth_class!("BluetoothGatt")).unwrap(),
                rc_env.new_object(
                    "java/lang/Integer",
                    "(I)V",
                    &[jni::objects::JValue::Int(BluetoothGatt::GATT_SUCCESS)]
                )
                .unwrap()
            ]
        );

        callback_test!(
            rc_env,
            callback,
            "onReliableWriteCompleted",
            &[
                rc_env.find_class(bluetooth_class!("BluetoothGatt")).unwrap().into(),
                get_jvalue! {
                    Object,
                    callback.jenv.get_static_field(
                        "java/lang/Integer",
                        "TYPE",
                        "Ljava/lang/Class;"
                    )
                    .unwrap()
                }
            ],
            &[
                rc_env.alloc_object(bluetooth_class!("BluetoothGatt")).unwrap(),
                rc_env.new_object(
                    "java/lang/Integer",
                    "(I)V",
                    &[jni::objects::JValue::Int(-1)]
                )
                .unwrap()
            ]
        );

        assert!( flag1.load(Ordering::Relaxed) );
        assert!( flag2.load(Ordering::Relaxed) );
    }

    #[android_test]
    fn bluetooth_gatt_callback_set_on_services_discovered(jenv: JNIEnv) {
        use android::ClassLoader;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        let rc_env = Rc::new(jenv);

        let flag1 = Arc::new(AtomicBool::new(false));
        let flag2 = Arc::new(AtomicBool::new(false));

        let flag1_clone = flag1.clone();
        let flag2_clone = flag2.clone();

        let callback = BluetoothGattCallbackBuilder::new()
            .set_on_services_discovered(
                forget_gatt!( move |_| flag1_clone.store(true, Ordering::Relaxed) ),
                move || flag2_clone.store(true, Ordering::Relaxed) )
            .build(rc_env.clone(), ClassLoader::system(rc_env.as_ref()));

        callback_test!(
            rc_env,
            callback,
            "onServicesDiscovered",
            &[
                rc_env.find_class(bluetooth_class!("BluetoothGatt")).unwrap().into(),
                get_jvalue! {
                    Object,
                    callback.jenv.get_static_field(
                        "java/lang/Integer",
                        "TYPE",
                        "Ljava/lang/Class;"
                    )
                    .unwrap()
                },
            ],
            &[
                rc_env.alloc_object(bluetooth_class!("BluetoothGatt")).unwrap(),
                rc_env.new_object(
                    "java/lang/Integer",
                    "(I)V",
                    &[jni::objects::JValue::Int(BluetoothGatt::GATT_SUCCESS)]
                )
                .unwrap()
            ]
        );

        callback_test!(
            rc_env,
            callback,
            "onServicesDiscovered",
            &[
                rc_env.find_class(bluetooth_class!("BluetoothGatt")).unwrap().into(),
                get_jvalue! {
                    Object,
                    callback.jenv.get_static_field(
                        "java/lang/Integer",
                        "TYPE",
                        "Ljava/lang/Class;"
                    )
                    .unwrap()
                }
            ],
            &[
                rc_env.alloc_object(bluetooth_class!("BluetoothGatt")).unwrap(),
                rc_env.new_object(
                    "java/lang/Integer",
                    "(I)V",
                    &[jni::objects::JValue::Int(-1)]
                )
                .unwrap()
            ]
        );

        assert!( flag1.load(Ordering::Relaxed) );
        assert!( flag2.load(Ordering::Relaxed) );
    }

    #[android_test]
    fn construct_bluetooth_gatt_descriptor(jenv: JNIEnv) {
        let test_uuid = ::UUID::from(0x10u32);

        let permissions = [Permissions::Read, Permissions::Write, Permissions::WriteEncryptedMITM];

        BluetoothGattDescriptor::new(Rc::new(jenv), test_uuid, &permissions);
    }

    #[android_test]
    fn bluetooth_gatt_descriptor_get_permissions(jenv: JNIEnv) {
        let test_uuid = ::UUID::from(0x10u32);

        let permissions = [Permissions::Read, Permissions::Write, Permissions::WriteEncryptedMITM];

        let desc = BluetoothGattDescriptor::new(Rc::new(jenv), test_uuid, &permissions);

        assert_eq!(desc.get_permissions(), permissions.to_vec().into_boxed_slice());
    }

    #[android_test]
    fn bluetooth_gatt_descriptor_get_uuid(jenv: JNIEnv) {
        let test_uuid = ::UUID::from(0x10u32);

        let permissions = [Permissions::Read, Permissions::Write, Permissions::WriteEncryptedMITM];

        let desc = BluetoothGattDescriptor::new(Rc::new(jenv), test_uuid, &permissions);

        assert_eq!(desc.get_uuid(), test_uuid);
    }

    #[android_test]
    fn bluetooth_gatt_descriptor_get_set_value(jenv: JNIEnv) {
        let test_uuid = ::UUID::from(0x10u32);

        let permissions = [Permissions::Read, Permissions::Write, Permissions::WriteEncryptedMITM];

        let desc = BluetoothGattDescriptor::new(Rc::new(jenv), test_uuid, &permissions);

        let test_vals = [0x1, 0x2, 0x3, 0x4, 0x5];

        desc.set_value(&test_vals);

        assert_eq!( test_vals.to_vec().into_boxed_slice(), desc.get_value() );
    }

    #[android_test]
    fn construct_bluetooth_gatt_service(jenv: JNIEnv) {
        let test_uuid = ::UUID::from(0x12u32);

        BluetoothGattService::new(Rc::new(jenv), test_uuid, true);
    }

    #[android_test]
    fn create_bluetooth_gatt_caracteristic(jenv: JNIEnv) {
        let rc_env = Rc::new(jenv);

        // All setup data is arbitrary

        let uuid = ::UUID::from(0x12345u32);

        let properties = [
            CharacteristicProperties::Read,
            CharacteristicProperties::WriteWithoutResponse,
            CharacteristicProperties::Broadcast,
        ];

        let permissions = [
            Permissions::Write,
            Permissions::Read,
        ];

        BluetoothGattCharacteristic::new(rc_env, uuid, &properties, &permissions);
    }

    #[android_test]
    fn bluetooth_gatt_characteristic_get_permissions_and_properties_and_uuid(jenv: JNIEnv) {
        let rc_env = Rc::new(jenv);

        let uuid = ::UUID::from(0x12345u128);

        let properties = [
            CharacteristicProperties::Read,
            CharacteristicProperties::Write,
            CharacteristicProperties::Indicate
        ];

        let permissions = [
            Permissions::Read,
            Permissions::Write,
            Permissions::WriteSignedMITM
        ];

        let characteristic = BluetoothGattCharacteristic::new(rc_env, uuid, &properties, &permissions);

        assert_eq!(permissions.to_vec().into_boxed_slice(), characteristic.get_permissions());

        assert_eq!(properties.to_vec().into_boxed_slice(), characteristic.get_properties());

        assert_eq!(uuid, characteristic.get_uuid());
    }

    #[android_test]
    fn bluetooth_gatt_characteristic_add_and_get_descriptor(jenv: JNIEnv) {
        let rc_env = Rc::new(jenv);

        let c_uuid = ::UUID::from(0x12345u32);
        let d_uuid = ::UUID::from(0x54321u32);

        let properties = [
            CharacteristicProperties::Read,
            CharacteristicProperties::Write,
            CharacteristicProperties::Indicate
        ];

        let permissions = [
            Permissions::Read,
            Permissions::Write,
            Permissions::WriteSignedMITM
        ];

        let characteristic = BluetoothGattCharacteristic::new(rc_env.clone(), c_uuid, &properties, &permissions);

        characteristic.add_descriptor(BluetoothGattDescriptor::new(rc_env.clone(), d_uuid, &permissions));

        assert!( characteristic.get_descriptor(d_uuid).is_some() );

        assert!( characteristic.get_descriptor(::UUID::from(0u16)).is_none() );
    }

    #[android_test]
    fn create_bluetooth_gatt_service(jenv: JNIEnv) {
        let rc_env = Rc::new(jenv);

        let uuid = ::UUID::from(0x1234567890u128);

        BluetoothGattService::new(rc_env, uuid, true);
    }

    #[android_test]
    fn bluetooth_gatt_service_is_primary(jenv: JNIEnv) {
        let rc_env = Rc::new(jenv);

        let bs1 = BluetoothGattService::new(rc_env.clone(), ::UUID::from(0u128), true);
        let bs2 = BluetoothGattService::new(rc_env.clone(), ::UUID::from(1u128), false);

        assert!( bs1.is_primary() );

        assert!( !bs2.is_primary() );
    }

    #[android_test]
    fn bluetooth_gatt_service_add_and_get_characteristic(jenv: JNIEnv) {
        let rc_env = Rc::new(jenv);

        let characteristic_uuid = ::UUID::from(0x1111111u128);

        let service_uuid = ::UUID::from(0x222222u128);

        let properties = [ CharacteristicProperties::Read ];

        let permissions = [ Permissions::Read ];

        let characteristic = BluetoothGattCharacteristic::new(
            rc_env.clone(),
            characteristic_uuid,
            &properties,
            &permissions
        );

        let service = BluetoothGattService::new(rc_env.clone(), service_uuid, true);

        service.add_characteristic(characteristic);

        assert!(service.get_characteristic(characteristic_uuid).is_some())
    }

    #[android_test]
    fn bluetooth_gatt_service_get_characteristics(jenv: JNIEnv) {
        let rc_env = Rc::new(jenv);

        let characteristic_uuids = [
            ::UUID::from(0x123u128),
            ::UUID::from(0x234u128),
            ::UUID::from(0x345u128),
            ::UUID::from(0x456u128),
        ];

        let bluetooth_service = BluetoothGattService::new(rc_env.clone(), ::UUID::from(1u128), true);

        characteristic_uuids.iter().for_each( |uuid| {
            let permissions = [ Permissions::Read ];
            let properties  = [ CharacteristicProperties::Read ];

            let characteristic = BluetoothGattCharacteristic::new(
                rc_env.clone(),
                *uuid,
                &properties,
                &permissions
            );

            bluetooth_service.add_characteristic(characteristic);
        });

        let retrieved_characteristics_uuids = bluetooth_service.get_characteristics().into_iter().map(
            | characteristic | characteristic.get_uuid()
        )
        .collect::<Vec<_>>();

        for uuid in characteristic_uuids.iter() {
            assert!( retrieved_characteristics_uuids.contains(uuid) );
        }
    }

    #[android_test]
    fn bluetooth_gat_service_add_and_get_included_services(jenv: JNIEnv) {
        let rc_env = Rc::new(jenv);

        let main_service = BluetoothGattService::new(rc_env.clone(), ::UUID::from(1u128), true);

        let included_service_uuids = [
            ::UUID::from(2u128),
            ::UUID::from(3u128),
            ::UUID::from(4u128),
            ::UUID::from(5u128),
        ];

        for uuid in included_service_uuids.iter() {
            main_service.include_service(BluetoothGattService::new(rc_env.clone(), *uuid, true));
        }

        let included_services = main_service.get_included_services().iter()
            .map(|service| service.get_uuid())
            .collect::<Vec<_>>();

        for uuid in included_service_uuids.iter() {
            assert!( included_services.contains(uuid) );
        }
    }
}
