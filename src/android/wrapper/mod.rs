//! wrapper contains structures that wrap around the raw jni objects
//!
//! Every Struct in jni_wrapper carries the JNIEnv variable along with the jobject variable so
//! that the lifetime of the JNIEnv

use self::advertise::*;

use std::cmp::{
    Eq,
    PartialEq,
};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

pub mod advertise;

/// Bluetooth device identifier
#[derive(Clone)]
pub struct BluetoothDevice<'a> {
    jenv: Rc<jni::JNIEnv<'a>>,
    object: jni::sys::jobject,
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
    pub fn get_bluetooth_le_advertisier(&'a self) -> BluetoothLEAdvertiser<'a> {
        let object = get_jvalue!( Object,
            self.jenv.call_method(
                jni::objects::JObject::from(self.object),
                "getBluetoothLEAdvertiser",
                format!("()L{};", bluetooth_le_class!("BluetoothLeAdvertiser")),
                &[])
            .unwrap());

        BluetoothLEAdvertiser::new(self.jenv.clone(), object)
    }
}

#[cfg(test)]
mod test_util {

    #[derive(Debug)]
    pub enum JNIEnvArgError {
        MissingArgument,
        ArgumentNotUtf8,
        ArgumentNotParsableToUsize,
        ArgumentIsNull,
    }

    pub fn get_jni_env<'a>() -> Result<jni::JNIEnv<'a>,JNIEnvArgError> {
        use std::env;

        let args = env::args_os().collect::<Vec<_>>();

        let jenv_ptr = args.get(2).ok_or(JNIEnvArgError::MissingArgument)?
            .to_str().ok_or(JNIEnvArgError::ArgumentNotUtf8)?
            .parse::<usize>().or(Err(JNIEnvArgError::ArgumentNotParsableToUsize))?
            as *mut jni::sys::JNIEnv;

        Ok(unsafe { jni::JNIEnv::from_raw(jenv_ptr).or(Err(JNIEnvArgError::ArgumentIsNull))? })
    }
}
