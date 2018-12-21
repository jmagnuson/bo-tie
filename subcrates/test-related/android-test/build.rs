fn main() {
    println!("cargo:rustc-env=JAVA_PACKAGE={}",
        match ::std::env::var_os("JAVA_PACKAGE") {
            Some(val) => val,
            None => ::std::ffi::OsString::from(""),
        }
        .into_string()
        .expect("Couldn't get value of env JAVA_PACKAGE")
    );

    println!("cargo:rustc-env=JAVA_OUTPUT_PATH={}",
        match ::std::env::var_os("GEN_JAVA_FILE_PATH") {
            Some(val) => val.into_string().expect("Couldnt' get value of env GEN_JAVA_FILE_PATH"),
            None => panic!("To use android-test, the enviroment variable 'GEN_JAVA_FILE_PATH' must \
            be set, this is the path for where the generated java unit test file will be placed"),
        }
    );
}
