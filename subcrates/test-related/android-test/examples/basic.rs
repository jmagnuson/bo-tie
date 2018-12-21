extern crate android_test;
extern crate jni;

use android_test::android_test;
use jni::JNIEnv;

#[android_test]
#[allow(dead_code)]
fn my_basic_function( _env: JNIEnv ) {}

fn main() {
    println!("Basic Example");
}
