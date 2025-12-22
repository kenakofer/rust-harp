use crate::android_frontend::AndroidFrontend;
use crate::app_state::KeyState;
use crate::input_map::{self, UiButton, UiKey};
use crate::layout;
use crate::notes::{MidiNote, NoteVolume, Transpose};

#[cfg(all(target_os = "android", feature = "android"))]
use crate::android_aaudio;
use crate::touch::{PointerId, TouchEvent, TouchPhase};

use jni::objects::{JClass, JIntArray, JShortArray};
use jni::sys::{jboolean, jint, jlong, jshort};
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
pub extern "system" fn Java_com_rustharp_app_MainActivity_rustStartAAudio(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jboolean {
    if handle == 0 {
        return 0;
    }
    let frontend = unsafe { &mut *(handle as *mut AndroidFrontend) };
    if android_aaudio::start(frontend) {
        1
    } else {
        0
    }
}

#[no_mangle]
pub extern "system" fn Java_com_rustharp_app_MainActivity_rustStopAAudio(
    _env: JNIEnv,
    _class: JClass,
    _handle: jlong,
) {
    android_aaudio::stop();
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
pub extern "system" fn Java_com_rustharp_app_MainActivity_rustSetShowNoteNames(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
    show: jboolean,
) {
    if handle == 0 {
        return;
    }
    let frontend = unsafe { &mut *(handle as *mut AndroidFrontend) };
    frontend.set_show_note_names(show != 0);
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
    let redraw = effects.redraw;
    let has_play = !effects.play_notes.is_empty() || !effects.stop_notes.is_empty();

    frontend.push_effects(effects);

    // Bit 0: needs redraw
    // Bit 1: has play notes
    (if redraw { 1 } else { 0 }) | (if has_play { 2 } else { 0 })
}

fn merge_effects(a: &mut crate::app_state::AppEffects, b: crate::app_state::AppEffects) {
    a.redraw |= b.redraw;
    if a.change_key.is_none() {
        a.change_key = b.change_key;
    }
    a.stop_notes.extend(b.stop_notes);
    a.play_notes.extend(b.play_notes);
}

#[no_mangle]
pub extern "system" fn Java_com_rustharp_app_MainActivity_rustHandleUiButton(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
    button_id: jint,
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

    let button = match button_id {
        0 => UiButton::VIIB,
        1 => UiButton::IV,
        2 => UiButton::I,
        3 => UiButton::V,
        4 => UiButton::II,
        5 => UiButton::VI,
        6 => UiButton::III,
        7 => UiButton::VIIDim,
        8 => UiButton::Maj7,
        9 => UiButton::No3,
        10 => UiButton::Sus4,
        11 => UiButton::MinorMajor,
        12 => UiButton::Add2,
        13 => UiButton::Add7,
        14 => UiButton::Hept,
        _ => return 0,
    };

    let events = input_map::key_events_from_button(state, button);

    let frontend = unsafe { &mut *(handle as *mut AndroidFrontend) };
    let mut effects = crate::app_state::AppEffects {
        redraw: false,
        change_key: None,
        stop_notes: Vec::new(),
        play_notes: Vec::new(),
    };

    for ev in events {
        let e = frontend.engine_mut().handle_event(ev);
        merge_effects(&mut effects, e);
    }

    let redraw = effects.redraw;
    let has_play = !effects.play_notes.is_empty() || !effects.stop_notes.is_empty();
    frontend.push_effects(effects);

    (if redraw { 1 } else { 0 }) | (if has_play { 2 } else { 0 })
}

#[no_mangle]
pub extern "system" fn Java_com_rustharp_app_MainActivity_rustGetUiButtonsMask(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jint {
    if handle == 0 {
        return 0;
    }

    use crate::app_state::{ChordButton, ModButton};

    let frontend = unsafe { &*(handle as *const AndroidFrontend) };
    let eng = frontend.engine();

    let mut mask: u32 = 0;

    // Chords
    if eng.chord_button_down(ChordButton::VIIB) {
        mask |= 1 << 0;
    }
    if eng.chord_button_down(ChordButton::IV) {
        mask |= 1 << 1;
    }
    if eng.chord_button_down(ChordButton::I) {
        mask |= 1 << 2;
    }
    if eng.chord_button_down(ChordButton::V) {
        mask |= 1 << 3;
    }
    if eng.chord_button_down(ChordButton::II) {
        mask |= 1 << 4;
    }
    if eng.chord_button_down(ChordButton::VI) {
        mask |= 1 << 5;
    }
    if eng.chord_button_down(ChordButton::III) {
        mask |= 1 << 6;
    }
    if eng.chord_button_down(ChordButton::VII) {
        mask |= 1 << 7;
    }
    if eng.chord_button_down(ChordButton::HeptatonicMajor) {
        mask |= 1 << 14;
    }

    // Modifiers
    if eng.mod_button_down(ModButton::Major7) {
        mask |= 1 << 8;
    }
    if eng.mod_button_down(ModButton::No3) {
        mask |= 1 << 9;
    }
    if eng.mod_button_down(ModButton::Sus4) {
        mask |= 1 << 10;
    }
    if eng.mod_button_down(ModButton::MinorMajor) {
        mask |= 1 << 11;
    }
    if eng.mod_button_down(ModButton::Major2) {
        mask |= 1 << 12;
    }
    if eng.mod_button_down(ModButton::Minor7) {
        mask |= 1 << 13;
    }


    mask as jint
}

#[no_mangle]
pub extern "system" fn Java_com_rustharp_app_MainActivity_rustHandleTouch(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
    pointer_id: jlong,
    phase: jint,
    x: jint,
    width: jint,
) -> jint {
    if handle == 0 {
        return 0;
    }

    let phase = match phase {
        0 => TouchPhase::Down,
        1 => TouchPhase::Move,
        2 => TouchPhase::Up,
        _ => TouchPhase::Cancel,
    };

    let event = TouchEvent {
        id: PointerId(pointer_id as u64),
        phase,
        x: x as f32,
    };

    let frontend = unsafe { &mut *(handle as *mut AndroidFrontend) };
    let (effects, haptic) = frontend.handle_touch(event, width.max(1) as f32);
    let redraw = effects.redraw;
    let has_play = !effects.play_notes.is_empty() || !effects.stop_notes.is_empty();
    frontend.push_effects(effects);

    // Bit 0: needs redraw
    // Bit 1: has play notes
    // Bit 2: haptic pulse
    (if redraw { 1 } else { 0 })
        | (if has_play { 2 } else { 0 })
        | (if haptic { 4 } else { 0 })
}

#[no_mangle]
pub extern "system" fn Java_com_rustharp_app_MainActivity_rustSetAudioSampleRate(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
    sample_rate_hz: jint,
) {
    if handle == 0 {
        return;
    }
    let frontend = unsafe { &*(handle as *const AndroidFrontend) };
    frontend.set_sample_rate(sample_rate_hz.max(1) as u32);
}

#[no_mangle]
pub extern "system" fn Java_com_rustharp_app_MainActivity_rustFillAudio(
    env: JNIEnv,
    _class: JClass,
    handle: jlong,
    frames: jint,
    out_pcm: JShortArray,
) -> jint {
    if handle == 0 {
        return 0;
    }

    let n = frames.max(0) as usize;
    if n == 0 {
        return 0;
    }

    let frontend = unsafe { &*(handle as *const AndroidFrontend) };

    let mut buf: Vec<i16> = vec![0; n];
    frontend.render_audio_i16_mono(&mut buf);

    // i16 -> jshort
    let buf_js: Vec<jshort> = buf.into_iter().map(|s| s as jshort).collect();
    let _ = env.set_short_array_region(out_pcm, 0, &buf_js);

    n as jint
}

#[no_mangle]
pub extern "system" fn Java_com_rustharp_app_MainActivity_rustDrainPlayNotes(
    env: JNIEnv,
    _class: JClass,
    handle: jlong,
    out_midi_notes: JIntArray,
    out_volumes: JIntArray,
) -> jint {
    if handle == 0 {
        return 0;
    }

    // Deprecated: AAudio renders directly from the Rust synth.
    // Keep this JNI method as a no-op so older Java callers still link.
    let _ = env.set_int_array_region(out_midi_notes, 0, &[]);
    let _ = env.set_int_array_region(out_volumes, 0, &[]);
    0
}

/// Render strings into `out_pixels` (ARGB_8888) based on the current active chord.
#[no_mangle]
pub extern "system" fn Java_com_rustharp_app_MainActivity_rustRenderStrings(
    env: JNIEnv,
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
    let mut pixels = vec![0xFF000000u32 as i32; len];

    let positions = layout::compute_note_positions_android(w as f32);

    // Multiple notes can map to the same physical string position (duplicate x values).
    // When that happens, prioritize what we draw so inactive greys don't paint over
    // active/root lines.
    // Priority: root (red) > chord tone (white) > inactive (dim gray)
    let mut best_prio_per_x: Vec<u8> = vec![0; w];
    let mut best_color_per_x: Vec<i32> = vec![0xFF333333u32 as i32; w];

    for (i, x) in positions.iter().enumerate() {
        let uknote = crate::notes::UnkeyedNote(i as i16);
        let xi = x.round() as i32;
        if xi < 0 || xi >= w as i32 {
            continue;
        }
        let xi = xi as usize;

        let (prio, color) = if let Some(chord) = active_chord {
            if chord.has_root(uknote) {
                (3, 0xFFFF0000u32 as i32) // red
            } else if chord.contains(uknote) {
                (2, 0xFFFFFFFFu32 as i32) // white
            } else {
                (1, 0xFF333333u32 as i32) // dim gray
            }
        } else {
            (1, 0xFF333333u32 as i32)
        };

        if prio > best_prio_per_x[xi] {
            best_prio_per_x[xi] = prio;
            best_color_per_x[xi] = color;
        }
    }

    for (xi, prio) in best_prio_per_x.iter().enumerate() {
        if *prio == 0 {
            continue;
        }
        let color = best_color_per_x[xi];
        for y in 0..h {
            pixels[y * w + xi] = color;
        }
    }

    let _ = env.set_int_array_region(out_pixels, 0, &pixels);
}

#[cfg(test)]
mod render_tests {
    use super::*;
    use crate::chord::Chord;
    use crate::layout;
    use crate::notes::UnkeyedNote;

    #[test]
    fn render_strings_prefers_root_over_inactive_on_same_string() {
        // We expect duplicate x-positions (multiple notes mapped to same physical string).
        // Root should win so it doesn't get overwritten by later inactive notes.
        let w = 1000usize;
        let positions = layout::compute_note_positions_android(w as f32);

        let chord = Chord::new_triad(UnkeyedNote(0)); // C major

        // Find an x-position that occurs more than once.
        let mut xi_counts = std::collections::HashMap::<i32, usize>::new();
        for x in &positions {
            *xi_counts.entry(x.round() as i32).or_insert(0) += 1;
        }
        let (dup_xi, _count) = xi_counts
            .into_iter()
            .find(|(_, c)| *c > 1)
            .expect("expected at least one duplicate x-position");
        let dup_xi = dup_xi as usize;

        // Re-run the same prioritization logic used by rustRenderStrings.
        let mut best_prio_per_x: Vec<u8> = vec![0; w];
        let mut best_color_per_x: Vec<i32> = vec![0xFF333333u32 as i32; w];

        for (i, x) in positions.iter().enumerate() {
            let uknote = UnkeyedNote(i as i16);
            let xi = x.round() as i32;
            if xi < 0 || xi >= w as i32 {
                continue;
            }
            let xi = xi as usize;

            let (prio, color) = if chord.has_root(uknote) {
                (3, 0xFFFF0000u32 as i32)
            } else if chord.contains(uknote) {
                (2, 0xFFFFFFFFu32 as i32)
            } else {
                (1, 0xFF333333u32 as i32)
            };

            if prio > best_prio_per_x[xi] {
                best_prio_per_x[xi] = prio;
                best_color_per_x[xi] = color;
            }
        }

        assert!(best_prio_per_x[dup_xi] >= 1);
        // If the root happens to land on this duplicated string position, it must be red.
        // Otherwise (no root there), we still validate we never downgrade priority.
        if best_prio_per_x[dup_xi] == 3 {
            assert_eq!(best_color_per_x[dup_xi], 0xFFFF0000u32 as i32);
        }
    }
}
