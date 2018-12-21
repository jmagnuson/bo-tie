//! This crate is a sub crate of bo-tie and is used for providing *rust --test* like features
//! for testing methods that require the jni env pointer for calling java methods.
extern crate proc_macro;
extern crate proc_macro2;
#[macro_use] extern crate syn;
#[macro_use] extern crate quote;
#[macro_use] extern crate lazy_static;
extern crate scribe;

use proc_macro::TokenStream;

use std::fs::File;
use std::path::PathBuf;
use std::sync::{Mutex};

static JAVA_METHOD_HOOK: &'static str = "//<{Method hook for crate android-test. Unfortunately \
    I cannot get removed, so please just ignore or delete me :P }>";

lazy_static! {
    static ref JAVA_FILE: ScribeWrapper = {
        let path = [ env!("JAVA_OUTPUT_PATH"), concat!(env!("CARGO_PKG_NAME"), "InstrumentTests", ".java")]
            .iter()
            .collect::<PathBuf>();

        initialize_java_file(File::create(&path).expect("Couldn't create file"));

        ScribeWrapper {
            buffer: Mutex::from(scribe::Buffer::from_file(&path).expect("Couldn't create buffer"))
        }
    };
}

/// Initialize the file
///
/// This writes to the file the imports, class declaration, and anything else needed for
/// implementation tests
fn initialize_java_file( mut file: File ) -> File {
    use std::io::Write;

    file.set_len(0).unwrap();

    write!(file, "\
import androidx.test.runner.AndroidJUnit4;
import org.junit.Test;

@LargeTest
public class {0}InstrumentTests {{

    {1}

}}\n",
        env!("CARGO_PKG_NAME"),
        JAVA_METHOD_HOOK
    ).expect("Couldn't write inizilation to file");

    file.sync_data().expect("Couldn't sync data");

    file
}

/// Wrapper for scribe::Buffer
///
/// This is dangerous, use carefully.
///
/// Unfortunately I don't think I have a better option but to use a static object for generating
/// the java file for use with android studio. This can deref to the lock
struct ScribeWrapper {
    buffer: Mutex<scribe::Buffer>,
}

unsafe impl Sync for ScribeWrapper {}
unsafe impl Send for ScribeWrapper {}

impl ScribeWrapper {
    fn write_android_test( &self, rust_fn: &syn::ItemFn ) {
        use scribe::buffer;

        let rust_fn_name = rust_fn.ident.to_string();

        let mut buffer_lg = self.buffer.lock().expect("Couldn't acquire lock for scribe::Buffer");

        let hook_position = {
            let needles = buffer_lg.search(JAVA_METHOD_HOOK);

            assert!(needles.len() == 1,
                "Found needles {:?}, this is a bug with android-test", needles);

            needles.first().unwrap().clone()
        };

        let hook_end_position = buffer::Position {
            line: hook_position.line,
            offset: hook_position.offset + JAVA_METHOD_HOOK.len(),
        };

        let method_name = format!("public native void {0}();

    @Test
    public void {0}UnitTest() {{
        {0}();
    }}

    {1}",
            get_java_method_name_from(rust_fn_name),
            JAVA_METHOD_HOOK
        );

        buffer_lg.delete_range(buffer::Range::new(hook_position, hook_end_position));

        (*buffer_lg).cursor.move_to(hook_position);

        buffer_lg.insert(method_name);

        buffer_lg.save().expect("Couldn't save file");
    }
}

fn is_type_jni_env( ty: &syn::Type ) -> bool {
    let jni_name = "jni";
    let env_name = "JNIEnv";

    match ty {
        syn::Type::Path(ref ty_path) =>  {

            let ref path = ty_path.path;

            match path.segments.len() {
                2 => {
                    let jni = path.segments.first().unwrap().into_value();
                    let env = path.segments.last().unwrap().into_value();

                    jni.ident.to_string() == jni_name &&
                    env.ident.to_string() == env_name
                },
                1 => path.is_ident(env_name),
                _ => false,
            }
        },
        syn::Type::Verbatim(_) => { panic!("oops") },
        _ => false
    }
}

/// Performs a number of asserts on an the passed fn-type item to make sure that it conforms to the
/// requirements for the android_test attribute
///
/// This returns Some(arg) if the JNIEnv input is specified in the function, otherwise None is
/// returned if there is no input to the function
fn function_check<'a>( fn_item: &'a syn::ItemFn) -> Option<&'a syn::Ident> {

    let max_lifetimes = 1;
    let max_inputs = 1;

    let mut lifetime: Option<&syn::LifetimeDef> = None;
    let mut function_input: Option<&syn::Ident> = None;

    // Basic checks on test function type
    assert!(fn_item.constness.is_none(), "Android test functions cannot be constant");
    assert!(fn_item.unsafety.is_none(), "Android test functions cannot be unsafe");
    assert!(fn_item.asyncness.is_none(), "Android test functions cannot be async");
    assert!(fn_item.abi.is_none(), r#"Android test functions cannot contain any abi (eg *extern "C"*)"#);

    // ---- Generics ----
    // function cannot be generic, but it can have a lifetime for parameter
    assert!(fn_item.decl.generics.params.len() <= max_lifetimes,
        "Android test functions cannot have more then one generic parameter");

    // check (if there is one generic parameter) that the only generic parameter is a lifetime
    assert!(fn_item.decl.generics.params.first()
        .map_or(true, |g| {

            if let syn::GenericParam::Lifetime(ref lt) = g.value() {
                lifetime = Some(lt);
                true
            } else {
                false
            }

        }),
        "Android test functions can only have a lifetime as its only generic parameter"
    );

    // ---- Function inputs ---
    assert!(fn_item.decl.inputs.len() <= max_inputs,
        "Android test functions can only have one input, jenv: jni::JNIEnv ");

    // See if input type (if it exists) is jni::JNIEnv and that the input is a named variable.
    // If the input exists and is valid, then the variable name will then be set to be returned
    // from this function.
    assert!(fn_item.decl.inputs.first().map_or(true, |input| {
            match input.value() {
                syn::FnArg::Captured(ref arg_capt) => {
                    is_type_jni_env(&arg_capt.ty) && match arg_capt.pat {
                        syn::Pat::Ident( ref pat_ident ) => {
                            assert!(pat_ident.by_ref.is_none(), r#"function input cannot have "ref" qualifier"#);
                            assert!(pat_ident.mutability.is_none(), r#"function input cannot be mutable"#);
                            assert!(pat_ident.subpat.is_none(), r#"function input cannot have a subpattern"#);
                            function_input = Some(&pat_ident.ident);
                            true
                        },
                        syn::Pat::Wild(_) => true,
                        _ => false,
                    }
                }
                _ => false,
            }
        }),
        r#"Android test functions can only have a named variable input of type jni::JNIEnv (eg. "jenv: jni::JNIEnv")"#
    );

    function_input
}

/// Converts a character to the jni equavalent
///
/// The full_msg parameter is only there for when an error should occur when a character in that
/// string is invalid.
fn chars_to_jni( msg: &str ) -> String {

    let mut jni_msg = String::default();

    for character in msg.chars() {
        jni_msg.push_str(
            & if '_' == character || '-' == character {
                "_1".to_string()
            } else if character.is_ascii_alphanumeric() {
                character.to_string()
            } else {
                panic!("Character '{}' in '{}' unrecognized", character, msg);
            }
        );
    }

    jni_msg
}

/// Generates a Java appropriate method name for a function
fn get_java_method_name_from( function: String ) -> String {
    let mut java_name = String::default();
    let mut next_upper = false;

    for character in function.chars() {
        if character == '_' {
            next_upper = true;
        } else {
            if next_upper {
                java_name.push_str(
                    & if character.is_ascii_alphabetic() {
                        character.to_ascii_uppercase().to_string()
                    } else {
                        character.to_string()
                    }
                );
            }
            else {
                java_name.push(character);
            }
            next_upper = false;
        }
    }

    java_name
}

/// Generates a Java fully qualified class name for the crate
fn get_java_fq_jni_class_name( package: String, class: String ) -> String {
    if package.is_empty() {
        chars_to_jni(&class)
    } else {
        chars_to_jni(&package) + "_" + &chars_to_jni(&class)
    }
}

/// This takes the function that is labeled by the attribute *android_test* and returns a new
/// function that has the external interface
fn make_jni_method( function: &syn::ItemFn ) -> proc_macro2::TokenStream {
    let java_fq_class_name = get_java_fq_jni_class_name(
        env!("JAVA_PACKAGE").to_string(),
        env!("CARGO_PKG_NAME").to_string()
    );
    let java_function_name = get_java_method_name_from(function.ident.to_string());

    let jni_function_name = {
        let mut name = "Java_".to_string();
        name.push_str(&java_fq_class_name);
        name.push('_');
        name.push_str(&java_function_name);
        name
    };

    let jni_function = syn::Ident::new( &jni_function_name, proc_macro2::Span::call_site() );

    let called_function = function.clone();

    quote! {
        #[no_mangle]
        #[allow(non_snake_case)]
        pub extern "system" fn #jni_function ( env: ::jni::JNIEnv, _: ::jni::objects::JClass ) {
            #called_function(env);
        }
    }
}

/// Test labeling macro
///
/// Functions maked by this attribute must have the signature fn( _: jni::JNIEnv ) -> (). For now
/// The return type of Result<(),E> where E is the error is not implemented.
#[allow(unused_variables)] // this is needed for input 'item' when feature 'android-test' isn't used
#[proc_macro_attribute]
pub fn android_test( _attr: TokenStream, item: TokenStream ) -> TokenStream {
    if cfg!(feature="android-test") {
        let input = parse_macro_input!(item as syn::ItemFn);

        function_check(&input);

        let java_interface_function = make_jni_method(&input);

        JAVA_FILE.write_android_test(&input);

        TokenStream::from( quote!{
            #java_interface_function
            #input
        })
    }
    else {
        TokenStream::new()
    }
}
