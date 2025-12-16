mod chord;
mod notes;
mod app_state;

use chord::{Chord, Modifiers};
use notes::{MidiNote, Transpose, UnbottomedNote, UnkeyedNote};
use app_state::{AppState, ChordButton, ModButton, ActionButton, Actions, KeyEvent, KeyState, LOWEST_NOTE};

use midir::{MidiOutput, MidiOutputConnection};
use softbuffer::{Context, Surface};
use std::collections::HashSet;
use std::error::Error;
use std::num::NonZeroU32;
use std::rc::Rc;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

// MIDI Note 48 is C3. 48 strings = 4 octaves.
const VELOCITY: u8 = 70;
const MICRO_CHANNEL: u8 = 3; // MIDI channel 2 (0-based)
const MICRO_PROGRAM: u8 = 115; // instrument program for micro-steps, 115 = Wood block
const MICRO_NOTE: MidiNote = MidiNote(20); // middle C for micro-step trigger
const MICRO_VELOCITY: u8 = 50; // quiet click
const MAIN_PROGRAM: u8 = 25; // Steel String Guitar (zero-based)
const MAIN_CHANNEL: u8 = 0;
const BASS_PROGRAM: u8 = 26;
const BASS_CHANNEL: u8 = 2;
const BASS_VELOCITY_MULTIPLIER: f32 = 1.0;
const MAIN_BASS_BOTTOM: MidiNote = MidiNote(35);
const MAIN_BASS_TOP: MidiNote = MidiNote(80);

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


struct ChordButtonTableEntry {
    button: ChordButton,
    key_check: fn(&winit::keyboard::Key) -> bool,
}

const CHORD_BUTTON_TABLE: [ChordButtonTableEntry; 9] = [
    ChordButtonTableEntry {
        button: ChordButton::VIIB,
        key_check: |k| matches!(k, winit::keyboard::Key::Character(s) if s == "a"),
    },
    ChordButtonTableEntry {
        button: ChordButton::IV,
        key_check: |k| matches!(k, winit::keyboard::Key::Character(s) if s == "s"),
    },
    ChordButtonTableEntry {
        button: ChordButton::I,
        key_check: |k| matches!(k, winit::keyboard::Key::Character(s) if s == "d"),
    },
    ChordButtonTableEntry {
        button: ChordButton::V,
        key_check: |k| matches!(k, winit::keyboard::Key::Character(s) if s == "f"),
    },
    ChordButtonTableEntry {
        button: ChordButton::II,
        key_check: |k| matches!(k, winit::keyboard::Key::Character(s) if s == "z"),
    },
    ChordButtonTableEntry {
        button: ChordButton::VI,
        key_check: |k| matches!(k, winit::keyboard::Key::Character(s) if s == "x"),
    },
    ChordButtonTableEntry {
        button: ChordButton::III,
        key_check: |k| matches!(k, winit::keyboard::Key::Character(s) if s == "c"),
    },
    ChordButtonTableEntry {
        button: ChordButton::VII,
        key_check: |k| matches!(k, winit::keyboard::Key::Character(s) if s == "v"),
    },
    ChordButtonTableEntry {
        button: ChordButton::HeptatonicMajor,
        key_check: |k| {
            matches!(
                k,
                winit::keyboard::Key::Named(winit::keyboard::NamedKey::Control)
            )
        },
    },
];


struct ModButtonTableEntry {
    button: ModButton,
    key_check: fn(&winit::keyboard::Key) -> bool,
    modifiers: Modifiers,
}

const MOD_BUTTON_TABLE: [ModButtonTableEntry; 6] = [
    ModButtonTableEntry {
        button: ModButton::Major2,
        key_check: |k| matches!(k, winit::keyboard::Key::Character(s) if s == "5"),
        modifiers: Modifiers::AddMajor2,
    },
    ModButtonTableEntry {
        button: ModButton::Major7,
        key_check: |k| matches!(k, winit::keyboard::Key::Character(s) if s == "b"),
        modifiers: Modifiers::AddMajor7,
    },
    ModButtonTableEntry {
        button: ModButton::Minor7,
        key_check: |k| matches!(k, winit::keyboard::Key::Character(s) if s == "6"),
        modifiers: Modifiers::AddMinor7,
    },
    ModButtonTableEntry {
        button: ModButton::Sus4,
        key_check: |k| matches!(k, winit::keyboard::Key::Character(s) if s == "3"),
        modifiers: Modifiers::Sus4,
    },
    ModButtonTableEntry {
        button: ModButton::MinorMajor,
        key_check: |k| matches!(k, winit::keyboard::Key::Character(s) if s == "4"),
        modifiers: Modifiers::SwitchMinorMajor,
    },
    ModButtonTableEntry {
        button: ModButton::No3,
        key_check: |k| matches!(k, winit::keyboard::Key::Character(s) if s == "."),
        modifiers: Modifiers::No3,
    },
];

fn chord_button_for(key: &winit::keyboard::Key) -> Option<ChordButton> {
    CHORD_BUTTON_TABLE
        .iter()
        .find(|e| (e.key_check)(key))
        .map(|e| e.button)
}
fn mod_button_for(key: &winit::keyboard::Key) -> Option<(ModButton, Modifiers)> {
    MOD_BUTTON_TABLE
        .iter()
        .find(|e| (e.key_check)(key))
        .map(|e| (e.button, e.modifiers))
}

fn action_button_for(key: &winit::keyboard::Key) -> Option<(ActionButton, Actions)> {
    use winit::keyboard::Key::Character;
    use winit::keyboard::Key::Named;
    use winit::keyboard::NamedKey::Tab;

    match key {
        Character(s) if s == "1" => Some((ActionButton::ChangeKey, Actions::ChangeKey)),
        Named(Tab) => Some((ActionButton::Pulse, Actions::Pulse)),
        _ => None,
    }
}


fn key_event_from_winit(event: &winit::event::KeyEvent) -> Option<KeyEvent> {
    let state = match event.state {
        winit::event::ElementState::Pressed => KeyState::Pressed,
        winit::event::ElementState::Released => KeyState::Released,
    };

    let key = &event.logical_key;

    if let Some(button) = chord_button_for(key) {
        return Some(KeyEvent::Chord { state, button });
    }

    if let Some((button, modifiers)) = mod_button_for(key) {
        return Some(KeyEvent::Modifier {
            state,
            button,
            modifiers,
        });
    }

    if let Some((button, action)) = action_button_for(key) {
        return Some(KeyEvent::Action {
            state,
            button,
            action,
        });
    }

    None
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    // 1. Setup MIDI Output
    // We try to create a virtual port first (best for Linux/ALSA).
    let midi_out = MidiOutput::new("Rust Harp Client")?;
    let mut conn_out: Option<MidiOutputConnection> = None;

    // Attempt to create virtual port on systems where that exists
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    match midi_out.create_virtual("Rust Harp Output") {
        Ok(conn) => {
            println!("Created virtual MIDI port: 'Rust Harp Output'");
            conn_out = Some(conn);
        }
        Err(_) => {
            // Fallback for non-ALSA environments or errors
            let midi_out = MidiOutput::new("Rust Harp Client")?;
            let ports = midi_out.ports();
            if let Some(port) = ports.first() {
                println!(
                    "Virtual port failed. Connecting to first available hardware port: {}",
                    midi_out.port_name(port)?
                );
                conn_out = Some(midi_out.connect(port, "Rust Harp Connection")?);
            } else {
                eprintln!("Warning: No MIDI ports found. Application will emit no sound.");
            }
        }
    }

    // Fallback for Windows or failure
    if conn_out.is_none() {
        let ports = midi_out.ports();
        if let Some(port) = ports.first() {
            println!(
                "Connecting to hardware MIDI port: {}",
                midi_out.port_name(port)?
            );
            conn_out = Some(midi_out.connect(port, "Rust Harp Connection")?);
        } else {
            eprintln!("Warning: No MIDI ports found. Application will emit no sound.");
        }
    }

    // If we have a virtual/hardware connection, set the instruments
    if let Some(conn) = conn_out.as_mut() {
        // Set main channel (channel 0) to main program
        let _ = conn.send(&[0xC0 | MAIN_CHANNEL, MAIN_PROGRAM]);
        // Set bass channel program
        let _ = conn.send(&[0xC0 | BASS_CHANNEL, BASS_PROGRAM]);
        // Set micro-step instrument on MICRO_CHANNEL
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
    let mut midi_connection = conn_out;
    let mut note_positions: Vec<f32> = Vec::new();

    // App State
    let mut app_state = AppState::new();
    let mut active_notes = HashSet::new();

    // 4. Run Event Loop
    event_loop.run(move |event, elwt| {
        // Set ControlFlow to Wait. This is efficient; it sleeps until an event (like mouse move) arrives.
        // For a controller, this provides immediate response upon OS interrupt.
        elwt.set_control_flow(ControlFlow::Wait);

        match event {
            Event::WindowEvent { window_id, event } if window_id == window.id() => {
                match event {
                    WindowEvent::CloseRequested => {
                        // Turn off all active notes before closing
                        let notes_to_stop: Vec<MidiNote> = active_notes.iter().cloned().collect();
                        for note in notes_to_stop {
                            stop_note(&mut midi_connection, note, &mut active_notes);
                        }
                        elwt.exit();
                    }

                    WindowEvent::KeyboardInput { event, .. } => {
                        if let Some(app_event) = key_event_from_winit(&event) {
                            let effects = app_state.handle_key_event(app_event);

                            if effects.redraw {
                                window.request_redraw();
                            }
                            if let Some(transpose) = effects.change_key {
                                println!("Changed key: {:?}", transpose);
                            }
                            if let Some(chord) = app_state.active_chord.as_ref() {
                                let notes_to_stop: Vec<MidiNote> = (0..128)
                                    .map(|i| MidiNote(i))
                                    .filter(|mn| {
                                        !chord.contains(*mn - LOWEST_NOTE - app_state.transpose)
                                    })
                                    .filter(|mn| active_notes.contains(mn))
                                    .collect();
                                for mn in notes_to_stop {
                                    stop_note(&mut midi_connection, mn, &mut active_notes)
                                }
                            }
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

                        // Redraw lines on resize
                        draw_strings(
                            &mut surface,
                            physical_size.width,
                            physical_size.height,
                            &app_state.active_chord,
                            &note_positions,
                        );
                    }

                    WindowEvent::MouseInput { state, button, .. } => {
                        if button == winit::event::MouseButton::Left {
                            is_mouse_down = state == winit::event::ElementState::Pressed;
                        }
                    }

                    WindowEvent::CursorMoved { position, .. } => {
                        let curr_x = position.x as f32;
                        let curr_y = position.y as f32;

                        if is_mouse_down {
                            if let Some((last_x, _)) = prev_pos {
                                // High-priority: Check for string crossings immediately
                                check_pluck(
                                    last_x,
                                    curr_x,
                                    &mut midi_connection,
                                    &app_state.active_chord,
                                    &mut active_notes,
                                    app_state.transpose,
                                    &note_positions,
                                );
                            }
                        }

                        prev_pos = Some((curr_x, curr_y));
                    }

                    WindowEvent::RedrawRequested => {
                        // Initial draw if needed, though Resized usually handles it on startup
                        let size = window.inner_size();
                        draw_strings(
                            &mut surface,
                            size.width,
                            size.height,
                            &app_state.active_chord,
                            &note_positions,
                        );
                    }

                    _ => {}
                }
            }
            _ => {}
        }
    })?;

    Ok(())
}

fn _compute_string_positions(width: f32) -> Vec<f32> {
    let mut positions: Vec<f32> = vec![0.0; NUM_STRINGS];

    for i in 0..NUM_STRINGS {
        positions[i] = UNSCALED_RELATIVE_X_POSITIONS[i] * width;
    }

    positions
}

fn recompute_note_positions(positions: &mut Vec<f32>, width: f32) {
    positions.clear(); // Keeps the memory allocated for fast re-use

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

/// Core Logic: Detects if the mouse cursor crossed any string boundaries.
/// We calculate the string positions dynamically based on window width.
fn check_pluck(
    x1: f32,
    x2: f32,
    conn: &mut Option<MidiOutputConnection>,
    active_chord: &Option<Chord>,
    active_notes: &mut HashSet<MidiNote>,
    transpose: Transpose,
    note_positions: &[f32],
) {
    if conn.is_none() {
        return;
    }

    // Determine the range of movement
    let min_x = x1.min(x2);
    let max_x = x1.max(x2);

    let mut played_note_at_pos = false;
    let mut crossed_pos = false;
    let mut string_x: f32 = 0.0;

    // Iterate through all string positions to see if one lies within the movement range
    for i in 0..note_positions.len() {
        let uknote = UnkeyedNote(i as i16);
        let ubnote = transpose + uknote;

        // If we're proceeding to the next string position, crossed the previous one, and didn't
        // play a note at it, then play the dampened string sound.
        if string_x != note_positions[i] {
            if crossed_pos && !played_note_at_pos {
                // Play the MICRO sound
                if let Some(c) = conn.as_mut() {
                    send_note_on(c, MICRO_CHANNEL, MICRO_NOTE, MICRO_VELOCITY);
                    send_note_off(c, MICRO_CHANNEL, MICRO_NOTE);
                }
            }
            played_note_at_pos = false;
            crossed_pos = false;
        }

        string_x = note_positions[i];

        // Strict crossing check
        if string_x > min_x && string_x <= max_x {
            crossed_pos = true;
            if active_chord.map_or(true, |c| c.contains(uknote)) {
                let vel = VELOCITY as u8;
                play_note(conn, ubnote, active_notes, vel);
                played_note_at_pos = true;
            }
        }
    }
}

fn send_note_on(c: &mut MidiOutputConnection, channel: u8, note: MidiNote, vel: u8) {
    if vel == 0 {
        send_note_off(c, channel, note);
        return;
    }
    let on = 0x90 | (channel & 0x0F);
    let _ = c.send(&[on, note.0, vel]);
}
fn send_note_off(c: &mut MidiOutputConnection, channel: u8, note: MidiNote) {
    let off = 0x80 | (channel & 0x0F);
    let _ = c.send(&[off, note.0, 0]);
}

fn play_note(
    conn: &mut Option<MidiOutputConnection>,
    note: UnbottomedNote,
    active_notes: &mut HashSet<MidiNote>,
    velocity: u8,
) {
    if let Some(c) = conn {
        let midi_note = LOWEST_NOTE + note;

        let main_factor = (midi_note - MAIN_BASS_BOTTOM)
            .ratio(MAIN_BASS_TOP - MAIN_BASS_BOTTOM)
            .clamp(0.0, 1.0);
        let bass_factor = 1.0 - main_factor;

        // On second thought, lets give main_factor twice as long of a fade
        let main_factor = 1.0 - 0.5 * (1.0 - main_factor);

        if main_factor > 0.0 {
            let mut main_vel = ((velocity as f32) * main_factor).round() as u8;
            main_vel = main_vel.clamp(1, 127);
            send_note_on(c, MAIN_CHANNEL, midi_note, main_vel);
        }
        if bass_factor > 0.0 {
            let mut bass_vel =
                ((velocity as f32) * bass_factor * BASS_VELOCITY_MULTIPLIER).round() as u8;
            bass_vel = bass_vel.clamp(1, 127);
            // Send an off to bass first to get a solid rearticulation
            send_note_off(c, BASS_CHANNEL, midi_note);
            send_note_on(c, BASS_CHANNEL, midi_note, bass_vel);
        }

        active_notes.insert(midi_note);
    }
}

fn stop_note(
    conn: &mut Option<MidiOutputConnection>,
    note: MidiNote,
    active_notes: &mut HashSet<MidiNote>,
) {
    if let Some(c) = conn {
        // Send Note Off on both channels to ensure silence
        send_note_off(c, MAIN_CHANNEL, note);
        send_note_off(c, BASS_CHANNEL, note);
        active_notes.remove(&note);
    }
}

/// Minimalist drawing function.
/// Fills buffer with black and draws vertical lines for active strings.
fn draw_strings(
    surface: &mut Surface<Rc<Window>, Rc<Window>>,
    width: u32,
    height: u32,
    active_chord: &Option<Chord>,
    positions: &[f32],
) {
    let mut buffer = surface.buffer_mut().unwrap();
    buffer.fill(0); // Fill with black

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
                0xFF0000 // Red for root
            } else {
                0xFFFFFF // White for other active notes
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
