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

/// Render strings into `out_pixels` (ARGB_8888) based on the current active chord.
#[no_mangle]
pub extern "system" fn Java_com_rustharp_app_MainActivity_rustRenderStrings(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    width: jint,
    height: jint,
    out_pixels: JIntArray,
) {
    let w = width.max(0) as usize;
    let h = height.max(0) as usize;
    if w == 0 || h == 0 {
        return;
    }

    let active_chord = if handle != 0 {
        let frontend = unsafe { &*(handle as *const AndroidFrontend) };
        *frontend.engine().active_chord()
    } else {
        None
    };

    let len = w * h;
    let mut pixels = vec![0xFF000000i32; len];

    let positions = layout::compute_note_positions(w as f32);
    for (i, x) in positions.iter().enumerate() {
        let uknote = crate::notes::UnkeyedNote(i as i16);
        let xi = x.round() as i32;
        if xi < 0 || xi >= w as i32 {
            continue;
        }
        let xi = xi as usize;

        let color = if let Some(chord) = active_chord {
            if chord.has_root(uknote) {
                0x00FF0000i32 // red
            } else if chord.contains(uknote) {
                0x00FFFFFFi32 // white
            } else {
                0x00333333i32 // dim gray
            }
        } else {
            0x00333333i32
        };

        for y in 0..h {
            pixels[y * w + xi] = color;
        }
    }

    let _ = env.set_int_array_region(out_pixels, 0, &pixels);
}
