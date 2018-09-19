//! This is the builder for the bluez library
//!
//! It is a temporary requirement, but for now all platforms must generate bindings to the bluez
//! header files listed in build/wrapper.h. For Unix/GNU toolchains this should be trivial, but on
//! other toolchains it will not be because of unix system header files required. BINDGEN_CLANG_FLAGS
//! is the enviroment variable to set to specify the include path to the unix header files
//! required.
//!
//! # Windows Configuration
//! On windows if BINDGEN_CLANG_FLAGS is not specified, then it is assumed that cygwin is installed
//! with the required development packages to include the nessicary unix header files at
//! `C:\cygwin64`.

extern crate bindgen;

use std::env;

static BINDGEN_HEADER_FILE: &'static str = "build/wrapper.h";
static BINDGEN_OUTPUT_FILE: &'static str = "build/generated_bindings/bluetooth.rs";
static REQUIRED_CLANG_FLAGS: &'static [&str] = &["-Ibluez"];

fn main() {
    println!("cargo:rerun-if-changed=build");

    let bindgen_clang_flags = env::var_os("BINDGEN_CLANG_FLAGS");

    let flags = if let Some(val) = bindgen_clang_flags {
        val.into_string().expect("BINDGEN_CLANG_FLAGS contains non-Unicode characters")
    }
    else  {
        String::from(concat!("-I", r#"C:\cygwin64\usr\include"#))
    };

    let bindgen_bindings = bindgen::builder()
        .header(BINDGEN_HEADER_FILE)
        .clang_args(REQUIRED_CLANG_FLAGS)
        .clang_arg(flags)
        .derive_partialeq(true)
        .derive_debug(false)
        .derive_default(true)
        .derive_copy(true)
        .generate_comments(false)
        .rustfmt_bindings(true)
        .layout_tests(false)
        .generate()
        .unwrap();

    bindgen_bindings
        .write_to_file(BINDGEN_OUTPUT_FILE)
        .expect("Couldn't write to file");
}
