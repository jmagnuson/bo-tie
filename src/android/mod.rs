use jni;

pub struct AndroidBluetooth {
    env: jni::JNIEnv;
    adapter: jni::objects::JClass;
}

impl AndroidBluetooth {
    pub fn new( jenv: jni::JNIEnv ) -> Self {
        AndroidBluetooth {
            env: jenv,
            adapter: jenv.find_class("android/bluetooth/BluetoothAdapter"),
        }
    }
}
