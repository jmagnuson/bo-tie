//! classbytes is a crate for generating dalvik formatted bytecode from a java class
//!
//! # Output
//! classbytes generates bo-tie's source file classbytes.rs. There are two ways to generate the
//! output, but the best way is to install docker
//!
//! ## Generate classbytes.rs with docker
//! If you don't have docker (or don't know what it is) you will need to at install docker ce.
//! [Click here for official installation instructions](https://docs.docker.com/install/)
//!
//! Once you have docker (or if you had it already), run the script run-docker.sh in the ci folder.
//! If all goes well then classbytes.rs has been re-generated.
//!
//! ## Generate classbytes.rs the longer way
//! To generate classbytes.rs, this crate needs to utilized the android sdk and some installable
//! packages.
//!
//! Download the [command line tools](https://developer.android.com/studio/#command-tools)
//! for the android sdk and unzip it to a perminant location. You will also need some SDK packages
//! also so next step is to go use the sdkmanager to get the packages 'platforms;android-28' and
//! 'build-tools;28.0.3' (the latest version of build-tools can be used, but right now
//! `classbytes` requires that the platform be for android version 28). sdkmanager can
//! be found from the android sdk folder in tools/bin. Run `sdkmanager --licenses` and
//! agree to Google's licensing, then run `sdkmanager 'platforms;android-28' 'build-tools;28.0.3'`
//! to get the needed packages.
//!
//! All that is left is to run the crate which unfortunately requires that the environment variable
//! ANDROID_SDK_PATH be defined as the path to the android sdk.
//!
//! # Configuration
//! The class information is taken from JavaFiles.toml in the root of the crate. It only has one
//! table array `classes`. Each entry in the array needs to contain 3 things, a fully qualified
//! name, the path to the package containing the class, and the name of the constant as it will
//! appear in the generated file.
//!
//! ## classes
//! The name of the table array of class information
//!
//! ### fully_qualified_name
//! This is the key for the fully qualified name of the Class. This name is in folder notation
//! (eg. my/package/MyClass) without any extenstion
//!
//! ### path_to_package
//! The full path from the **bo-tie crate root** to the package (or the java file if class is not
//! in any package) containing the class
//!
//! ### name_of_constant
//! The name of the bytecode array generated in classbytes.rs

use std::process::Command;
use std::path::{Path, PathBuf};

const GENERATED_FILE_LOCATION: &'static str = "temp";

#[derive(serde_derive::Deserialize, Clone)]
struct JavaFiles {
    classes: Vec<Class>
}

impl std::iter::IntoIterator for JavaFiles {
    type Item = Class;
    type IntoIter = std::vec::IntoIter<Class>;

    fn into_iter(self) -> Self::IntoIter {
        self.classes.into_iter()
    }
}

#[derive(serde_derive::Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Class {
    pub fully_qualified_name: String,
    pub path_to_package: String,
    pub name_of_constant: String,
}

impl std::default::Default for Class {
    fn default() -> Self {
        Class {
            fully_qualified_name: "".into(),
            path_to_package: "".into(),
            name_of_constant: "".into(),
        }
    }
}

/// Compile the java files
///
/// This will return a vector of Class structures with a string of the java code extracted from
/// The file. However the first line is a comment containing the file path of the java file.
fn build_java_files( android_sdk: &str, sdk_version: usize, classes: JavaFiles ) -> Vec<(Class, String)>
{
    std::fs::create_dir_all(GENERATED_FILE_LOCATION).unwrap();

    let (mut file_contents, class_paths): (Vec<(_,_)>, Vec<_>) = classes.into_iter()
    .map(|class| {
        let bo_tie_path = [
                class.path_to_package.clone(),
                class.fully_qualified_name.clone() + ".java"
            ]
            .iter()
            .collect::<PathBuf>();

        let path = [ Path::new("../../../"), &bo_tie_path].iter().collect::<PathBuf>();

        // The contents of the java file. This is added as documentation of the generated constant.
        let java_contents =
            "/// Java bytecode of ".to_string() +
            &class.fully_qualified_name +
            "\n///" +
            "\n/// # File Path " +
            "\n/// " +
            bo_tie_path.to_str().expect("Couldn't read path") +
            "\n///" +
            "\n/// # Java Code" +
            "\n///```java\n/// " +
            &std::fs::read_to_string(&path).expect("Couldn't read file").replace("\n", "\n/// ") +
            "\n///```\n";

        (( class, java_contents), path)
    })
    .unzip();

    let jar_file = format!("platforms/android-{}/android.jar", sdk_version);

    let output = Command::new("javac")
        .arg("-classpath")
        .arg([android_sdk, &jar_file].iter().collect::<PathBuf>())
        .args(&[
            "-d", GENERATED_FILE_LOCATION,
            "-s", GENERATED_FILE_LOCATION,
            "-h", GENERATED_FILE_LOCATION
            ]
        )
        .args(&class_paths.iter().map(|path| path.into()).collect::<Vec<PathBuf>>())
        .output()
        .expect("failed to execute javac");

    if !output.status.success() {
        panic!("Couldn't compile javac: {}",
            String::from_utf8(output.stderr).expect("Couldn't get stderr").replace("\\n", "\n")
        );
    }

    // This is to add as documentation the requrired function signatures for native methods (in "c"
    // a.k.a. unmangled) to the documentation of the generated constant.
    for (class, doc_contents) in &mut file_contents {
        let h_name = class.fully_qualified_name.replace("/", "_") + ".h";

        let h_path = [GENERATED_FILE_LOCATION.to_string(), h_name].iter().collect::<PathBuf>();

        // If there is no native methods in the java file then there would be
        if let Ok(header_contents) = std::fs::read_to_string(h_path) {
            doc_contents.push_str("/// # Required Native Methods\n///```c\n///");

            doc_contents.push_str( &regex::Regex::new(r"/\*[^#]*?\);").expect("Bad Regex")
                .find_iter(&header_contents)
                .map(|match_val| match_val.as_str().replace("\n", "\n///") )
                .fold(String::new(), |mut c_code, match_str| {
                    c_code.push_str( &(match_str + "\n///\n///") );
                    c_code
                })
            );

            doc_contents.push_str("\n///```");
        }
    }

    file_contents
}

fn create_dex_files(android_sdk: &str, class_info: Vec<(Class, String)>) -> Vec<(Class, String)> {

    for (class, _) in class_info.iter() {

        let class_file_name = class.fully_qualified_name.clone() + ".class";

        let dex_file_name =  class.fully_qualified_name.clone() + ".dex";

        let dx_output = Command::new(
            [ android_sdk.as_ref(), Path::new("build-tools/28.0.3/dx")]
            .iter()
            .collect::<PathBuf>()
        )
        .current_dir( GENERATED_FILE_LOCATION )
        .args( &[
            "--dex",
            &( "--output=".to_string() + &dex_file_name ),
            &class_file_name
        ])
        .output()
        .expect("failed to execute dx");

        if !dx_output.status.success() {
            panic!("dx command failed: {}",
                String::from_utf8(dx_output.stderr).expect("Couldn't get stderr")
            );
        }
    }

    class_info
}

fn add_consts(
    consts_writer: build_const::ConstWriter,
    classes: JavaFiles,
    sdk_path: &str,
    sdk_version: usize
){
    let class_info = create_dex_files( &sdk_path, build_java_files( &sdk_path, sdk_version, classes));

    let mut consts_writer = consts_writer.finish_dependencies();

    for (class, code) in class_info {
        let class_bytes = std::fs::read(
            [GENERATED_FILE_LOCATION.to_string(), class.fully_qualified_name + ".dex"]
            .iter()
            .collect::<PathBuf>()
        )
        .expect("Couldn't read file");

        consts_writer.add_raw(&code);

        consts_writer.add_array(&class.name_of_constant, "u8", &class_bytes);
    }

    consts_writer.finish();
}

fn main() {
    let consts_file = Path::new("classbytes.rs");

    let android_sdk = std::env::var("ANDROID_SDK_PATH")
        .expect("ANDROID_SDK_PATH not defined, this must be the path to android.jar \
            (usually it's at {ANDROID_SDK}/platforms/android-{version}/android.jar)");

    let toml_bytes = std::fs::read("JavaFiles.toml").expect("File doesn't exist");

    let toml: JavaFiles = toml::de::from_slice(toml_bytes.as_slice()).expect("Bytes not deserializable");

    let mut consts = build_const::ConstWriter::from_path(consts_file).expect("ConstWrite not created");

    consts.add_raw(
        "//! This is an autogenerated file\n\
         //!\n\
         //! It was generated by the crate `classbytes` which can be found in subcrates/android\n\
         //!\n\
         //! This file contains constant arrays of java class data in android delvik bytecode.\n\
         //! In the comments above each constant is the java code that was used to create the class data.\n\
         \n\n"
    );

    add_consts(consts, toml, &android_sdk, 28);
}
