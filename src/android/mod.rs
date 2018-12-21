use self::wrapper::*;
use std::rc::Rc;
use std::collections::HashSet;

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

mod wrapper;

pub struct AndroidBluetooth<'a> {
    env: Rc<jni::JNIEnv<'a>>,
    adapter: BluetoothAdapter<'a>,
    devices: HashSet<BluetoothDevice<'a>>
}

impl<'a> AndroidBluetooth<'a> {
    pub fn new( jenv: jni::JNIEnv<'a>, context: jni::objects::JObject<'a> ) -> Self {
        let rcjenv = Rc::new(jenv);

        let bluetooth_manager = BluetoothManager::<'a>::new(rcjenv.clone(), context);

        AndroidBluetooth {
            env: rcjenv.clone(),
            adapter: bluetooth_manager.get_adapter(),
            devices: HashSet::new(),
        }
    }
}

#[cfg(feature = "android_test")]
mod tests {
    extern crate android_test;

    use jni::JNIEnv;
    use self::android_test::android_test;

    #[android_test]
    fn panic_test( _: JNIEnv ) {
        panic!("What did you expect this test to do?")
    }
}
