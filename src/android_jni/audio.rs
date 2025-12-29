use crate::android_frontend::AndroidFrontend;

use jni::objects::JShortArray;
use jni::sys::{jint, jlong, jshort};
use jni::JNIEnv;

pub(crate) fn fill_audio(env: JNIEnv, handle: jlong, frames: jint, out_pcm: JShortArray) -> jint {
    if handle == 0 {
        return 0;
    }

    let frontend = unsafe { &*(handle as *const AndroidFrontend) };

    let n = frames.max(0) as usize;
    if n == 0 {
        return 0;
    }

    let mut buf: Vec<i16> = vec![0; n];
    frontend.render_audio_i16_mono(&mut buf);

    // i16 -> jshort
    let buf_js: Vec<jshort> = buf.into_iter().map(|s| s as jshort).collect();
    let _ = env.set_short_array_region(out_pcm, 0, &buf_js);

    n as jint
}
