use crate::android_frontend::AndroidFrontend;
use crate::app_state::KeyState;
use crate::input_map::{self, UiKey};
use crate::layout;

use jni::objects::{JBoolean, JClass, JIntArray};
use jni::sys::{jboolean, jint, jlong};
use jni::JNIEnv;

/// Simple JNI hook so an Android Activity can verify the Rust library loads.
#[no_mangle]
pub extern "system" fn Java_com_rustharp_app_MainActivity_rustInit(
    _env: JNIEnv,
    _class: JClass,
) -> jint {
    1
}

#[no_mangle]
pub extern "system" fn Java_com_rustharp_app_MainActivity_rustCreateFrontend(
    _env: JNIEnv,
    _class: JClass,
) -> jlong {
    let frontend = Box::new(AndroidFrontend::new());
    Box::into_raw(frontend) as jlong
}

#[no_mangle]
pub extern "system" fn Java_com_rustharp_app_MainActivity_rustDestroyFrontend(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) {
    if handle == 0 {
        return;
    }
    unsafe {
        drop(Box::from_raw(handle as *mut AndroidFrontend));
    }
}

#[no_mangle]
pub extern "system" fn Java_com_rustharp_app_MainActivity_rustHandleAndroidKey(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
    key_code: jint,
    unicode_char: jint,
    is_down: jboolean,
) -> jint {
    if handle == 0 {
        return 0;
    }

    let state = if is_down != 0 {
        KeyState::Pressed
    } else {
        KeyState::Released
    };

    let key = if unicode_char != 0 {
        // Java already lowercases for us.
        UiKey::Char(char::from_u32(unicode_char as u32).unwrap_or('\0'))
    } else {
        // Key codes from android.view.KeyEvent
        match key_code {
            61 => UiKey::Tab,          // KEYCODE_TAB
            113 | 114 => UiKey::Control, // KEYCODE_CTRL_LEFT / KEYCODE_CTRL_RIGHT
            _ => return 0,
        }
    };

    let Some(app_event) = input_map::key_event_from_ui(state, key) else {
        return 0;
    };

    let frontend = unsafe { &mut *(handle as *mut AndroidFrontend) };
    let effects = frontend.engine_mut().handle_event(app_event);

    // Bit 0: needs redraw
    if effects.redraw { 1 } else { 0 }
}

/// Render a black background + vertical string lines into `out_pixels` (ARGB_8888).
#[no_mangle]
pub extern "system" fn Java_com_rustharp_app_MainActivity_rustRenderStrings(
    mut env: JNIEnv,
    _class: JClass,
    width: jint,
    height: jint,
    out_pixels: JIntArray,
) {
    let w = width.max(0) as usize;
    let h = height.max(0) as usize;
    if w == 0 || h == 0 {
        return;
    }

    let len = w * h;
    let mut pixels = vec![0xFF000000u32; len];

    // Default: show I chord root (pitch class 0) as red.
    let root_pc = 0usize;
    let root_string_in_octave = layout::NOTE_TO_STRING_IN_OCTAVE[root_pc] as usize;

    for (string_idx, x) in layout::compute_string_positions(w as f32).enumerate() {
        let xi = x.round() as i32;
        if xi < 0 || xi >= w as i32 {
            continue;
        }
        let xi = xi as usize;

        let color = if (string_idx % 7) == root_string_in_octave {
            0xFFFF0000
        } else {
            0xFFFFFFFF
        };

        for y in 0..h {
            pixels[y * w + xi] = color;
        }
    }

    // Convert u32 ARGB -> i32 as expected by Java int[]
    let pixels_i32: Vec<i32> = pixels.into_iter().map(|p| p as i32).collect();
    let _ = env.set_int_array_region(out_pixels, 0, &pixels_i32);
}
