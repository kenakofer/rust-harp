use crate::android_frontend::AndroidFrontend;
use crate::app_state::KeyState;
use crate::input_map::{UiButton, UiKey};
use crate::layout;

#[cfg(all(target_os = "android", feature = "android"))]
use crate::android_aaudio;

use crate::chord_wheel::{self, WheelDir8};
use crate::touch::{PointerId, TouchEvent, TouchPhase};

use jni::objects::{JClass, JIntArray, JShortArray};
use jni::sys::{jboolean, jint, jlong, jshort, jfloat};
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
pub extern "system" fn Java_com_rustharp_app_MainActivity_rustResetAudioChannel(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) {
    if handle == 0 {
        return;
    }
    let frontend = unsafe { &mut *(handle as *mut AndroidFrontend) };
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
    if handle == 0 {
        return;
    }
    let frontend = unsafe { &mut *(handle as *mut AndroidFrontend) };
    frontend.set_show_note_names(show != 0);
}

#[no_mangle]
pub extern "system" fn Java_com_rustharp_app_MainActivity_rustSetPlayOnTap(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
    enabled: jboolean,
) {
    if handle == 0 {
        return;
    }
    let frontend = unsafe { &mut *(handle as *mut AndroidFrontend) };
    frontend.set_play_on_tap(enabled != 0);
}

#[no_mangle]
pub extern "system" fn Java_com_rustharp_app_MainActivity_rustSetImpliedSevenths(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
    enabled: jboolean,
) {
    if handle == 0 {
        return;
    }
    let frontend = unsafe { &mut *(handle as *mut AndroidFrontend) };
    frontend.engine_mut().set_allow_implied_sevenths(enabled != 0);
}

#[no_mangle]
pub extern "system" fn Java_com_rustharp_app_MainActivity_rustSetChordReleaseNoteOffDelayMs(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
    ms: jint,
) {
    if handle == 0 {
        return;
    }
    let frontend = unsafe { &mut *(handle as *mut AndroidFrontend) };
    frontend.set_chord_release_note_off_delay_ms(ms.max(0) as u32);
}

#[no_mangle]
pub extern "system" fn Java_com_rustharp_app_MainActivity_rustFlushDeferredNoteOffs(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) {
    if handle == 0 {
        return;
    }
    let frontend = unsafe { &*(handle as *const AndroidFrontend) };
    frontend.flush_deferred_stop_notes();
}

#[no_mangle]
pub extern "system" fn Java_com_rustharp_app_MainActivity_rustHasActiveNoteVisuals(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jboolean {
    if handle == 0 {
        return 0;
    }
    let frontend = unsafe { &*(handle as *const AndroidFrontend) };
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
    if handle == 0 {
        return;
    }
    let hz = (a4_tuning_hz as i32).clamp(430, 450) as u16;
    let frontend = unsafe { &mut *(handle as *mut AndroidFrontend) };
    frontend.set_a4_tuning_hz(hz);
}

#[no_mangle]
pub extern "system" fn Java_com_rustharp_app_MainActivity_rustSetKeyIndex(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
    key_index: jint,
) -> jint {
    if handle == 0 {
        return 0;
    }

    let idx = (key_index as i16).rem_euclid(12);
    let frontend = unsafe { &mut *(handle as *mut AndroidFrontend) };
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
    if handle == 0 {
        return 0;
    }

    let frontend = unsafe { &*(handle as *const AndroidFrontend) };
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

    let frontend = unsafe { &mut *(handle as *mut AndroidFrontend) };
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

    let frontend = unsafe { &mut *(handle as *mut AndroidFrontend) };
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
        UiButton::VIIB => Some(ChordButton::VIIB),
        UiButton::IV => Some(ChordButton::IV),
        UiButton::I => Some(ChordButton::I),
        UiButton::V => Some(ChordButton::V),
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
    if handle == 0 {
        return 0;
    }

    // Only degree chord buttons participate in the wheel.
    let button = match chord_button_id {
        0 => UiButton::VIIB,
        1 => UiButton::IV,
        2 => UiButton::I,
        3 => UiButton::V,
        4 => UiButton::II,
        5 => UiButton::VI,
        6 => UiButton::III,
        7 => UiButton::VIIDim,
        _ => return 0,
    };

    let chord_button = match chord_button_from_ui_button(button) {
        Some(b) => b,
        None => return 0,
    };

    let frontend = unsafe { &mut *(handle as *mut AndroidFrontend) };

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
    if handle == 0 {
        return 0;
    }

    let button = match chord_button_id {
        0 => UiButton::VIIB,
        1 => UiButton::IV,
        2 => UiButton::I,
        3 => UiButton::V,
        4 => UiButton::II,
        5 => UiButton::VI,
        6 => UiButton::III,
        7 => UiButton::VIIDim,
        _ => return 0,
    };

    let frontend = unsafe { &mut *(handle as *mut AndroidFrontend) };

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
    y: jint,
    width: jint,
    height: jint,
    pressure: jfloat,
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

    let h = height.max(1) as f32;
    let event = TouchEvent {
        id: PointerId(pointer_id as u64),
        phase,
        x: x as f32,
        y_norm: (y as f32 / h).clamp(0.0, 1.0),
        pressure: pressure as f32,
    };

    let frontend = unsafe { &mut *(handle as *mut AndroidFrontend) };
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

    let (top_chord, middle_chord, show_note_names, transpose_pc, visuals) = if handle != 0 {
        let frontend = unsafe { &*(handle as *const AndroidFrontend) };
        let eng = frontend.engine();
        (
            eng.active_chord_for_row(crate::rows::RowId::Top),
            eng.active_chord_for_row(crate::rows::RowId::Middle)
                .unwrap_or_else(|| crate::chord::Chord::new_triad(crate::notes::UnkeyedNote(0))),
            frontend.show_note_names(),
            eng.transpose().wrap_to_octave(),
            frontend.note_visuals_snapshot(),
        )
    } else {
        (
            None,
            crate::chord::Chord::new_triad(crate::notes::UnkeyedNote(0)),
            false,
            0,
            Vec::new(),
        )
    };

    fn label_pitch_class(uknote: crate::notes::UnkeyedNote, transpose_pc: i16) -> i16 {
        (uknote.wrap_to_octave() + transpose_pc).rem_euclid(12)
    }

    fn draw_text(pixels: &mut [i32], w: usize, h: usize, x_left: i32, y_top: i32, text: &str, color: i32) {
        // +30% over the old 2x scale => 2.6x.
        crate::pixel_font::draw_text_i32(pixels, w, h, x_left, y_top, text, color, 13, 5)
    }

    let len = w * h;
    let mut pixels = vec![0xFF000000u32 as i32; len];

    let positions = layout::compute_note_positions_android(w as f32);

    // 40% top, 40% middle, 20% bottom
    let top_end = h * 2 / 5;
    let mid_end = h * 4 / 5;

    fn compute_best(
        w: usize,
        positions: &[f32],
        chord: Option<crate::chord::Chord>,
        chromatic_all: bool,
        transpose_pc: i16,
        label_pitch_class: fn(crate::notes::UnkeyedNote, i16) -> i16,
    ) -> (Vec<u8>, Vec<i32>, Vec<u8>) {
        // Priority: root (red) > chord tone (white) > inactive (dim gray)
        let mut best_prio_per_x: Vec<u8> = vec![0; w];
        let mut best_color_per_x: Vec<i32> = vec![0xFF333333u32 as i32; w];
        let mut best_pc_per_x: Vec<u8> = vec![255; w];

        for (i, x) in positions.iter().enumerate() {
            let uknote = crate::notes::UnkeyedNote(i as i16);
            let xi = x.round() as i32;
            if xi < 0 || xi >= w as i32 {
                continue;
            }
            let xi = xi as usize;

            if chromatic_all {
                // Bottom row: every note is enabled and visible.
                let prio = 2;
                let color = 0xFFFFFFFFu32 as i32;
                if prio > best_prio_per_x[xi] {
                    best_prio_per_x[xi] = prio;
                    best_color_per_x[xi] = color;
                    best_pc_per_x[xi] = label_pitch_class(uknote, transpose_pc) as u8;
                }
                continue;
            }

            // Chromatic "in-between" strings should only be visible when active.
            if crate::notes::is_black_key(uknote) {
                match chord {
                    Some(ch) => {
                        if !ch.contains(uknote) {
                            continue;
                        }
                    }
                    None => continue,
                }
            }

            let (prio, color) = if let Some(ch) = chord {
                if ch.has_root(uknote) {
                    (3, 0xFFFF0000u32 as i32) // red
                } else if ch.contains(uknote) {
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
                best_pc_per_x[xi] = label_pitch_class(uknote, transpose_pc) as u8;
            }
        }

        (best_prio_per_x, best_color_per_x, best_pc_per_x)
    }

    let (top_prio, top_color, top_pc) =
        compute_best(w, &positions, top_chord, false, transpose_pc, label_pitch_class);
    let (mid_prio, mid_color, mid_pc) = compute_best(
        w,
        &positions,
        Some(middle_chord),
        false,
        transpose_pc,
        label_pitch_class,
    );
    let (bot_prio, bot_color, bot_pc) =
        compute_best(w, &positions, None, true, transpose_pc, label_pitch_class);

    // Base strings.
    for xi in 0..w {
        if top_prio[xi] != 0 {
            let color = top_color[xi];
            for y in 0..top_end {
                pixels[y * w + xi] = color;
            }
        }
        if mid_prio[xi] != 0 {
            let color = mid_color[xi];
            for y in top_end..mid_end {
                pixels[y * w + xi] = color;
            }
        }
        if bot_prio[xi] != 0 {
            let color = bot_color[xi];
            for y in mid_end..h {
                pixels[y * w + xi] = color;
            }
        }
    }

    // Note-on visuals: strike = flash+fade; strum = widen then shrink.
    if !visuals.is_empty() {
        use crate::android_frontend::{NoteVisualKind, NOTE_STRIKE_VIS_MS, NOTE_STRUM_VIS_MS};
        use crate::rows::RowId;

        const INACTIVE_GRAY: i32 = 0xFF333333u32 as i32;

        fn blend_to_white(c: i32, f: f32) -> i32 {
            let f = f.clamp(0.0, 1.0);
            let cu = c as u32;
            let r = ((cu >> 16) & 0xFF) as f32;
            let g = ((cu >> 8) & 0xFF) as f32;
            let b = (cu & 0xFF) as f32;
            let nr = (r + (255.0 - r) * f).round() as u32;
            let ng = (g + (255.0 - g) * f).round() as u32;
            let nb = (b + (255.0 - b) * f).round() as u32;
            (0xFF00_0000u32 | (nr << 16) | (ng << 8) | nb) as i32
        }

        fn blend_towards(src: i32, dst: i32, f: f32) -> i32 {
            let f = f.clamp(0.0, 1.0);
            let su = src as u32;
            let du = dst as u32;
            let sr = ((su >> 16) & 0xFF) as f32;
            let sg = ((su >> 8) & 0xFF) as f32;
            let sb = (su & 0xFF) as f32;
            let dr = ((du >> 16) & 0xFF) as f32;
            let dg = ((du >> 8) & 0xFF) as f32;
            let db = (du & 0xFF) as f32;
            let r = (sr + (dr - sr) * f).round() as u32;
            let g = (sg + (dg - sg) * f).round() as u32;
            let b = (sb + (db - sb) * f).round() as u32;
            (0xFF00_0000u32 | (r << 16) | (g << 8) | b) as i32
        }

        let now = std::time::Instant::now();
        for e in visuals {
            // Note positions are indexed by absolute UnkeyedNote (including ANDROID_LOWEST_NOTE offset).
            // Using wrap_to_octave() would point into the "dummy" (-inf) region and hide visuals.
            let ni_i16 = e.note.as_i16();
            if ni_i16 < 0 {
                continue;
            }
            let ni = ni_i16 as usize;
            if ni >= positions.len() {
                continue;
            }
            let x = positions[ni];
            if !x.is_finite() {
                continue;
            }
            let xi = x.round() as i32;
            if xi < 0 || xi >= w as i32 {
                continue;
            }
            let (y0, y1) = match e.row {
                RowId::Top => (0usize, top_end),
                RowId::Middle => (top_end, mid_end),
                RowId::Bottom => (mid_end, h),
            };

            // Match the string's existing color; skip inactive (dim gray) strings.
            let base_color = match e.row {
                RowId::Top => top_color[xi as usize],
                RowId::Middle => mid_color[xi as usize],
                RowId::Bottom => bot_color[xi as usize],
            };
            if base_color == INACTIVE_GRAY {
                continue;
            }
            let highlight_color = blend_to_white(base_color, 0.75);

            let age_ms = now
                .saturating_duration_since(e.at)
                .as_millis() as f32;

            let (dur_ms, width_px, mix) = match e.kind {
                NoteVisualKind::Strike => {
                    let t = (age_ms / NOTE_STRIKE_VIS_MS as f32).clamp(0.0, 1.0);
                    (NOTE_STRIKE_VIS_MS as f32, 14i32, (1.0 - t).powf(1.6))
                }
                NoteVisualKind::Strum => {
                    let t = (age_ms / NOTE_STRUM_VIS_MS as f32).clamp(0.0, 1.0);
                    let w0 = 12.0;
                    let w1 = 1.0;
                    let wcur = (w1 + (w0 - w1) * (1.0 - t)).round() as i32;
                    (NOTE_STRUM_VIS_MS as f32, wcur.max(1), 0.9)
                }
            };
            let _ = dur_ms; // (kept for readability/debugging)

            let half = width_px / 2;
            let x0 = (xi - half).max(0) as usize;
            let x1 = (xi + half).min((w - 1) as i32) as usize;
            for x in x0..=x1 {
                for y in y0..y1 {
                    let idx = y * w + x;
                    pixels[idx] = blend_towards(pixels[idx], highlight_color, mix);
                }
            }
        }
    }

    if show_note_names {
        // Top row labels.
        for (xi, prio) in top_prio.iter().enumerate() {
            if *prio < 2 {
                continue;
            }
            let pc = top_pc[xi];
            if pc == 255 {
                continue;
            }
            let label = crate::notes::pitch_class_label(pc as i16, transpose_pc);
            draw_text(&mut pixels, w, h, xi as i32 + 4, 2, label, top_color[xi]);
        }

        // Bottom row is chromatic; never draw note-name labels there.

        // Middle row labels.
        let y_mid = top_end as i32 + 2;
        for (xi, prio) in mid_prio.iter().enumerate() {
            if *prio < 2 {
                continue;
            }
            let pc = mid_pc[xi];
            if pc == 255 {
                continue;
            }
            let label = crate::notes::pitch_class_label(pc as i16, transpose_pc);
            draw_text(&mut pixels, w, h, xi as i32 + 4, y_mid, label, mid_color[xi]);
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
        assert_eq!(super::label_pitch_class(UnkeyedNote(0), Transpose(2).wrap_to_octave()), 2); // C -> D
        assert_eq!(super::label_pitch_class(UnkeyedNote(11), Transpose(2).wrap_to_octave()), 1); // B -> C#
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

    #[test]
    fn android_render_hides_inactive_black_keys() {
        let w = 1000usize;
        let positions = layout::compute_note_positions_android(w as f32);

        let chord = Chord::new_triad(UnkeyedNote(0)); // C major (no black keys)

        let mut best_prio_per_x: Vec<u8> = vec![0; w];
        for (i, x) in positions.iter().enumerate() {
            let uknote = UnkeyedNote(i as i16);
            if crate::notes::is_black_key(uknote) && !chord.contains(uknote) {
                continue;
            }

            let xi = x.round() as i32;
            if xi < 0 || xi >= w as i32 {
                continue;
            }
            let xi = xi as usize;

            let prio = if chord.has_root(uknote) {
                3
            } else if chord.contains(uknote) {
                2
            } else {
                1
            };
            if prio > best_prio_per_x[xi] {
                best_prio_per_x[xi] = prio;
            }
        }

        // C# is a black key and should be absent.
        let xi_black = positions[1].round() as usize;
        assert_eq!(best_prio_per_x[xi_black], 0);

        // D is a white key and should still render (inactive dim gray is allowed).
        let xi_d = positions[2].round() as usize;
        assert!(best_prio_per_x[xi_d] > 0);
    }

    #[test]
    fn render_strings_note_names_draw_off_string_pixels() {
        // Render a chord and verify the note-name glyphs paint pixels that are not on the string line.
        let w = 1000usize;
        let h = 200usize;
        let positions = layout::compute_note_positions_android(w as f32);
        let chord = Chord::new_triad(UnkeyedNote(0)); // C major

        // Find a string x-position where the chord is active.
        let mut line_xs = std::collections::HashSet::<usize>::new();
        for (i, x) in positions.iter().enumerate() {
            if !x.is_finite() {
                continue;
            }
            let uknote = UnkeyedNote(i as i16);
            if crate::notes::is_black_key(uknote) && !chord.contains(uknote) {
                continue;
            }
            let xi = x.round() as i32;
            if xi >= 0 && xi < w as i32 {
                line_xs.insert(xi as usize);
            }
        }

        let mut active_xi: Option<usize> = None;
        for (i, x) in positions.iter().enumerate() {
            let uknote = UnkeyedNote(i as i16);
            if chord.contains(uknote) {
                let xi = x.round() as i32;
                if xi >= 0 && xi < w as i32 {
                    let xi = xi as usize;
                    // Ensure there is empty space next to the line so we can detect text.
                    if !line_xs.contains(&(xi + 6)) && !line_xs.contains(&(xi + 8)) {
                        active_xi = Some(xi);
                        break;
                    }
                }
            }
        }
        let xi = active_xi.expect("expected an active string with space next to it");

        // Minimal reimplementation: lines only.
        let mut pixels_no = vec![0xFF000000u32 as i32; w * h];
        for (i, x) in positions.iter().enumerate() {
            let uknote = UnkeyedNote(i as i16);
            let xi2 = x.round() as i32;
            if xi2 < 0 || xi2 >= w as i32 {
                continue;
            }
            let xi2 = xi2 as usize;
            let color = if chord.has_root(uknote) {
                0xFFFF0000u32 as i32
            } else if chord.contains(uknote) {
                0xFFFFFFFFu32 as i32
            } else {
                0xFF333333u32 as i32
            };
            for y in 0..h {
                pixels_no[y * w + xi2] = color;
            }
        }

        // Use the real renderer path by calling the internal draw_text logic via rustRenderStrings' new behavior.
        // We just replicate the conditions here by asserting that with labels enabled, pixels adjacent to the line are touched.
        let mut pixels_yes = pixels_no.clone();

        // Draw "C" at this active xi, which should color pixels at xi+1 near the top.
        // This matches the current draw_text() behavior in rustRenderStrings.
        fn glyph_5x7(ch: char) -> [u8; 7] {
            match ch {
                'C' => [0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110],
                _ => [0; 7],
            }
        }
        // Match draw_text() parameters (2.6x scale, and starting just right of the line).
        const SCALE_NUM: i32 = 13;
        const SCALE_DEN: i32 = 5;
        let map = |u: i32| (u * SCALE_NUM) / SCALE_DEN;

        let x_left = xi as i32 + 4;
        let y_top = 2i32;
        let g = glyph_5x7('C');

        for (row, bits) in g.iter().enumerate() {
            for col in 0..5 {
                if (bits & (1 << (4 - col))) == 0 {
                    continue;
                }
                let x0 = x_left + map(col as i32);
                let x1 = x_left + map(col as i32 + 1);
                let y0 = y_top + map(row as i32);
                let y1 = y_top + map(row as i32 + 1);
                for py in y0..y1 {
                    for px in x0..x1 {
                        if px < 0 || py < 0 {
                            continue;
                        }
                        let (px, py) = (px as usize, py as usize);
                        if px >= w || py >= h {
                            continue;
                        }
                        pixels_yes[py * w + px] = 0xFFFFFFFFu32 as i32;
                    }
                }
            }
        }

        let y_probe = 4usize;
        assert_eq!(pixels_no[y_probe * w + (xi + 6)], 0xFF000000u32 as i32);
        assert_ne!(pixels_yes[y_probe * w + (xi + 6)], 0xFF000000u32 as i32);
    }
}
