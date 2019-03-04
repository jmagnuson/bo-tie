#![recursion_limit="256"]
#![feature(arbitrary_self_types)]

//! This crate is a sub crate of bo-tie and is used for providing a rust like process
//! for testing methods that require the jni env pointer. However, do not
//! actually provide the "--test" flag (or use 'cargo test') when using this crate. Instead, in the
//! `cargo.toml` file, add a feature that will build this crate with the feature `android-test`
//! (bo-tie just uses the same feature name in its cargo.toml).
//!
//! What this crate does is provide an attribute `android_test` that is used to generate a java
//! file that can be used with a android project for running instrument tests. Another file is also
//! generated (if not already generated) which is the system library that needs to be included with
//! the project in the 'libs/{abi}/' folder. If the feature isn't specified to be used, then the
//! default is to build without `android_test` and nothing is generated.
//!
//! Functions that have the attribue `android_test` can take an input of the type JniEnv defined in
//! the crate `jni` or have no inputs.
//!
//! This attribute is designed to look like it works the same way as the '#\[test\]' attribute.
//! What it does however is create another (exported) rust function with the correct JNI method
//! signature for use with the generated java file.
//!
//! The following environment variable is required
//! * LIBRARY_FILE_NAME The name of the library file containing the 'native' (to java) test
//! functions
//! * JAVA_OUTPUT_PATH The path where to put the generated java file
//!
//! The following enviroment variables can be used to configure the generated java test file
//! * JAVA_PACKAGE The package that the class should be part of
//! * INSTRUMENT_TEST_CLASS_PREFIX A prefix to be added to the class name
//!
extern crate proc_macro;
extern crate proc_macro2;
#[macro_use] extern crate syn;
#[macro_use] extern crate quote;
#[macro_use] extern crate lazy_static;
extern crate scribe;

use proc_macro::TokenStream;

use std::fs::File;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

static JAVA_METHOD_HOOK: &'static str = "//<{Method hook}>";
static JAVA_RULE_HOOK: &'static str = "//<{Rule hook}>";

macro_rules! class_name {
    () => { "InstrumentTests" };
}

lazy_static! {
    static ref JAVA_FILE: Mutex<ScribeWrapper> = {
        let path = [ env!("JAVA_OUTPUT_PATH"), concat!( class_name!(), ".java")]
            .iter()
            .collect::<PathBuf>();

        initialize_java_file(File::create(&path).expect("Couldn't create file"));

        Mutex::from(ScribeWrapper {
            permissions: std::collections::HashSet::new(),
            permission_number: 0,
            buffer: scribe::Buffer::from_file(&path).expect("Couldn't create buffer")
        })
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
{package}import androidx.test.runner.AndroidJUnit4;
import org.junit.Test;

public class {prefix}InstrumentTests {{

    static {{
        System.loadLibrary(\"{library}\");
    }}

    {rule}

    {method}

}}\n",
        package = if let Some(package) = option_env!("JAVA_PACKAGE") {
            format!("package {};\n", package)
        } else {
            "".to_string()
        },
        prefix = env!("INSTRUMENT_TEST_CLASS_PREFIX"),
        library = env!("LIBRARY_FILE_NAME"),
        method = JAVA_METHOD_HOOK,
        rule = JAVA_RULE_HOOK,
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
    permissions: std::collections::HashSet<String>,
    permission_number: usize,
    buffer: scribe::Buffer,
}

unsafe impl Sync for ScribeWrapper {}
unsafe impl Send for ScribeWrapper {}

impl ScribeWrapper {

    fn insert_at( mut self: MutexGuard<Self>, hook: &str, to_write: &str) {
        use scribe::buffer;

        let hook_position = {
            let needles = self.buffer.search(hook);

            assert!(needles.len() == 1,
                "Found more then one needle: '{:?}'; this is a bug with android-test", needles);

            needles.first().unwrap().clone()
        };

        let hook_end_position = buffer::Position {
            line: hook_position.line,
            offset: hook_position.offset + hook.len(),
        };

        self.buffer.delete_range(buffer::Range::new(hook_position, hook_end_position));

        self.buffer.cursor.move_to(hook_position);

        self.buffer.insert(to_write);

        self.buffer.save().expect("Couldn't save file");
    }

    fn write_android_rule( mut self: MutexGuard<Self>, rule: &str ) {
        if self.permissions.insert(rule.to_string()) {
            let java_rule = format!(
                "@{rule} public {gp} mPermissionRule{cnt} = {gp}.grant({per}); {hook}",
                rule = "org.junit.Rule",
                gp = "androidx.test.rule.GrantPermissionRule",
                cnt = self.permission_number,
                per = format!("android.Manifest.permission.{}", rule.trim_matches('"').trim()),
                hook = JAVA_RULE_HOOK,
            );

            self.permission_number += 1;

            self.insert_at( JAVA_RULE_HOOK, &java_rule);
        }
    }

    fn write_android_test( self: MutexGuard<Self>, rust_fn: &syn::ItemFn ) {

        let rust_fn_name = rust_fn.ident.to_string();

        let method_name = format!(
            "public native void {method}(); @Test public void {method}UnitTest() {{ {method}(); }} {hook}",
            method = get_java_method_name_from(rust_fn_name),
            hook = JAVA_METHOD_HOOK
        );

        self.insert_at(JAVA_METHOD_HOOK, &method_name);
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
    let max_inputs = 2;

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
        "Android test functions can only have up to two input, jenv: jni::JNIEnv ");

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

/// Converts a java string to the jni equavalent
///
/// This replaces the characters that are part of a method, package, or class into the string
/// sequence that would be representing those characters in the jni symbol name
fn chars_to_jni( msg: &str ) -> String {

    let mut jni_msg = String::default();

    for character in msg.chars() {
        jni_msg.push_str(
            & if '_' == character || '-' == character {
                "_1".to_string()
            } else if '.' == character {
                "_".to_string()
            }else if character.is_ascii_alphanumeric() {
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
///
/// This function assumes that the input `function` has already been checked by `function_check`
fn make_jni_method( function: &syn::ItemFn ) -> proc_macro2::TokenStream {

    let java_fq_class_name = get_java_fq_jni_class_name(
        env!("JAVA_PACKAGE").to_string(),
        env!("INSTRUMENT_TEST_CLASS_PREFIX").to_string() + class_name!()
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

    let called_function_ident = function.ident.clone();

    let call_function = if function.decl.inputs.is_empty() {
        quote! {
            #called_function_ident()
        }
    } else if function.decl.inputs.len() == 2 {
        quote! {
            #called_function_ident(env, class)
        }
    } else {
        quote! {
            #called_function_ident(env)
        }
    };

    quote! {
        #[no_mangle]
        #[allow(non_snake_case)]
        #[allow(unused_variables)]
        pub extern "system" fn #jni_function ( env: ::jni::JNIEnv, class: ::jni::objects::JClass ) {
            use jni;
            use std::panic;
            use std::sync::{Arc,Mutex};

            let ni = env.get_native_interface();

            let rust_panic_message = Arc::new(Mutex::new(String::from("none")));

            let hook_rust_panic_message = rust_panic_message.clone();

            panic::set_hook(Box::new(move |panic_info| {
                if let Ok(mut msg) = hook_rust_panic_message.lock() {

                    let mut set_message = | payload: &str |  {
                        if let Some(location) = panic_info.location() {
                            *msg = format!("\nthread '{t}' panicked at {m:?}, {f}:{l}:{c}",
                                t = std::thread::current().name().unwrap_or("Unknown"),
                                m = payload,
                                f = location.file(),
                                l = location.line(),
                                c = location.column(),
                            );
                        }
                        else {
                            *msg = format!("\nthread '{t}' panicked at {m:?}",
                                t = std::thread::current().name().unwrap_or("Unknown"),
                                m = payload
                            );
                        }
                    };

                    if let Some(payload) = panic_info.payload().downcast_ref::<&str>() {
                        set_message(payload);
                    }
                    else if let Some(payload) = panic_info.payload().downcast_ref::<String>() {
                        set_message(&payload);
                    }
                }
            }));

            if let Err(_) = panic::catch_unwind(|| {
                #call_function;
            }) {
                let jenv = unsafe{ jni::JNIEnv::from_raw(ni) }.expect("couldn't get jni env");

                let jni_exception_msg = if let Ok(jthrowable) = jenv.exception_occurred() {

                    jenv.exception_clear().expect("Couldn't clear jni exception");

                    match jenv.call_method(
                        jthrowable.into(),
                        "toString",
                        "()Ljava/lang/String;",
                        &[])
                    {
                        Ok(jni::objects::JValue::Object(exception_msg)) => {
                            match jenv.get_string( jni::objects::JString::from(exception_msg) ) {
                                Ok(jstring) => jstring.into(),
                                Err(_) => {
                                    jenv.exception_clear().unwrap();
                                    "Could not get jni exception message".to_string()
                                }
                            }
                        }
                        _ => {
                            jenv.exception_clear().unwrap();
                            "Could not get jni exception message".to_string()
                        }
                    }
                } else {
                    "n/a".to_string()
                };

                jenv.throw_new("java/lang/RuntimeException",
                    format!("\nrust error: {},\njni error: {}\n",
                        (*rust_panic_message.lock().expect("Couldn't get panic message")).replace("\\n", "\n"),
                        jni_exception_msg,
                    )
                ).expect("Couldn't throw error");
            }
        }
    }
}

fn parse_permissions( token_itr: &mut Iterator<Item=proc_macro::TokenTree> ) {

    use proc_macro::TokenTree::*;

    let err_msg = "permissions must be in the form of 'permissions = ( ANDROID_PERMISSION_1, \
        ANDROID_PERMISSION_2, ... )";

    macro_rules! get_punctuation {
        ($default:expr, $next_item:expr ) => {
            $next_item.map_or(' ', |tree| { if let Punct(p) = tree { p.as_char() } else { ' ' } })
        }
    }

    assert_eq!( '=', get_punctuation!(' ', token_itr.next()), "{}", err_msg );

    if let Some(Group(group)) = token_itr.next() {
        assert_eq!( proc_macro::Delimiter::Parenthesis, group.delimiter(), "{}", err_msg );

        let mut group_itr = group.stream().into_iter();

        while let Some(token) = group_itr.next() {
            if let Literal(literal) = token {

                // add the permission
                JAVA_FILE.lock()
                .expect("Couldn't lock JAVA_FILE")
                .write_android_rule(&literal.to_string());

                // Check that the next token is either a comma or nothing\
                // If it is nothing then there is nothing left to process
                match group_itr.next() {
                    Some(Punct(p)) => assert_eq!( ',', p.as_char(), "{}", err_msg),
                    None => break,
                    _ => panic!("{}", err_msg),
                };
            } else {
                panic!("{}", err_msg);
            }
        }
    }
}

/// A struct containing all the information returned from parsing the arguments
struct ParsedArgs {
    pub is_ignored: bool
}

impl Default for ParsedArgs {
    fn default() -> Self {
        ParsedArgs {
            is_ignored: false,
        }
    }
}

fn parse_args( args: TokenStream ) -> ParsedArgs {
    let mut token_iter = args.into_iter();

    let mut parsed_args = ParsedArgs::default();

    while let Some(token) = token_iter.next() {
        match token {
            proc_macro::TokenTree::Ident(ident) => match ident.to_string().as_str() {
                "permissions" => parse_permissions(&mut token_iter),
                "ignore" => parsed_args.is_ignored = true,
                _ => panic!("Unknown meta name value: {}", ident.to_string())
            },
            _ => panic!("Unexpected attribute meta list item")
        }
    }

    parsed_args
}

/// Test labeling macro
///
/// Functions maked by this attribute must have the signature fn( _: jni::JNIEnv ) -> (). For now
/// The return type of Result<(),E> where E is the error is not implemented.
///
/// The macro can take the following as arguments
#[proc_macro_attribute]
pub fn android_test( args: TokenStream, item: TokenStream ) -> TokenStream {

    let parsed_args = parse_args( args );

    if cfg!(feature="android-test") && !parsed_args.is_ignored {
        let input = parse_macro_input!(item as syn::ItemFn);

        function_check(&input);

        let java_interface_function = make_jni_method(&input);

        JAVA_FILE.lock().expect("Couldn't lock JAVA_FILE").write_android_test(&input);

        TokenStream::from( quote!{
            #java_interface_function
            #input
        })
    }
    else {
        TokenStream::new()
    }
}
