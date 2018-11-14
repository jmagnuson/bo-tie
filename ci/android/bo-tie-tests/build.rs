extern crate bindgen;

use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::path::{Path,PathBuf};
use std::process::Command;

static JAVA_FILE: &'static str = "src/java/Test.java";

static JAVA_INCLUDE: &'static str = "/usr/lib/jvm/java-8-openjdk-amd64/include";

static BINDINGS_FILE: &'static str = "src/jni_gen.rs";

#[cfg(target_os = "linux")]
static MD_PATH: &'static str = "/linux";

#[cfg(target_os = "windows")]
static MD_PATH: &'static str = "/win*";

#[cfg(target_os = "macos")]
static MD_PATH: &'static str = "/macos"; // I don't think this is right, might be darwin

fn main() {

    println!("cargo:rerun-if-changed=src/java/Test.java");

    let output_path = env::var("OUT_DIR").unwrap();

    let jni_inc_path: String = match env::var_os("JNI_INCLUDE") {
        Some(path) => path.into_string().unwrap(),
        None => if Path::new(JAVA_INCLUDE).exists() {
            String::from(JAVA_INCLUDE)
        } else {
            panic!("The include path to the jni header file is not defined (build.rs default \
                location is {}). Set the env 'JNI_INCLUDE' to the include path for jni.h");
        }
    };

    let bindgen_header_path = [output_path.as_str(), "bindings.h"].iter().collect::<PathBuf>();

    let jni_file_path = [output_path.as_str(),"Test.h"].iter().collect::<PathBuf>();

    let mut bindgen_header_file = File::create(bindgen_header_path.clone()).unwrap();

    bindgen_header_file.set_len(0).unwrap();
    write!(bindgen_header_file, r#"#include "{}""#, jni_file_path.display() ).unwrap();
    write!(bindgen_header_file, "\n").unwrap();
    bindgen_header_file.flush().unwrap();

    Command::new("javac")
        .args(&["-d", output_path.as_str()])
        .args(&["-h", output_path.as_str()])
        .arg(JAVA_FILE)
        .output()
        .expect("Java JDK not installed (uses Java v1.8)");

    let generation = bindgen::Builder::default()
                     .header(bindgen_header_path.to_str().unwrap())
                     .generate_comments(true)
                     .layout_tests(false)
                     .whitelist_recursively(false)
                     .whitelist_function(".*testInit")
                     .whitelist_function(".*runTests")
                     .clang_arg(format!("{}{}", "-I", jni_inc_path))
                     .clang_arg(format!("{}{}{}", "-I", jni_inc_path, MD_PATH))
                     .generate()
                     .expect("Couldn't create binding generation");

    let mut bindings_file = File::create(BINDINGS_FILE).expect("Couldn't create bindings file");

    let mut bindings = generation.to_string()
        .split("\n")
        .map(|line| {
            let mut new_line = String::from("    ");
            new_line.push_str(line);
            new_line.push('\n');
            new_line
        })
        .collect::<String>();

    let extern_c_pat = r#"extern "C" {"#;

    while let Some(pos) = bindings.find(extern_c_pat) {

        let next_fn_kwrd = bindings[pos..].find("fn").unwrap() + pos;

        bindings.replace_range(pos..next_fn_kwrd, "#[no_mangle]\n    ");

        let next_semi_colon = bindings[pos..].find(";").unwrap() + pos;

        let next_end_line_1 = bindings[next_semi_colon..].find("\n").unwrap() + next_semi_colon;

        let next_close_bracket = bindings[next_end_line_1..].find("}").unwrap() + next_end_line_1;

        let next_end_line_2 = bindings[next_close_bracket..].find("\n").unwrap() + next_close_bracket;

        bindings.replace_range((next_end_line_1 + 1)..=(next_end_line_2), "\n");
    }

    bindings_file.set_len(0).unwrap();
    bindings_file.write_all(b"use jni::JNIEnv;\n").unwrap();
    bindings_file.write_all(b"use jni::objects::JClass as jclass;\n").unwrap();
    bindings_file.write_all(b"pub trait TestJNI {\n").unwrap();
    bindings_file.write_all(bindings.as_bytes()).unwrap();
    bindings_file.write_all(b"}\n").unwrap();
}
