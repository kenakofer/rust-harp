use crate::chord::Chord;
use crate::notes::{MidiNote, Transpose, UnkeyedNote, UnmidiNote};
use crate::output_midir::MidiBackend;
use crate::strum;
use crate::touch::{PointerId, TouchEvent, TouchPhase};
use crate::rows::RowId;
use crate::ui_adapter::{self, AppAdapter};
use crate::ui_events::{UiEvent, UiSession};

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

const MIDI_BASE_TRANSPOSE: Transpose = Transpose(36); // Add with UnmidiNote to get MidiNote. MIDI Note 36 is C2
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

    let mut midi = MidiBackend::new(conn_out, MAIN_CHANNEL, BASS_CHANNEL);

    // If we have a connection, set the instruments
    if let Some(conn) = midi.conn_mut() {
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

    // 4. Run Event Loop
    event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Wait);

        match event {
            Event::WindowEvent { window_id, event } if window_id == window.id() => match event {
                WindowEvent::CloseRequested => {
                    // Turn off all active notes before closing
                    let notes_to_stop: Vec<UnmidiNote> = ui.engine().active_notes().collect();
                    for note in notes_to_stop {
                        midi.stop_note(MIDI_BASE_TRANSPOSE + note);
                    }
                    elwt.exit();
                }

                WindowEvent::KeyboardInput { event, .. } => {
                    if let Some(ue) = ui_adapter::ui_event_from_winit(&event) {
                        let out = ui.handle(ue, &note_positions);
                        let _ = process_app_effects(out.effects, &mut midi, Some(window.as_ref()));
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
                    );
                }

                WindowEvent::MouseInput { state, button, .. } => {
                    if button == winit::event::MouseButton::Left {
                        let pressed = state == winit::event::ElementState::Pressed;
                        is_mouse_down = pressed;

                        let Some((x, y)) = prev_pos else {
                            return;
                        };

                        let phase = if pressed { TouchPhase::Down } else { TouchPhase::Up };
                        let h = window.inner_size().height.max(1) as f32;
                        let out = ui.handle(
                            UiEvent::Touch(TouchEvent {
                                id: PointerId(0),
                                phase,
                                x,
                                y_norm: (y / h).clamp(0.0, 1.0),
                            }),
                            &note_positions,
                        );
                        let _ = process_app_effects(out.effects, &mut midi, Some(window.as_ref()));
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
                            }),
                            &note_positions,
                        );
                        let _ = process_app_effects(out.effects, &mut midi, Some(window.as_ref()));
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

fn process_app_effects(
    effects: crate::app_state::AppEffects,
    midi: &mut MidiBackend,
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
        midi.stop_note(MIDI_BASE_TRANSPOSE + un);
    }
    for pn in effects.play_notes {
        midi.play_note(MIDI_BASE_TRANSPOSE + pn.note, pn.volume);
    }

    played
}

#[allow(dead_code)]
fn check_pluck(
    x1: f32,
    x2: f32,
    midi: &mut MidiBackend,
    app: &mut AppAdapter,
    note_positions: &[f32],
) {
    if !midi.is_available() {
        return;
    }

    for crossing in strum::detect_crossings(x1, x2, note_positions) {
        let mut played_any = false;

        for note in crossing.notes {
            let effects = app.handle_strum_crossing(note);
            if process_app_effects(effects, midi, None) {
                played_any = true;
            }
        }

        if !played_any {
            // Damped string sound
            midi.send_note_on(MICRO_CHANNEL, MICRO_NOTE, MICRO_VELOCITY);
            midi.send_note_off(MICRO_CHANNEL, MICRO_NOTE);
        }
    }
}


fn draw_strings(
    surface: &mut Surface<Rc<Window>, Rc<Window>>,
    width: u32,
    height: u32,
    top_chord: Option<Chord>,
    bottom_chord: Chord,
    positions: &[f32],
) {
    let mut buffer = surface.buffer_mut().unwrap();
    buffer.fill(0);

    let split = height / 2;

    fn fold_best(
        chord: Option<Chord>,
        width: u32,
        positions: &[f32],
    ) -> (Vec<u8>, Vec<u32>) {
        let mut best_prio = vec![0u8; width as usize];
        let mut best_color = vec![0u32; width as usize];

        let Some(chord) = chord else {
            return (best_prio, best_color);
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
                (2, 0xFF0000)
            } else {
                (1, 0xFFFFFF)
            };

            if prio > best_prio[xi] {
                best_prio[xi] = prio;
                best_color[xi] = color;
            }
        }

        (best_prio, best_color)
    }

    let (top_prio, top_color) = fold_best(top_chord, width, positions);
    let (bot_prio, bot_color) = fold_best(Some(bottom_chord), width, positions);

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

    buffer.present().unwrap();
}
