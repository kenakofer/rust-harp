//! Minimal AAudio output path (API 26+) for low-latency Android audio.
//!
//! We keep the implementation small and self-contained so it can be swapped/refined later.

#![cfg(all(target_os = "android", feature = "android"))]

use crate::android_audio::SquareSynth;
use crate::android_frontend::{AndroidFrontend, AudioMsg};
use crate::notes::{MidiNote, NoteVolume, Transpose};

use std::ffi::{c_char, c_void};
use std::sync::{mpsc::Receiver, Mutex, OnceLock};
use std::sync::atomic::{AtomicI32, AtomicU32, Ordering};

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

// NDK constants:
//   AAUDIO_FORMAT_UNSPECIFIED = 0
//   AAUDIO_FORMAT_PCM_I16     = 1
//   AAUDIO_FORMAT_PCM_FLOAT   = 2
const AAUDIO_FORMAT_PCM_I16: i32 = 1;
const AAUDIO_FORMAT_PCM_FLOAT: i32 = 2;
const AAUDIO_FORMAT_PCM_I8: i32 = 3;

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
    fn AAudioStream_getChannelCount(stream: *mut AAudioStream) -> i32;
    fn AAudioStream_getFormat(stream: *mut AAudioStream) -> i32;
    fn AAudioStream_getBufferSizeInFrames(stream: *mut AAudioStream) -> i32;
    fn AAudioStream_getXRunCount(stream: *mut AAudioStream) -> i32;

    fn AAudioStream_setBufferSizeInFrames(stream: *mut AAudioStream, num_frames: i32) -> i32;
}

#[link(name = "log")]
extern "C" {
    fn __android_log_write(prio: i32, tag: *const c_char, text: *const c_char) -> i32;
}

const ANDROID_LOG_INFO: i32 = 4;

fn android_log(prio: i32, msg: &str) {
    let tag = b"RustHarp\0";
    let mut buf = msg.as_bytes().to_vec();
    buf.push(0);
    unsafe {
        let _ = __android_log_write(prio, tag.as_ptr() as *const c_char, buf.as_ptr() as *const c_char);
    }
}

fn android_log_static(prio: i32, msg: &'static [u8]) {
    let tag = b"RustHarp\0";
    unsafe {
        let _ = __android_log_write(prio, tag.as_ptr() as *const c_char, msg.as_ptr() as *const c_char);
    }
}

static XRUNS_INCREASED: &[u8] = b"AAudio xruns increased\0";

struct CallbackCtx {
    rx: Receiver<AudioMsg>,
    synth: SquareSynth,
    channels: i32,
    format: i32,
    call_count: AtomicU32,
    last_xruns: AtomicI32,
}

unsafe extern "C" fn data_cb(
    stream: *mut AAudioStream,
    user_data: *mut c_void,
    audio_data: *mut c_void,
    num_frames: i32,
) -> i32 {
    if user_data.is_null() || audio_data.is_null() || num_frames <= 0 {
        return AAUDIO_CALLBACK_RESULT_CONTINUE;
    }

    let ctx = &mut *(user_data as *mut CallbackCtx);

    // Match desktop's MIDI_BASE_TRANSPOSE (C2)
    const MIDI_BASE_TRANSPOSE: Transpose = Transpose(36);

    // Drain control messages (no locks/allocations in steady state).
    while let Ok(msg) = ctx.rx.try_recv() {
        match msg {
            AudioMsg::NoteOn(pn) => {
                let MidiNote(m) = MIDI_BASE_TRANSPOSE + pn.note;
                let NoteVolume(v) = pn.volume;
                ctx.synth.note_on(MidiNote(m), v);
            }
            AudioMsg::SetSampleRate(sr) => {
                ctx.synth = SquareSynth::new(sr.max(1));
            }
        }
    }

    // Underrun detection: query xRun count periodically and log when it changes.
    // IMPORTANT: avoid allocations/log spam inside the realtime callback.
    if !stream.is_null() {
        let n = ctx.call_count.fetch_add(1, Ordering::Relaxed);
        if (n % 128) == 0 {
            let xruns = AAudioStream_getXRunCount(stream);
            let prev = ctx.last_xruns.load(Ordering::Relaxed);
            if xruns >= 0 && xruns != prev {
                ctx.last_xruns.store(xruns, Ordering::Relaxed);
                // Log a static message to avoid heap allocation in the callback.
                android_log_static(4, XRUNS_INCREASED);
            }
        }
    }

    // Be defensive: some devices may ignore our requested channel count.
    let channels = ctx.channels.max(1) as usize;

    match ctx.format {
        AAUDIO_FORMAT_PCM_I16 => {
            let out = std::slice::from_raw_parts_mut(
                audio_data as *mut i16,
                (num_frames as usize) * channels,
            );
            ctx.synth.render_i16_interleaved(out, channels);
        }
        AAUDIO_FORMAT_PCM_FLOAT => {
            let out = std::slice::from_raw_parts_mut(
                audio_data as *mut f32,
                (num_frames as usize) * channels,
            );
            ctx.synth.render_f32_interleaved(out, channels);
        }
        AAUDIO_FORMAT_PCM_I8 => {
            let out = std::slice::from_raw_parts_mut(
                audio_data as *mut i8,
                (num_frames as usize) * channels,
            );
            out.fill(0);
        }
        _ => {
            // Unknown format; best effort: write silence assuming 32-bit samples.
            let out = std::slice::from_raw_parts_mut(
                audio_data as *mut u32,
                (num_frames as usize) * channels,
            );
            out.fill(0);
        }
    }

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

pub fn start(frontend: &mut AndroidFrontend) -> bool {
    let mut guard = aaudio_state().lock().unwrap();
    if guard.is_some() {
        return true;
    }

    let Some(rx) = frontend.take_audio_rx() else {
        return false;
    };

    let ctx = Box::new(CallbackCtx {
        rx,
        synth: SquareSynth::new(48_000),
        channels: 1,
        format: AAUDIO_FORMAT_PCM_FLOAT,
        call_count: AtomicU32::new(0),
        last_xruns: AtomicI32::new(-1),
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
        AAudioStreamBuilder_setPerformanceMode(builder, AAUDIO_PERFORMANCE_MODE_LOW_LATENCY);
        AAudioStreamBuilder_setDataCallback(builder, Some(data_cb), ctx_ptr as *mut c_void);
        AAudioStreamBuilder_setErrorCallback(builder, Some(error_cb), ctx_ptr as *mut c_void);

        // Try low-latency combos first.
        for &sharing in &[AAUDIO_SHARING_MODE_EXCLUSIVE, AAUDIO_SHARING_MODE_SHARED] {
            AAudioStreamBuilder_setSharingMode(builder, sharing);
            for &fmt in &[AAUDIO_FORMAT_PCM_FLOAT, AAUDIO_FORMAT_PCM_I16] {
                AAudioStreamBuilder_setFormat(builder, fmt);

                let mut stream: *mut AAudioStream = std::ptr::null_mut();
                let rc = AAudioStreamBuilder_openStream(builder, &mut stream);
                if rc != AAUDIO_OK || stream.is_null() {
                    continue;
                }

                let sr = AAudioStream_getSampleRate(stream).max(1) as u32;
                (*ctx_ptr).synth = SquareSynth::new(sr.max(1));

                // Record actual stream config for the callback.
                (*ctx_ptr).channels = AAudioStream_getChannelCount(stream).max(1);
                (*ctx_ptr).format = AAudioStream_getFormat(stream);

                let fpb = AAudioStream_getFramesPerBurst(stream).max(1);
                // Small buffer to keep latency down but avoid underruns.
                let target_buf = fpb.saturating_mul(2);
                let rc_buf = AAudioStream_setBufferSizeInFrames(stream, target_buf);
                let actual_buf = AAudioStream_getBufferSizeInFrames(stream);
                android_log(
                    ANDROID_LOG_INFO,
                    &format!(
                        "AAudio cfg sr={sr} ch={} fmt={} fpb={fpb} buf={actual_buf} (set rc={rc_buf})",
                        (*ctx_ptr).channels,
                        (*ctx_ptr).format
                    ),
                );

                let _ = AAudioStream_requestStart(stream);

                AAudioStreamBuilder_delete(builder);

                *guard = Some(AAudioOut { stream, ctx: ctx_ptr });
                return true;
            }
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
