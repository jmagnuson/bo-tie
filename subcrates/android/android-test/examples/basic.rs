//! Build this example with the command `GEN_JAVA_FILE_PATH=temp LIBRARY_FILE_NAME=basic cargo
//! rustc --example basic -- -Zunstable-options --pretty=expanded` to see how this crate works.
//! This is more usefull then running
//!
//! Run this example with the command `GEN_JAVA_FILE_PATH=temp LIBRARY_FILE_NAME=basic cargo run
//! --example basic`

extern crate android_test;
extern crate jni;

use android_test::android_test;

#[android_test(
    // Permissions names are not checked and assumed to be valid which is why FAKE_PERMISSION and
    // DOES_NOT_EXIST do not cause a compile time error.
    permissions = ( "READ_PHONE_STATE", "FAKE_PERMISSION", "DOES_NOT_EXIST" )
)]
fn my_basic_function() {
    println!("It works!");
    println!("Don't forget to also build with pretty=expanded to show how this crate works \
        'GEN_JAVA_FILE_PATH=temp LIBRARY_FILE_NAME=basic cargo rustc --example basic -- \
        -Zunstable-options --pretty=expanded'");
}

fn main() {
    Java_InstrumentTests_myBasicFunction(
        unsafe { std::mem::uninitialized() },
        unsafe { std::mem::uninitialized() }
    );
}
