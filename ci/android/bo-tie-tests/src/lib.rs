extern crate jni;
extern crate bo_tie;

mod jni_gen;

use jni::JNIEnv;
use jni::objects;

struct Tests;

impl jni_gen::TestJNI for Tests {
    #[no_mangle]
    fn Java_Test_testInit(env: *mut JNIEnv, class: objects::JClass ) {
    }

    #[no_mangle]
    fn Java_Test_runTests(env: *mut JNIEnv, class: objects::JClass ) {
    }
}
