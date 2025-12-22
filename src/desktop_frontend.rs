use crate::chord::Chord;
use crate::notes::{MidiNote, Transpose, UnkeyedNote, UnmidiNote};
use crate::output_midir::MidiBackend;
use crate::strum;
use crate::touch::{PointerId, TouchEvent, TouchPhase, TouchTracker};
use crate::ui_adapter::AppAdapter;

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

// Pre-calculated unscaled relative x-positions for each string, ranging from 0.0 to 1.0.
// ensuring string positions scale correctly with window resizing while
// maintaining the musical interval spacing.
const UNSCALED_RELATIVE_X_POSITIONS: &[f32] = &[
    2.03124999999999972e-02,
    5.19531250000000028e-02,
    9.02343750000000056e-02,
    1.31445312499999994e-01,
    1.63281249999999989e-01,
    1.96289062500000000e-01,
    2.33203125000000011e-01,
    2.66015624999999978e-01,
    3.05859374999999989e-01,
    3.38867187500000000e-01,
    3.75000000000000000e-01,
    4.05468749999999989e-01,
    4.49414062499999989e-01,
    4.85546874999999989e-01,
    5.20312499999999956e-01,
    5.55273437499999911e-01,
    5.92578124999999956e-01,
    6.29687499999999956e-01,
    6.65429687500000089e-01,
    6.99999999999999956e-01,
    7.34960937500000022e-01,
    7.71289062499999956e-01,
    8.07617187500000000e-01,
    8.42773437500000000e-01,
    8.80664062499999956e-01,
    9.18359374999999978e-01,
    9.49999999999999956e-01,
    9.91796875000000022e-01,
];

// Use length of array
const NUM_STRINGS: usize = UNSCALED_RELATIVE_X_POSITIONS.len();

const NOTE_TO_STRING_IN_OCTAVE: [u16; 12] = [0, 0, 1, 1, 2, 3, 3, 4, 4, 5, 6, 6];

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
    let mut touch = TouchTracker::new();

    // App State
    let mut app = AppAdapter::new();

    // 4. Run Event Loop
    event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Wait);

        match event {
            Event::WindowEvent { window_id, event } if window_id == window.id() => match event {
                WindowEvent::CloseRequested => {
                    // Turn off all active notes before closing
                    let notes_to_stop: Vec<UnmidiNote> = app.active_notes().collect();
                    for note in notes_to_stop {
                        midi.stop_note(MIDI_BASE_TRANSPOSE + note);
                    }
                    elwt.exit();
                }

                WindowEvent::KeyboardInput { event, .. } => {
                    if let Some(effects) = app.handle_winit_key_event(&event) {
                        let _ = process_app_effects(effects, &mut midi, Some(window.as_ref()));
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
                        app.active_chord(),
                        &note_positions,
                    );
                }

                WindowEvent::MouseInput { state, button, .. } => {
                    if button == winit::event::MouseButton::Left {
                        let pressed = state == winit::event::ElementState::Pressed;
                        is_mouse_down = pressed;

                        let Some((x, _)) = prev_pos else {
                            return;
                        };

                        let phase = if pressed { TouchPhase::Down } else { TouchPhase::Up };
                        let chord = *app.active_chord();
                        let out = touch.handle_event(
                            TouchEvent {
                                id: PointerId(0),
                                phase,
                                x,
                            },
                            &note_positions,
                            |n| match chord {
                                Some(c) => c.contains(n),
                                None => true,
                            },
                        );

                        if let Some(note) = out.strike {
                            let effects = app.handle_strum_crossing(note);
                            let _ = process_app_effects(effects, &mut midi, Some(window.as_ref()));
                        }
                    }
                }

                WindowEvent::CursorMoved { position, .. } => {
                    let curr_x = position.x as f32;
                    let curr_y = position.y as f32;

                    if is_mouse_down {
                        let chord = *app.active_chord();
                        let out = touch.handle_event(
                            TouchEvent {
                                id: PointerId(0),
                                phase: TouchPhase::Move,
                                x: curr_x,
                            },
                            &note_positions,
                            |n| match chord {
                                Some(c) => c.contains(n),
                                None => true,
                            },
                        );

                        for crossing in out.crossings {
                            for note in crossing.notes {
                                let effects = app.handle_strum_crossing(note);
                                let _ = process_app_effects(effects, &mut midi, Some(window.as_ref()));
                            }
                        }
                    }

                    prev_pos = Some((curr_x, curr_y));
                }

                WindowEvent::RedrawRequested => {
                    let size = window.inner_size();
                    draw_strings(
                        &mut surface,
                        size.width,
                        size.height,
                        app.active_chord(),
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
    positions.clear();

    // Add as many notes til we go off the right side of the screen.
    for octave in 0.. {
        for uknote in 0..12 {
            let string_in_octave = NOTE_TO_STRING_IN_OCTAVE[uknote as usize] as usize;
            let string = octave * 7 + string_in_octave;
            if string >= NUM_STRINGS.into() {
                return;
            }
            let x = UNSCALED_RELATIVE_X_POSITIONS[string] * width;
            positions.push(x);
        }
    }
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
    active_chord: &Option<Chord>,
    positions: &[f32],
) {
    let mut buffer = surface.buffer_mut().unwrap();
    buffer.fill(0);

    if active_chord.is_none() {
        buffer.present().unwrap();
        return;
    }

    for i in 0..positions.len() {
        let uknote = UnkeyedNote(i as i16);
        if active_chord.map_or(true, |c| c.contains(uknote)) {
            let x = positions[i].round() as u32;
            if x >= width {
                continue;
            }

            let color = if active_chord.map_or(false, |c| c.has_root(uknote)) {
                0xFF0000
            } else {
                0xFFFFFF
            };

            for y in 0..height {
                let index = (y * width + x) as usize;
                if index < buffer.len() {
                    buffer[index] = color;
                }
            }
        }
    }

    buffer.present().unwrap();
}
