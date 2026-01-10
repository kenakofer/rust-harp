use crate::android_frontend::AndroidFrontend;
use crate::app_state::KeyState;
use crate::input_map::{UiButton, UiKey};

#[cfg(all(target_os = "android", feature = "android"))]
use crate::android_aaudio;

use crate::chord_wheel::{self, WheelDir8};
use crate::touch::{PointerId, TouchEvent, TouchPhase};

use jni::objects::{JClass, JIntArray, JShortArray, JString};
use jni::sys::{jboolean, jfloat, jint, jlong};
use jni::JNIEnv;

macro_rules! frontend_mut_or_return {
    ($handle:expr, $ret:expr) => {{
        if $handle == 0 {
            return $ret;
        }
        unsafe { &mut *($handle as *mut AndroidFrontend) }
    }};
}

macro_rules! frontend_or_return {
    ($handle:expr, $ret:expr) => {{
        if $handle == 0 {
            return $ret;
        }
        unsafe { &*($handle as *const AndroidFrontend) }
    }};
}

mod audio;
mod render;

/// Simple JNI hook so an Android Activity can verify the Rust library loads.
#[no_mangle]
pub extern "system" fn Java_com_rustharp_app_MainActivity_rustInit(
    _env: JNIEnv,
    _class: JClass,
) -> jint {
    1
}

#[no_mangle]
pub extern "system" fn Java_com_rustharp_app_MainActivity_rustGetUiButtonsCount(
    _env: JNIEnv,
    _class: JClass,
) -> jint {
    UiButton::COUNT as jint
}

#[no_mangle]
pub extern "system" fn Java_com_rustharp_app_MainActivity_rustGetUiButtonIndex(
    mut env: JNIEnv,
    _class: JClass,
    id: JString,
) -> jint {
    let id: String = match env.get_string(&id) {
        Ok(s) => s.into(),
        Err(_) => return -1,
    };

    UiButton::from_id(&id)
        .map(|b| b.index() as jint)
        .unwrap_or(-1)
}

#[no_mangle]
pub extern "system" fn Java_com_rustharp_app_MainActivity_rustStartAAudio(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jboolean {
    let frontend = frontend_mut_or_return!(handle, 0);
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
pub extern "system" fn Java_com_rustharp_app_MainActivity_rustResetAudioChannel(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) {
    let frontend = frontend_mut_or_return!(handle, ());
    frontend.reset_audio_channel();
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
    let frontend = frontend_mut_or_return!(handle, ());
    frontend.set_show_note_names(show != 0);
}

#[no_mangle]
pub extern "system" fn Java_com_rustharp_app_MainActivity_rustSetPlayOnTap(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
    enabled: jboolean,
) {
    let frontend = frontend_mut_or_return!(handle, ());
    frontend.set_play_on_tap(enabled != 0);
}

#[no_mangle]
pub extern "system" fn Java_com_rustharp_app_MainActivity_rustSetImpliedSevenths(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
    enabled: jboolean,
) {
    let frontend = frontend_mut_or_return!(handle, ());
    frontend
        .engine_mut()
        .set_allow_implied_sevenths(enabled != 0);
}

#[no_mangle]
pub extern "system" fn Java_com_rustharp_app_MainActivity_rustSetWheelModifiers(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
    modifiers_bits: jint,
) {
    let frontend = frontend_mut_or_return!(handle, ());
    let mods = crate::chord::Modifiers::from_bits_truncate(modifiers_bits as u16);
    frontend.engine_mut().set_wheel_modifiers(mods);
}

#[no_mangle]
pub extern "system" fn Java_com_rustharp_app_MainActivity_rustSetChordReleaseNoteOffDelayMs(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
    ms: jint,
) {
    let frontend = frontend_mut_or_return!(handle, ());
    frontend.set_chord_release_note_off_delay_ms(ms.max(0) as u32);
}

#[no_mangle]
pub extern "system" fn Java_com_rustharp_app_MainActivity_rustFlushDeferredNoteOffs(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) {
    let frontend = frontend_or_return!(handle, ());
    frontend.flush_deferred_stop_notes();
}

#[no_mangle]
pub extern "system" fn Java_com_rustharp_app_MainActivity_rustGetActiveChord(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jlong {
    let frontend = frontend_or_return!(handle, 0);
    let chord = frontend.engine().active_chord();
    let transpose = frontend.engine().transpose().wrap_to_octave();
    let root_pc = (chord.get_root().wrap_to_octave() + transpose).rem_euclid(12);
    let mods_bits = chord.mods.bits();
    
    // Pack root (lower 16 bits) and modifiers (upper 16 bits) into a single jlong.
    ((mods_bits as i64) << 16) | (root_pc as i64)
}

#[no_mangle]
pub extern "system" fn Java_com_rustharp_app_MainActivity_rustHasActiveNoteVisuals(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jboolean {
    let frontend = frontend_or_return!(handle, 0);
    if frontend.has_active_note_visuals() {
        1
    } else {
        0
    }
}

#[no_mangle]
pub extern "system" fn Java_com_rustharp_app_MainActivity_rustSetA4TuningHz(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
    a4_tuning_hz: jint,
) {
    let frontend = frontend_mut_or_return!(handle, ());
    let hz = (a4_tuning_hz as i32).clamp(430, 450) as u16;
    frontend.set_a4_tuning_hz(hz);
}

#[no_mangle]
pub extern "system" fn Java_com_rustharp_app_MainActivity_rustSetKeyIndex(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
    key_index: jint,
) -> jint {
    let frontend = frontend_mut_or_return!(handle, 0);

    let idx = (key_index as i16).rem_euclid(12);
    let effects = frontend.handle_ui_event(crate::ui_events::UiEvent::SetTranspose(
        crate::notes::Transpose(idx),
    ));
    let redraw = effects.redraw;
    let has_play = !effects.play_notes.is_empty() || !effects.stop_notes.is_empty();

    frontend.push_effects(effects);

    (if redraw { 1 } else { 0 }) | (if has_play { 2 } else { 0 })
}

#[no_mangle]
pub extern "system" fn Java_com_rustharp_app_MainActivity_rustGetKeyIndex(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jint {
    let frontend = frontend_or_return!(handle, 0);
    frontend.engine().transpose().wrap_to_octave() as jint
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
    let frontend = frontend_mut_or_return!(handle, 0);

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
            61 => UiKey::Tab,            // KEYCODE_TAB
            113 | 114 => UiKey::Control, // KEYCODE_CTRL_LEFT / KEYCODE_CTRL_RIGHT
            _ => return 0,
        }
    };

    let effects = frontend.handle_ui_event(crate::ui_events::UiEvent::Key { state, key });
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
    let frontend = frontend_mut_or_return!(handle, 0);

    let state = if is_down != 0 {
        KeyState::Pressed
    } else {
        KeyState::Released
    };

    let button = match UiButton::from_index(button_id as usize) {
        Some(b) => b,
        None => return 0,
    };

    let is_chord_button = chord_button_from_ui_button(button).is_some();

    if is_chord_button && state == KeyState::Pressed {
        frontend.set_chord_hold_active(true);
    }

    let mut effects = frontend.handle_ui_event(crate::ui_events::UiEvent::Button { state, button });

    if is_chord_button {
        // Suppress chord-change stop-notes while selecting a chord; we'll release them once the
        // chord button is released and the double-tap window has expired.
        if frontend.chord_hold_active() || state == KeyState::Released {
            frontend.defer_stop_notes(std::mem::take(&mut effects.stop_notes));
        }

        if state == KeyState::Released {
            frontend.set_chord_hold_active(false);
            frontend.arm_deferred_stop_notes();
        }
    }

    let redraw = effects.redraw;
    let has_play = !effects.play_notes.is_empty() || !effects.stop_notes.is_empty();
    frontend.push_effects(effects);

    (if redraw { 1 } else { 0 }) | (if has_play { 2 } else { 0 })
}

fn chord_button_from_ui_button(button: UiButton) -> Option<crate::app_state::ChordButton> {
    use crate::app_state::ChordButton;
    match button {
        UiButton::V => Some(ChordButton::V),
        UiButton::I => Some(ChordButton::I),
        UiButton::IV => Some(ChordButton::IV),
        UiButton::VIIB => Some(ChordButton::VIIB),
        UiButton::II => Some(ChordButton::II),
        UiButton::VI => Some(ChordButton::VI),
        UiButton::III => Some(ChordButton::III),
        UiButton::VIIDim => Some(ChordButton::VII),
        UiButton::Hept => Some(ChordButton::HeptatonicMajor),
        _ => None,
    }
}

#[no_mangle]
pub extern "system" fn Java_com_rustharp_app_MainActivity_rustApplyChordWheelChoice(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
    chord_button_id: jint,
    dir8: jint,
) -> jint {
    let frontend = frontend_mut_or_return!(handle, 0);

    // Only degree chord buttons participate in the wheel.
    let button = match UiButton::from_index(chord_button_id as usize) {
        Some(b) => b,
        None => return 0,
    };

    let chord_button = match chord_button_from_ui_button(button) {
        Some(b) => b,
        None => return 0,
    };

    // The Java chord-wheel UI drives chord presses via this JNI call (it does not call rustHandleUiButton(true)).
    // Mark the chord as held so chord-change note-offs can be deferred until release + double-tap timeout.
    frontend.set_chord_hold_active(true);

    let mods = if dir8 < 0 {
        crate::chord::Modifiers::empty()
    } else {
        let dir = match WheelDir8::from_i32(dir8) {
            Some(d) => d,
            None => return 0,
        };
        chord_wheel::modifiers_for(chord_button, dir)
    };

    frontend.engine_mut().set_wheel_modifiers(mods);

    // Trigger a recompute immediately (while the chord button is still held).
    let mut effects = frontend.handle_ui_event(crate::ui_events::UiEvent::Button {
        state: KeyState::Pressed,
        button,
    });

    let redraw = effects.redraw;
    let has_play = !effects.play_notes.is_empty() || !effects.stop_notes.is_empty();

    // Defer chord-change note-offs while the chord wheel is active.
    if frontend.chord_hold_active() {
        frontend.defer_stop_notes(std::mem::take(&mut effects.stop_notes));
    }

    frontend.push_effects(effects);

    (if redraw { 1 } else { 0 }) | (if has_play { 2 } else { 0 })
}

#[no_mangle]
pub extern "system" fn Java_com_rustharp_app_MainActivity_rustToggleChordWheelMinorMajor(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
    chord_button_id: jint,
) -> jint {
    let frontend = frontend_mut_or_return!(handle, 0);

    let button = match UiButton::from_index(chord_button_id as usize) {
        Some(b) => b,
        None => return 0,
    };

    // Same as rustApplyChordWheelChoice: Java chord-wheel toggles happen while the button is logically held.
    frontend.set_chord_hold_active(true);

    frontend.engine_mut().toggle_wheel_minor_major();

    let mut effects = frontend.handle_ui_event(crate::ui_events::UiEvent::Button {
        state: KeyState::Pressed,
        button,
    });

    let redraw = effects.redraw;
    let has_play = !effects.play_notes.is_empty() || !effects.stop_notes.is_empty();

    // Defer chord-change note-offs while the chord wheel is active.
    if frontend.chord_hold_active() {
        frontend.defer_stop_notes(std::mem::take(&mut effects.stop_notes));
    }

    frontend.push_effects(effects);

    (if redraw { 1 } else { 0 }) | (if has_play { 2 } else { 0 })
}

#[no_mangle]
pub extern "system" fn Java_com_rustharp_app_MainActivity_rustGetUiButtonsMask(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jint {
    let frontend = frontend_or_return!(handle, 0);

    let eng = frontend.engine();

    let mut mask: u32 = 0;
    for b in UiButton::ORDER {
        if ui_button_down(eng, b) {
            mask |= 1 << b.index();
        }
    }

    mask as jint
}

fn ui_button_down(eng: &crate::engine::Engine, b: UiButton) -> bool {
    use crate::app_state::{ChordButton, ModButton};

    match b {
        UiButton::V => eng.chord_button_down(ChordButton::V),
        UiButton::I => eng.chord_button_down(ChordButton::I),
        UiButton::IV => eng.chord_button_down(ChordButton::IV),
        UiButton::VIIB => eng.chord_button_down(ChordButton::VIIB),
        UiButton::II => eng.chord_button_down(ChordButton::II),
        UiButton::VI => eng.chord_button_down(ChordButton::VI),
        UiButton::III => eng.chord_button_down(ChordButton::III),
        UiButton::VIIDim => eng.chord_button_down(ChordButton::VII),
        UiButton::Hept => eng.chord_button_down(ChordButton::HeptatonicMajor),

        UiButton::Maj7 => eng.mod_button_down(ModButton::Major7),
        UiButton::No3 => eng.mod_button_down(ModButton::No3),
        UiButton::Sus4 => eng.mod_button_down(ModButton::Sus4),
        UiButton::MinorMajor => eng.mod_button_down(ModButton::MinorMajor),
        UiButton::Add2 => eng.mod_button_down(ModButton::Major2),
        UiButton::Add7 => eng.mod_button_down(ModButton::Minor7),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_rustharp_app_MainActivity_rustHandleTouch(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
    pointer_id: jlong,
    phase: jint,
    x: jint,
    y: jint,
    width: jint,
    height: jint,
    pressure: jfloat,
) -> jint {
    let frontend = frontend_mut_or_return!(handle, 0);

    let phase = match phase {
        0 => TouchPhase::Down,
        1 => TouchPhase::Move,
        2 => TouchPhase::Up,
        _ => TouchPhase::Cancel,
    };

    let h = height.max(1) as f32;
    let event = TouchEvent {
        id: PointerId(pointer_id as u64),
        phase,
        x: x as f32,
        y_norm: (y as f32 / h).clamp(0.0, 1.0),
        pressure: pressure as f32,
    };

    let (effects, haptic) = frontend.handle_touch(event, width.max(1) as f32);
    let redraw = effects.redraw;
    let has_play = !effects.play_notes.is_empty() || !effects.stop_notes.is_empty();
    frontend.push_effects(effects);

    // Bit 0: needs redraw
    // Bit 1: has play notes
    // Bit 2: haptic pulse
    let wants_anim = frontend.has_active_note_visuals();
    (if redraw || wants_anim { 1 } else { 0 })
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
    let frontend = frontend_or_return!(handle, ());
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
    audio::fill_audio(env, handle, frames, out_pcm)
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
    render::render_strings(env, handle, width, height, out_pixels);
}

#[cfg(test)]
mod render_tests {
    use super::*;
    use crate::chord::Chord;
    use crate::layout;
    use crate::notes::UnkeyedNote;

    #[test]
    fn pitch_class_label_prefers_flats_in_flat_keys() {
        // Key E (4): prefer sharps.
        assert_eq!(crate::notes::pitch_class_label(8, 4), "G#");
        assert_eq!(crate::notes::pitch_class_label(1, 4), "C#");

        // Key Db (1): prefer flats.
        assert_eq!(crate::notes::pitch_class_label(8, 1), "Ab");
        assert_eq!(crate::notes::pitch_class_label(1, 1), "Db");
        assert_eq!(crate::notes::pitch_class_label(6, 1), "Gb");
    }

    #[test]
    fn label_pitch_class_applies_transpose() {
        use crate::notes::{Transpose, UnkeyedNote};
        assert_eq!(
            super::label_pitch_class(UnkeyedNote(0), Transpose(2).wrap_to_octave()),
            2
        ); // C -> D
        assert_eq!(
            super::label_pitch_class(UnkeyedNote(11), Transpose(2).wrap_to_octave()),
            1
        ); // B -> C#
    }

    #[test]
    fn android_layout_midpoints_do_not_duplicate_pixel_columns() {
        let w = 1000usize;
        let positions = layout::compute_note_positions_android(w as f32);

        let mut seen = std::collections::HashSet::<i32>::new();
        for x in &positions {
            if !x.is_finite() {
                continue;
            }
            let xi = x.round() as i32;
            assert!(seen.insert(xi), "duplicate rounded x={xi}");
        }
    }
}
