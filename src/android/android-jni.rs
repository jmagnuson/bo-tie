use jni;

pub struct AndroidBluetooth {
    env: jni::JNIEnv;
}

impl AndroidBluetooth {
    pub fn new( jenv: jni::JNIEnv ) -> Self {
        AndroidBluetooth {
            env: jenv
        }
    }
}
 
