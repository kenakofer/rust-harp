//! Minimal AAudio output path (API 26+) for low-latency Android audio.
//!
//! We keep the implementation small and self-contained so it can be swapped/refined later.

#![cfg(all(target_os = "android", feature = "android"))]

use crate::android_frontend::AndroidFrontend;

use std::ffi::c_void;
use std::sync::{Mutex, OnceLock};

#[repr(C)]
pub struct AAudioStream {
    _p: [u8; 0],
}

#[repr(C)]
pub struct AAudioStreamBuilder {
    _p: [u8; 0],
}

// From aaudio/AAudio.h
const AAUDIO_OK: i32 = 0;
const AAUDIO_DIRECTION_OUTPUT: i32 = 0;
const AAUDIO_FORMAT_PCM_I16: i32 = 2;

const AAUDIO_PERFORMANCE_MODE_NONE: i32 = 10;
const AAUDIO_PERFORMANCE_MODE_LOW_LATENCY: i32 = 12;

const AAUDIO_SHARING_MODE_SHARED: i32 = 1;
const AAUDIO_SHARING_MODE_EXCLUSIVE: i32 = 0;

const AAUDIO_CALLBACK_RESULT_CONTINUE: i32 = 0;

type AAudioDataCallback = Option<unsafe extern "C" fn(
    stream: *mut AAudioStream,
    user_data: *mut c_void,
    audio_data: *mut c_void,
    num_frames: i32,
) -> i32>;

type AAudioErrorCallback = Option<unsafe extern "C" fn(
    stream: *mut AAudioStream,
    user_data: *mut c_void,
    error: i32,
)>;

#[link(name = "aaudio")]
extern "C" {
    fn AAudio_createStreamBuilder(builder: *mut *mut AAudioStreamBuilder) -> i32;
    fn AAudioStreamBuilder_delete(builder: *mut AAudioStreamBuilder);

    fn AAudioStreamBuilder_setDirection(builder: *mut AAudioStreamBuilder, direction: i32);
    fn AAudioStreamBuilder_setSampleRate(builder: *mut AAudioStreamBuilder, sample_rate: i32);
    fn AAudioStreamBuilder_setChannelCount(builder: *mut AAudioStreamBuilder, channel_count: i32);
    fn AAudioStreamBuilder_setFormat(builder: *mut AAudioStreamBuilder, format: i32);
    fn AAudioStreamBuilder_setPerformanceMode(builder: *mut AAudioStreamBuilder, mode: i32);
    fn AAudioStreamBuilder_setSharingMode(builder: *mut AAudioStreamBuilder, mode: i32);
    fn AAudioStreamBuilder_setDataCallback(
        builder: *mut AAudioStreamBuilder,
        callback: AAudioDataCallback,
        user_data: *mut c_void,
    );
    fn AAudioStreamBuilder_setErrorCallback(
        builder: *mut AAudioStreamBuilder,
        callback: AAudioErrorCallback,
        user_data: *mut c_void,
    );

    fn AAudioStreamBuilder_openStream(
        builder: *mut AAudioStreamBuilder,
        stream: *mut *mut AAudioStream,
    ) -> i32;

    fn AAudioStream_requestStart(stream: *mut AAudioStream) -> i32;
    fn AAudioStream_requestStop(stream: *mut AAudioStream) -> i32;
    fn AAudioStream_close(stream: *mut AAudioStream) -> i32;

    fn AAudioStream_getSampleRate(stream: *mut AAudioStream) -> i32;
    fn AAudioStream_getFramesPerBurst(stream: *mut AAudioStream) -> i32;

    fn AAudioStream_setBufferSizeInFrames(stream: *mut AAudioStream, num_frames: i32) -> i32;
}

struct CallbackCtx {
    frontend: *const AndroidFrontend,
}

unsafe extern "C" fn data_cb(
    _stream: *mut AAudioStream,
    user_data: *mut c_void,
    audio_data: *mut c_void,
    num_frames: i32,
) -> i32 {
    if user_data.is_null() || audio_data.is_null() || num_frames <= 0 {
        return AAUDIO_CALLBACK_RESULT_CONTINUE;
    }

    let ctx = &*(user_data as *const CallbackCtx);
    let frontend = &*ctx.frontend;

    let out = std::slice::from_raw_parts_mut(audio_data as *mut i16, num_frames as usize);
    frontend.render_audio_i16_mono(out);

    AAUDIO_CALLBACK_RESULT_CONTINUE
}

unsafe extern "C" fn error_cb(
    _stream: *mut AAudioStream,
    _user_data: *mut c_void,
    _error: i32,
) {
    // For MVP, ignore; if we see stream death we can implement restart.
}

struct AAudioOut {
    stream: *mut AAudioStream,
    ctx: *mut CallbackCtx,
}

// AAudio owns the callback thread; we synchronize access via the global Mutex.
// These raw pointers are only used through the AAudio C API and freed in Drop.
unsafe impl Send for AAudioOut {}

impl Drop for AAudioOut {
    fn drop(&mut self) {
        unsafe {
            if !self.stream.is_null() {
                let _ = AAudioStream_requestStop(self.stream);
                let _ = AAudioStream_close(self.stream);
                self.stream = std::ptr::null_mut();
            }
            if !self.ctx.is_null() {
                drop(Box::from_raw(self.ctx));
                self.ctx = std::ptr::null_mut();
            }
        }
    }
}

static AAUDIO: OnceLock<Mutex<Option<AAudioOut>>> = OnceLock::new();

fn aaudio_state() -> &'static Mutex<Option<AAudioOut>> {
    AAUDIO.get_or_init(|| Mutex::new(None))
}

pub fn start(frontend: &AndroidFrontend) -> bool {
    let mut guard = aaudio_state().lock().unwrap();
    if guard.is_some() {
        return true;
    }

    let ctx = Box::new(CallbackCtx {
        frontend: frontend as *const AndroidFrontend,
    });
    let ctx_ptr = Box::into_raw(ctx);

    unsafe {
        let mut builder: *mut AAudioStreamBuilder = std::ptr::null_mut();
        if AAudio_createStreamBuilder(&mut builder) != AAUDIO_OK || builder.is_null() {
            drop(Box::from_raw(ctx_ptr));
            return false;
        }

        AAudioStreamBuilder_setDirection(builder, AAUDIO_DIRECTION_OUTPUT);
        AAudioStreamBuilder_setChannelCount(builder, 1);
        AAudioStreamBuilder_setFormat(builder, AAUDIO_FORMAT_PCM_I16);
        AAudioStreamBuilder_setPerformanceMode(builder, AAUDIO_PERFORMANCE_MODE_LOW_LATENCY);
        AAudioStreamBuilder_setDataCallback(builder, Some(data_cb), ctx_ptr as *mut c_void);
        AAudioStreamBuilder_setErrorCallback(builder, Some(error_cb), ctx_ptr as *mut c_void);

        // First try exclusive; if it fails, retry shared.
        for &sharing in &[AAUDIO_SHARING_MODE_EXCLUSIVE, AAUDIO_SHARING_MODE_SHARED] {
            AAudioStreamBuilder_setSharingMode(builder, sharing);

            let mut stream: *mut AAudioStream = std::ptr::null_mut();
            let rc = AAudioStreamBuilder_openStream(builder, &mut stream);
            if rc != AAUDIO_OK || stream.is_null() {
                continue;
            }

            let sr = AAudioStream_getSampleRate(stream).max(1) as u32;
            frontend.set_sample_rate(sr);

            let fpb = AAudioStream_getFramesPerBurst(stream).max(1);
            // Small buffer to keep latency down but avoid underruns.
            let _ = AAudioStream_setBufferSizeInFrames(stream, fpb.saturating_mul(2));

            let _ = AAudioStream_requestStart(stream);

            AAudioStreamBuilder_delete(builder);

            *guard = Some(AAudioOut { stream, ctx: ctx_ptr });
            return true;
        }

        AAudioStreamBuilder_delete(builder);
        drop(Box::from_raw(ctx_ptr));
        false
    }
}

pub fn stop() {
    let mut guard = aaudio_state().lock().unwrap();
    *guard = None;
}
