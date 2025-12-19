use jni_sys::{jint, jclass, JNIEnv};

/// Simple JNI hook so an Android Activity can verify the Rust library loads.
#[no_mangle]
pub unsafe extern "C" fn Java_com_rustharp_app_MainActivity_rustInit(
    _env: *mut JNIEnv,
    _class: jclass,
) -> jint {
    1
}
