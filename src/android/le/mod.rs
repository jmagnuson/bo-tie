//! wrapper contains structures that wrap around the raw jni objects
//!
//! Every Struct in jni_wrapper carries the JNIEnv variable along with the jobject variable so
//! that the lifetime of the JNIEnv

pub mod advertise;
pub mod scan;
