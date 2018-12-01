extern crate jni;
extern crate bo_tie;

mod jni_gen;

use jni::JNIEnv;
use jni::objects;
use jni::sys::jstring;

#[allow(dead_code)]
struct Tests;

impl jni_gen::TestJNI for Tests {
    #[no_mangle]
    extern "system" fn Java_botie_testproject_Interface_runTests(jenv: JNIEnv, _: objects::JClass) -> jstring {
        // let mut config = compiletest::Config::default();
        //
        // // Everything should pass
        // config.mode = "run-pass".parse.unwrap();
        // config.src_base = PathBuf::from();
        // config.link_deps();
        // config.clean_rmeta();
        //
        // compiletest::run_tests(&config);

        jenv.new_string("Hellow World".to_string()).unwrap().into_inner()
    }
}
