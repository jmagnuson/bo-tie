extern crate jni;

use jni::JNIEnv;
use jni::objects::{JClass, JString};
use jni::sys::jstring;

use std::process::Command;

#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn Java_bo_1tie_BoTieTester_MainActivity_runRustTests(
    env: JNIEnv, _: JClass, tests_file_path: JString) -> jstring
{
    let file_path: String = env.get_string(input).expect("Couldn't input as string").into();

    
}
