use crate::chord::Chord;
use crate::notes::{MidiNote, NoteVolume, Transpose, UnkeyedNote, UnmidiNote};
use crate::output_midir::MidiBackend;
use crate::strum;
use crate::touch::{PointerId, TouchEvent, TouchPhase};
use crate::rows::RowId;
use crate::ui_adapter::{self, AppAdapter};
use crate::ui_events::{UiEvent, UiSession};
use crate::ui_settings::UiAudioBackend;

#[cfg(feature = "synth")]
use crate::output_synth::SynthBackend;

#[cfg(any(target_os = "linux", target_os = "macos"))]
use midir::os::unix::VirtualOutput;

use midir::{MidiOutput, MidiOutputConnection};
use softbuffer::{Context, Surface};
use std::error::Error;
use std::num::NonZeroU32;
use std::rc::Rc;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

const MIDI_BASE_TRANSPOSE: Transpose = Transpose(48); // Add with UnmidiNote to get MidiNote. MIDI Note 48 is C3
const MICRO_CHANNEL: u8 = 3; // MIDI channel 2 (0-based)
const MICRO_PROGRAM: u8 = 115; // instrument program for micro-steps, 115 = Wood block
const MICRO_NOTE: MidiNote = MidiNote(20); // middle C for micro-step trigger
const MICRO_VELOCITY: u8 = 50; // quiet click
const MAIN_PROGRAM: u8 = 25; // Steel String Guitar (zero-based)
const MAIN_CHANNEL: u8 = 0;
const BASS_PROGRAM: u8 = 26;
const BASS_CHANNEL: u8 = 2;


pub fn run() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    // 1. Setup MIDI Output
    let midi_out = MidiOutput::new("Rust Harp Client")?;
    let mut conn_out: Option<MidiOutputConnection> = None;

    // Attempt to create virtual port on systems where that exists
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    match midi_out.create_virtual("Rust Harp Output") {
        Ok(conn) => {
            log::info!("Created virtual MIDI port: 'Rust Harp Output'");
            conn_out = Some(conn);
        }
        Err(_) => {
            // Fallback for non-ALSA environments or errors
            let midi_out = MidiOutput::new("Rust Harp Client")?;
            let ports = midi_out.ports();
            if let Some(port) = ports.first() {
                log::info!(
                    "Virtual port failed. Connecting to first available hardware port: {}",
                    midi_out.port_name(port)?
                );
                conn_out = Some(midi_out.connect(port, "Rust Harp Connection")?);
            } else {
                eprintln!("Warning: No MIDI ports found. Application will emit no sound.");
            }
        }
    }

    #[cfg(any(target_os = "windows"))]
    if let Some(port) = midi_out.ports().first() {
        log::info!(
            "Connecting to hardware MIDI port: {}",
            midi_out.port_name(port)?
        );
        conn_out = Some(midi_out.connect(port, "Rust Harp Connection")?);
    } else {
        eprintln!("Warning: No MIDI ports found. Application will emit no sound.");
    }

    let mut audio = DesktopAudio::new(MidiBackend::new(conn_out, MAIN_CHANNEL, BASS_CHANNEL));

    // If we have a connection, set the instruments
    if let Some(conn) = audio.midi_mut().conn_mut() {
        let _ = conn.send(&[0xC0 | MAIN_CHANNEL, MAIN_PROGRAM]);
        let _ = conn.send(&[0xC0 | BASS_CHANNEL, BASS_PROGRAM]);
        let _ = conn.send(&[0xC0 | MICRO_CHANNEL, MICRO_PROGRAM]);
    }

    // Setup Window
    let event_loop = EventLoop::new()?;
    let window = Rc::new(
        WindowBuilder::new()
            .with_title("Rust MIDI Harp")
            .with_inner_size(winit::dpi::LogicalSize::new(800.0, 600.0))
            .build(&event_loop)?,
    );

    // Setup Graphics Context and UX state
    let context = Context::new(window.clone()).expect("Failed to create graphics context");
    let mut surface = Surface::new(&context, window.clone()).expect("Failed to create surface");
    let mut prev_pos: Option<(f32, f32)> = None;
    let mut is_mouse_down = false;
    let mut note_positions: Vec<f32> = Vec::new();

    // App State
    let mut ui = UiSession::new();
    let mut settings = crate::ui_settings::load_desktop_settings();
    ui.set_play_on_tap(settings.play_on_tap);
    let mut show_settings = false;

    // 4. Run Event Loop
    event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Wait);

        match event {
            Event::WindowEvent { window_id, event } if window_id == window.id() => match event {
                WindowEvent::CloseRequested => {
                    audio.stop_all_notes_all_backends(ui.engine().active_notes());
                    elwt.exit();
                }

                WindowEvent::KeyboardInput { event, .. } => {
                    if let Some(ue) = ui_adapter::ui_event_from_winit(&event) {
                        let out = ui.handle(ue, &note_positions);
                        let _ = process_app_effects(
                            out.effects,
                            &mut audio,
                            settings.audio_backend,
                            Some(window.as_ref()),
                        );
                    }
                }

                WindowEvent::Resized(physical_size) => {
                    surface
                        .resize(
                            NonZeroU32::new(physical_size.width).unwrap(),
                            NonZeroU32::new(physical_size.height).unwrap(),
                        )
                        .unwrap();

                    let window_width = physical_size.width as f32;
                    recompute_note_positions(&mut note_positions, window_width);

                    draw_strings(
                        &mut surface,
                        physical_size.width,
                        physical_size.height,
                        *ui.engine().active_chord(),
                        ui.engine()
                            .active_chord_for_row(RowId::Bottom)
                            .unwrap_or_else(|| crate::chord::Chord::new_triad(UnkeyedNote(0))),
                        &note_positions,
                        settings.show_note_names,
                        ui.engine().transpose().wrap_to_octave(),
                        show_settings,
                        &settings,
                    );
                }

                WindowEvent::MouseInput { state, button, .. } => {
                    if button == winit::event::MouseButton::Left {
                        let pressed = state == winit::event::ElementState::Pressed;

                        let Some((x, y)) = prev_pos else {
                            return;
                        };

                        // Gear icon + settings panel (desktop only).
                        if pressed {
                            let (gear, panel, rows) = settings_layout(window.inner_size().width, window.inner_size().height);
                            if hit_rect(x, y, gear) {
                                show_settings = !show_settings;
                                window.request_redraw();
                                return;
                            }
                            if show_settings {
                                if hit_rect(x, y, panel) {
                                    if let Some(action) = hit_settings_rows(x, y, rows) {
                                        match action {
                                            SettingsAction::TogglePlayOnTap => {
                                                settings.play_on_tap = !settings.play_on_tap;
                                                ui.set_play_on_tap(settings.play_on_tap);
                                                crate::ui_settings::save_desktop_settings(&settings);
                                            }
                                            SettingsAction::ToggleShowNoteNames => {
                                                settings.show_note_names = !settings.show_note_names;
                                                crate::ui_settings::save_desktop_settings(&settings);
                                            }
                                            SettingsAction::ToggleShowRomanChords => {
                                                settings.show_roman_chords = !settings.show_roman_chords;
                                                crate::ui_settings::save_desktop_settings(&settings);
                                            }
                                            SettingsAction::CycleAudioBackend => {
                                                // Stop currently playing notes on the *current* backend so we don't leave
                                                // hanging notes behind when switching.
                                                let notes: Vec<UnmidiNote> = ui.engine().active_notes().collect();
                                                for n in notes {
                                                    audio.stop_note(settings.audio_backend, MIDI_BASE_TRANSPOSE + n);
                                                }

                                                settings.audio_backend = settings.audio_backend.cycle_desktop();

                                                #[cfg(feature = "synth")]
                                                if settings.audio_backend == UiAudioBackend::Synth && audio.synth.is_none() {
                                                    settings.audio_backend = UiAudioBackend::Midi;
                                                }

                                                crate::ui_settings::save_desktop_settings(&settings);
                                            }
                                        }
                                    }
                                    window.request_redraw();
                                    return;
                                }
                                // Click outside closes.
                                show_settings = false;
                                window.request_redraw();
                            }
                        }

                        is_mouse_down = pressed;

                        let phase = if pressed { TouchPhase::Down } else { TouchPhase::Up };
                        let h = window.inner_size().height.max(1) as f32;
                        let out = ui.handle(
                            UiEvent::Touch(TouchEvent {
                                id: PointerId(0),
                                phase,
                                x,
                                y_norm: (y / h).clamp(0.0, 1.0),
                                pressure: 1.0,
                            }),
                            &note_positions,
                        );
                        let _ = process_app_effects(
                            out.effects,
                            &mut audio,
                            settings.audio_backend,
                            Some(window.as_ref()),
                        );
                    }
                }

                WindowEvent::CursorMoved { position, .. } => {
                    let curr_x = position.x as f32;
                    let curr_y = position.y as f32;

                    if is_mouse_down {
                        let h = window.inner_size().height.max(1) as f32;
                        let out = ui.handle(
                            UiEvent::Touch(TouchEvent {
                                id: PointerId(0),
                                phase: TouchPhase::Move,
                                x: curr_x,
                                y_norm: (curr_y / h).clamp(0.0, 1.0),
                                pressure: 1.0,
                            }),
                            &note_positions,
                        );
                        let _ = process_app_effects(
                            out.effects,
                            &mut audio,
                            settings.audio_backend,
                            Some(window.as_ref()),
                        );
                    }

                    prev_pos = Some((curr_x, curr_y));
                }

                WindowEvent::RedrawRequested => {
                    let size = window.inner_size();
                    draw_strings(
                        &mut surface,
                        size.width,
                        size.height,
                        *ui.engine().active_chord(),
                        ui.engine()
                            .active_chord_for_row(RowId::Bottom)
                            .unwrap_or_else(|| crate::chord::Chord::new_triad(UnkeyedNote(0))),
                        &note_positions,
                        settings.show_note_names,
                        ui.engine().transpose().wrap_to_octave(),
                        show_settings,
                        &settings,
                    );
                }

                _ => {}
            },
            _ => {}
        }
    })?;

    Ok(())
}

fn recompute_note_positions(positions: &mut Vec<f32>, width: f32) {
    *positions = crate::layout::compute_note_positions(width);
}

struct DesktopAudio {
    midi: MidiBackend,
    #[cfg(feature = "synth")]
    synth: Option<SynthBackend>,
}

impl DesktopAudio {
    fn new(midi: MidiBackend) -> Self {
        #[cfg(feature = "synth")]
        let synth = SynthBackend::new().ok();

        Self {
            midi,
            #[cfg(feature = "synth")]
            synth,
        }
    }

    fn stop_note(&mut self, backend: UiAudioBackend, midi_note: MidiNote) {
        match backend {
            UiAudioBackend::Midi => self.midi.stop_note(midi_note),
            UiAudioBackend::Synth => {
                #[cfg(feature = "synth")]
                if let Some(s) = &self.synth {
                    s.stop_note(midi_note);
                    return;
                }
                self.midi.stop_note(midi_note);
            }
            _ => self.midi.stop_note(midi_note),
        }
    }

    fn play_note(&mut self, backend: UiAudioBackend, midi_note: MidiNote, volume: NoteVolume) {
        match backend {
            UiAudioBackend::Midi => self.midi.play_note(midi_note, volume),
            UiAudioBackend::Synth => {
                #[cfg(feature = "synth")]
                if let Some(s) = &self.synth {
                    s.play_note(midi_note, volume);
                    return;
                }
                self.midi.play_note(midi_note, volume);
            }
            _ => self.midi.play_note(midi_note, volume),
        }
    }

    fn stop_note_all_backends(&mut self, midi_note: MidiNote) {
        self.midi.stop_note(midi_note);
        #[cfg(feature = "synth")]
        if let Some(s) = &self.synth {
            s.stop_note(midi_note);
        }
    }

    fn stop_all_notes_all_backends(&mut self, notes: impl Iterator<Item = UnmidiNote>) {
        for n in notes {
            self.stop_note_all_backends(MIDI_BASE_TRANSPOSE + n);
        }
    }

    fn midi_available(&self) -> bool {
        self.midi.is_available()
    }

    fn midi_mut(&mut self) -> &mut MidiBackend {
        &mut self.midi
    }
}

fn process_app_effects(
    effects: crate::app_state::AppEffects,
    audio: &mut DesktopAudio,
    audio_backend: UiAudioBackend,
    window: Option<&Window>,
) -> bool {
    let played = !effects.play_notes.is_empty();

    if effects.redraw {
        if let Some(w) = window {
            w.request_redraw();
        }
    }
    if let Some(transpose) = effects.change_key {
        log::info!("Changed key: {:?}", transpose);
    }

    // IMPORTANT: stop before play so retriggering the same note doesn't immediately stop
    // the newly started note.
    for un in effects.stop_notes {
        audio.stop_note(audio_backend, MIDI_BASE_TRANSPOSE + un);
    }
    for pn in effects.play_notes {
        audio.play_note(audio_backend, MIDI_BASE_TRANSPOSE + pn.note, pn.volume);
    }

    played
}

#[allow(dead_code)]
fn check_pluck(
    x1: f32,
    x2: f32,
    audio: &mut DesktopAudio,
    audio_backend: UiAudioBackend,
    app: &mut AppAdapter,
    note_positions: &[f32],
) {
    if audio_backend == UiAudioBackend::Midi && !audio.midi_available() {
        return;
    }

    for crossing in strum::detect_crossings(x1, x2, note_positions) {
        let mut played_any = false;

        for note in crossing.notes {
            let effects = app.handle_strum_crossing(note);
            if process_app_effects(effects, audio, audio_backend, None) {
                played_any = true;
            }
        }

        if !played_any && audio_backend == UiAudioBackend::Midi {
            // Damped string sound (MIDI only)
            if let Some(conn) = audio.midi_mut().conn_mut() {
                let on = 0x90 | (MICRO_CHANNEL & 0x0F);
                let off = 0x80 | (MICRO_CHANNEL & 0x0F);
                let _ = conn.send(&[on, MICRO_NOTE.0, MICRO_VELOCITY]);
                let _ = conn.send(&[off, MICRO_NOTE.0, 0]);
            }
        }
    }
}


#[derive(Clone, Copy)]
struct RectI32 {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

fn hit_rect(x: f32, y: f32, r: RectI32) -> bool {
    let (x, y) = (x.round() as i32, y.round() as i32);
    x >= r.x && x < r.x + r.w && y >= r.y && y < r.y + r.h
}

#[derive(Clone, Copy, Debug)]
enum SettingsAction {
    TogglePlayOnTap,
    ToggleShowNoteNames,
    ToggleShowRomanChords,
    CycleAudioBackend,
}

fn settings_layout(width: u32, _height: u32) -> (RectI32, RectI32, [RectI32; 4]) {
    // Fixed-size pixel UI; good enough for now.
    let gear = RectI32 {
        x: width as i32 - 44,
        y: 8,
        w: 36,
        h: 18,
    };

    let row_h = 20;

    let panel = RectI32 {
        x: width as i32 - 170,
        y: 30,
        w: 162,
        h: 4 * row_h,
    };

    let rows = [
        RectI32 {
            x: panel.x,
            y: panel.y,
            w: panel.w,
            h: row_h,
        },
        RectI32 {
            x: panel.x,
            y: panel.y + row_h,
            w: panel.w,
            h: row_h,
        },
        RectI32 {
            x: panel.x,
            y: panel.y + 2 * row_h,
            w: panel.w,
            h: row_h,
        },
        RectI32 {
            x: panel.x,
            y: panel.y + 3 * row_h,
            w: panel.w,
            h: row_h,
        },
    ];

    (gear, panel, rows)
}

fn hit_settings_rows(x: f32, y: f32, rows: [RectI32; 4]) -> Option<SettingsAction> {
    if hit_rect(x, y, rows[0]) {
        return Some(SettingsAction::TogglePlayOnTap);
    }
    if hit_rect(x, y, rows[1]) {
        return Some(SettingsAction::ToggleShowNoteNames);
    }
    if hit_rect(x, y, rows[2]) {
        return Some(SettingsAction::ToggleShowRomanChords);
    }
    if hit_rect(x, y, rows[3]) {
        return Some(SettingsAction::CycleAudioBackend);
    }
    None
}

fn draw_strings(
    surface: &mut Surface<Rc<Window>, Rc<Window>>,
    width: u32,
    height: u32,
    top_chord: Option<Chord>,
    bottom_chord: Chord,
    positions: &[f32],
    show_note_names: bool,
    transpose_pc: i16,
    show_settings: bool,
    settings: &crate::ui_settings::UiSettings,
) {
    let mut buffer = surface.buffer_mut().unwrap();
    buffer.fill(0);

    let split = height / 2;

    fn fold_best(
        chord: Option<Chord>,
        width: u32,
        positions: &[f32],
        transpose_pc: i16,
    ) -> (Vec<u8>, Vec<u32>, Vec<u8>) {
        let mut best_prio = vec![0u8; width as usize];
        let mut best_color = vec![0u32; width as usize];
        let mut best_pc = vec![255u8; width as usize];

        let Some(chord) = chord else {
            return (best_prio, best_color, best_pc);
        };

        for (i, x) in positions.iter().enumerate() {
            let uknote = UnkeyedNote(i as i16);
            if !chord.contains(uknote) {
                continue;
            }
            let xi = x.round() as i32;
            if xi < 0 || xi >= width as i32 {
                continue;
            }
            let xi = xi as usize;

            let (prio, color) = if chord.has_root(uknote) {
                (2, 0x00FF0000)
            } else {
                (1, 0x00FFFFFF)
            };

            if prio > best_prio[xi] {
                best_prio[xi] = prio;
                best_color[xi] = color;
                best_pc[xi] = (uknote.wrap_to_octave() + transpose_pc).rem_euclid(12) as u8;
            }
        }

        (best_prio, best_color, best_pc)
    }

    let (top_prio, top_color, top_pc) = fold_best(top_chord, width, positions, transpose_pc);
    let (bot_prio, bot_color, bot_pc) = fold_best(Some(bottom_chord), width, positions, transpose_pc);

    for xi in 0..width as usize {
        if top_prio[xi] != 0 {
            for y in 0..split {
                let index = (y * width + xi as u32) as usize;
                buffer[index] = top_color[xi];
            }
        }
        if bot_prio[xi] != 0 {
            for y in split..height {
                let index = (y * width + xi as u32) as usize;
                buffer[index] = bot_color[xi];
            }
        }
    }

    if show_note_names {
        for (xi, prio) in top_prio.iter().enumerate() {
            if *prio == 0 {
                continue;
            }
            let pc = top_pc[xi];
            if pc == 255 {
                continue;
            }
            let label = crate::notes::pitch_class_label(pc as i16, transpose_pc);
            crate::pixel_font::draw_text_u32(&mut buffer, width as usize, height as usize, xi as i32 + 4, 2, label, top_color[xi], 13, 5);
        }

        let y_top = split as i32 + 2;
        for (xi, prio) in bot_prio.iter().enumerate() {
            if *prio == 0 {
                continue;
            }
            let pc = bot_pc[xi];
            if pc == 255 {
                continue;
            }
            let label = crate::notes::pitch_class_label(pc as i16, transpose_pc);
            crate::pixel_font::draw_text_u32(&mut buffer, width as usize, height as usize, xi as i32 + 4, y_top, label, bot_color[xi], 13, 5);
        }
    }

    // Settings overlay.
    let (gear, panel, rows) = settings_layout(width, height);
    // Gear button
    fill_rect(&mut buffer, width as usize, height as usize, gear, 0x00222222);
    crate::pixel_font::draw_text_u32(&mut buffer, width as usize, height as usize, gear.x + 4, gear.y + 4, "SET", 0x00FFFFFF, 13, 5);

    if show_settings {
        fill_rect(&mut buffer, width as usize, height as usize, panel, 0x00111111);
        stroke_rect(&mut buffer, width as usize, height as usize, panel, 0x00333333);

        // Row 1: TAP
        draw_checkbox_row(
            &mut buffer,
            width as usize,
            height as usize,
            rows[0],
            settings.play_on_tap,
            "TAP",
        );
        draw_checkbox_row(
            &mut buffer,
            width as usize,
            height as usize,
            rows[1],
            settings.show_note_names,
            "LBL",
        );
        draw_checkbox_row(
            &mut buffer,
            width as usize,
            height as usize,
            rows[2],
            settings.show_roman_chords,
            "ROM",
        );

        let backend_label = match settings.audio_backend {
            UiAudioBackend::Synth => "SYN",
            _ => "MID",
        };
        draw_value_row(
            &mut buffer,
            width as usize,
            height as usize,
            rows[3],
            "AUD",
            backend_label,
        );
    }

    buffer.present().unwrap();
}

fn fill_rect(buf: &mut [u32], w: usize, h: usize, r: RectI32, color: u32) {
    let x0 = r.x.max(0) as usize;
    let y0 = r.y.max(0) as usize;
    let x1 = (r.x + r.w).min(w as i32).max(0) as usize;
    let y1 = (r.y + r.h).min(h as i32).max(0) as usize;

    for y in y0..y1 {
        let row = y * w;
        for x in x0..x1 {
            buf[row + x] = color;
        }
    }
}

fn stroke_rect(buf: &mut [u32], w: usize, h: usize, r: RectI32, color: u32) {
    fill_rect(buf, w, h, RectI32 { x: r.x, y: r.y, w: r.w, h: 1 }, color);
    fill_rect(buf, w, h, RectI32 { x: r.x, y: r.y + r.h - 1, w: r.w, h: 1 }, color);
    fill_rect(buf, w, h, RectI32 { x: r.x, y: r.y, w: 1, h: r.h }, color);
    fill_rect(buf, w, h, RectI32 { x: r.x + r.w - 1, y: r.y, w: 1, h: r.h }, color);
}

fn draw_checkbox_row(buf: &mut [u32], w: usize, h: usize, row: RectI32, value: bool, label: &str) {
    let box_r = RectI32 {
        x: row.x + 6,
        y: row.y + 5,
        w: 10,
        h: 10,
    };
    fill_rect(buf, w, h, box_r, 0x00000000);
    stroke_rect(buf, w, h, box_r, 0x00777777);
    if value {
        fill_rect(
            buf,
            w,
            h,
            RectI32 {
                x: box_r.x + 2,
                y: box_r.y + 2,
                w: box_r.w - 4,
                h: box_r.h - 4,
            },
            0x00FFFFFF,
        );
    }

    crate::pixel_font::draw_text_u32(buf, w, h, row.x + 22, row.y + 3, label, 0x00FFFFFF, 13, 5);
}

fn draw_value_row(buf: &mut [u32], w: usize, h: usize, row: RectI32, label: &str, value: &str) {
    crate::pixel_font::draw_text_u32(buf, w, h, row.x + 6, row.y + 3, label, 0x00FFFFFF, 13, 5);
    crate::pixel_font::draw_text_u32(buf, w, h, row.x + 64, row.y + 3, value, 0x00FFFFFF, 13, 5);
}
