use crate::layout;

use jni::objects::{JClass, JIntArray};
use jni::sys::jint;
use jni::JNIEnv;

/// Simple JNI hook so an Android Activity can verify the Rust library loads.
#[no_mangle]
pub extern "system" fn Java_com_rustharp_app_MainActivity_rustInit(
    _env: JNIEnv,
    _class: JClass,
) -> jint {
    1
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
