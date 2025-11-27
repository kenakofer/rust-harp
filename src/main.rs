//! # Rust MIDI Harp
//!
//! A low-latency, windowed MIDI controller application designed for Linux.
//!
//! ## Functionality
//! * **Interaction**: Dragging the mouse cursor across a line triggers a MIDI Note On event.
//! * **Sound**: Acts as a virtual MIDI device (ALSA sequencer) named "Rust Harp Output".
//!     Connect this output to any DAW or synthesizer to produce sound.
//! * **Latency**: Prioritizes low-latency input handling by processing events directly
//!     in the `winit` event loop without waiting for frame redraws.
//! * **Visuals**: Super low priority. Displays a window with evenly spaced vertical lines
//!     representing strings.

// Ideas TODO:
//   Evenly spread active lines/notes to even out strum
//   Mouse capture that works with the wacom?
//   Why doesn't space work for input? Should we do input differently?
//

use midir::os::unix::VirtualOutput;
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

const NUM_STRINGS: usize = 48;
// MIDI Note 48 is C3. 48 strings = 4 octaves.
const START_NOTE: u8 = 41;
const VELOCITY: u8 = 100;

#[derive(Clone)]
struct BuiltChord {
    // Disable name for now, since this will be better as a debugging tool rather than crucial logic
    //name: &'static str,
    root: u8,
    relative_mask: u16, // bits 0..11
}

#[derive(Eq, Hash, PartialEq)]
enum Modifier {
    AddMajor2,
    AddMinor7,
    Minor3ToMajor,
}

fn build_with(root: u8, rels: &[u8]) -> BuiltChord {
    let mut mask: u16 = 0;
    for &r in rels.iter() {
        let rel = (r as usize) % 12;
        mask |= 1u16 << rel;
    }
    BuiltChord {
        root,
        relative_mask: mask,
    }
}

// Runtime root constants and builders
const ROOT_VIIB: u8 = 10;
const ROOT_IV: u8 = 5;
const ROOT_I: u8 = 0;
const ROOT_V: u8 = 7;
const ROOT_II: u8 = 2;
const ROOT_VI: u8 = 9;
const ROOT_III: u8 = 4;
const ROOT_VII: u8 = 11;

fn major_tri(root: u8) -> BuiltChord {
    build_with(root, &[0, 4, 7])
}
fn minor_tri(root: u8) -> BuiltChord {
    build_with(root, &[0, 3, 7])
}
fn diminished_tri(root: u8) -> BuiltChord {
    build_with(root, &[0, 3, 6])
}

// Named button identifiers for key tracking
const VIIB_BUTTON: &str = "VIIB_BUTTON";
const IV_BUTTON: &str = "IV_BUTTON";
const I_BUTTON: &str = "I_BUTTON";
const V_BUTTON: &str = "V_BUTTON";
const II_BUTTON: &str = "II_BUTTON";
const VI_BUTTON: &str = "VI_BUTTON";
const III_BUTTON: &str = "III_BUTTON";
const VII_BUTTON: &str = "VII_BUTTON";

const MINOR_7_BUTTON: &str = "MINOR_7_BUTTON";
const MAJOR_2_BUTTON: &str = "MAJOR_2_BUTTON";

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    // 1. Setup MIDI Output
    // We try to create a virtual port first (best for Linux/ALSA).
    let midi_out = MidiOutput::new("Rust Harp Client")?;
    let mut conn_out: Option<MidiOutputConnection> = None;

    // Attempt to create a virtual port.
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
                eprintln!("Warning: No MIDI ports found. Application will run visually but emit no sound.");
            }
        }
    }

    // 2. Setup Window
    let event_loop = EventLoop::new()?;
    let window = Rc::new(
        WindowBuilder::new()
            .with_title("Rust MIDI Harp")
            .with_inner_size(winit::dpi::LogicalSize::new(800.0, 600.0))
            .build(&event_loop)?,
    );

    // 3. Setup Graphics Context
    let context = Context::new(window.clone()).expect("Failed to create graphics context");
    let mut surface = Surface::new(&context, window.clone()).expect("Failed to create surface");

    // Application State
    let mut prev_x: Option<f64> = None;
    let mut window_width = 800.0;
    let mut is_mouse_down = false;
    let mut active_chord: Option<BuiltChord> = None;
    let mut active_notes = HashSet::new();
    // Key tracking using named buttons
    let mut chord_keys_down: HashSet<&'static str> = HashSet::new();
    let mut mod_keys_down: HashSet<&'static str> = HashSet::new();
    // Modifier queue: modifiers queued and applied on next chord key press
    let mut modifier_stage: HashSet<Modifier> = HashSet::new();
    // We move conn_out into the event loop
    let mut midi_connection = conn_out;

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
                        let notes_to_stop: Vec<u8> = active_notes.iter().cloned().collect();
                        for note in notes_to_stop {
                            stop_note(&mut midi_connection, note, &mut active_notes);
                        }
                        elwt.exit();
                    }

                    WindowEvent::KeyboardInput { event, .. } => {
                        // Track if a chord key was pressed in this event so old_chord is cleared
                        // when deciding the new chord. Avoid borrowing active_chord across
                        // mutation by computing old_chord after handling the key event.
                        let mut chord_was_pressed = false;

                        // Map key presses/releases into named buttons set. We don't want to
                        // remember old_chord if there's a new chord button pressed. We only want
                        // to remember it for releases and mod presses.
                        if event.state == winit::event::ElementState::Pressed {
                            match event.logical_key.as_ref() {
                                winit::keyboard::Key::Character("a") => {
                                    // Prevent held presses from re-adding or removing mods
                                    if chord_keys_down.contains(VIIB_BUTTON) {
                                        return;
                                    }
                                    chord_keys_down.insert(VIIB_BUTTON);
                                    chord_was_pressed = true;
                                }
                                winit::keyboard::Key::Character("s") => {
                                    if chord_keys_down.contains(IV_BUTTON) {
                                        return;
                                    }
                                    chord_keys_down.insert(IV_BUTTON);
                                    chord_was_pressed = true;
                                }
                                winit::keyboard::Key::Character("d") => {
                                    if chord_keys_down.contains(I_BUTTON) {
                                        return;
                                    }
                                    chord_keys_down.insert(I_BUTTON);
                                    chord_was_pressed = true;
                                }
                                winit::keyboard::Key::Character("f") => {
                                    if chord_keys_down.contains(V_BUTTON) {
                                        return;
                                    }
                                    chord_keys_down.insert(V_BUTTON);
                                    chord_was_pressed = true;
                                }
                                winit::keyboard::Key::Character("z") => {
                                    if chord_keys_down.contains(II_BUTTON) {
                                        return;
                                    }
                                    chord_keys_down.insert(II_BUTTON);
                                    chord_was_pressed = true;
                                }
                                winit::keyboard::Key::Character("x") => {
                                    if chord_keys_down.contains(VI_BUTTON) {
                                        return;
                                    }
                                    chord_keys_down.insert(VI_BUTTON);
                                    chord_was_pressed = true;
                                }
                                winit::keyboard::Key::Character("c") => {
                                    if chord_keys_down.contains(III_BUTTON) {
                                        return;
                                    }
                                    chord_keys_down.insert(III_BUTTON);
                                    chord_was_pressed = true;
                                }
                                winit::keyboard::Key::Character("v") => {
                                    if chord_keys_down.contains(VII_BUTTON) {
                                        return;
                                    }
                                    chord_keys_down.insert(VII_BUTTON);
                                    chord_was_pressed = true;
                                }
                                winit::keyboard::Key::Character("b") => {
                                    if mod_keys_down.contains(MAJOR_2_BUTTON) {
                                        return;
                                    }
                                    mod_keys_down.insert(MAJOR_2_BUTTON);
                                    // inserting here supports sequentially input mods
                                    modifier_stage.insert(Modifier::AddMajor2);
                                    if chord_keys_down.len() == 0 {
                                        return;
                                    }
                                }
                                winit::keyboard::Key::Character("n") => {
                                    if mod_keys_down.contains(MINOR_7_BUTTON) {
                                        return;
                                    }
                                    mod_keys_down.insert(MINOR_7_BUTTON);
                                    // inserting here supports sequentially input mods
                                    modifier_stage.insert(Modifier::AddMinor7);
                                    if chord_keys_down.len() == 0 {
                                        return;
                                    }
                                }
                                _ => {}
                            }
                        } else {
                            match event.logical_key.as_ref() {
                                winit::keyboard::Key::Character("a") => {
                                    chord_keys_down.remove(VIIB_BUTTON);
                                }
                                winit::keyboard::Key::Character("s") => {
                                    chord_keys_down.remove(IV_BUTTON);
                                }
                                winit::keyboard::Key::Character("d") => {
                                    chord_keys_down.remove(I_BUTTON);
                                }
                                winit::keyboard::Key::Character("f") => {
                                    chord_keys_down.remove(V_BUTTON);
                                }
                                winit::keyboard::Key::Character("z") => {
                                    chord_keys_down.remove(II_BUTTON);
                                }
                                winit::keyboard::Key::Character("x") => {
                                    chord_keys_down.remove(VI_BUTTON);
                                }
                                winit::keyboard::Key::Character("c") => {
                                    chord_keys_down.remove(III_BUTTON);
                                }
                                winit::keyboard::Key::Character("v") => {
                                    chord_keys_down.remove(VII_BUTTON);
                                }
                                winit::keyboard::Key::Character("b") => {
                                    mod_keys_down.remove(MAJOR_2_BUTTON);
                                }
                                winit::keyboard::Key::Character("n") => {
                                    mod_keys_down.remove(MINOR_7_BUTTON);
                                }
                                _ => {}
                            }
                        }

                        let old_chord = if chord_was_pressed {
                            None
                        } else {
                            active_chord.as_ref()
                        };
                        let mut new_chord = decide_chord_base(old_chord, &chord_keys_down);

                        // If a chord key was just pressed, detect pair combos that imply a minor-7
                        // and enqueue the AddMinor7 modifier so it is applied via the existing
                        // modifier pipeline.
                        if chord_was_pressed {
                            // Pairs that imply minor 7: VI+II, III+VI, VII+III, IV+I, IV+VIIB, I+V, V+II
                            if (chord_keys_down.contains(VI_BUTTON) && chord_keys_down.contains(II_BUTTON))
                                || (chord_keys_down.contains(III_BUTTON) && chord_keys_down.contains(VI_BUTTON))
                                || (chord_keys_down.contains(VII_BUTTON)
                                    && chord_keys_down.contains(III_BUTTON))
                                || (chord_keys_down.contains(IV_BUTTON) && chord_keys_down.contains(I_BUTTON))
                                || (chord_keys_down.contains(IV_BUTTON)
                                    && chord_keys_down.contains(VIIB_BUTTON))
                                || (chord_keys_down.contains(I_BUTTON) && chord_keys_down.contains(V_BUTTON))
                                || (chord_keys_down.contains(V_BUTTON) && chord_keys_down.contains(II_BUTTON))
                            {
                                modifier_stage.insert(Modifier::AddMinor7);
                                modifier_stage.insert(Modifier::Minor3ToMajor);
                            }
                        }

                        // Inserting here supports held mods
                        if mod_keys_down.contains(MAJOR_2_BUTTON) {
                            modifier_stage.insert(Modifier::AddMajor2);
                        }
                        if mod_keys_down.contains(MINOR_7_BUTTON) {
                            modifier_stage.insert(Modifier::AddMinor7);
                        }

                        // If there are modifiers queued and a chord key is down, apply them now to
                        // the freshly constructed chord, then remove it.
                        if !modifier_stage.is_empty() && chord_keys_down.len() > 0 {
                            if let Some(ref mut nc) = new_chord {
                                for m in modifier_stage.drain() {
                                    match m {
                                        Modifier::AddMajor2 => {
                                            nc.relative_mask |= 1u16 << 2;
                                        }
                                        Modifier::AddMinor7 => {
                                            nc.relative_mask |= 1u16 << 10;
                                        }
                                        Modifier::Minor3ToMajor => {
                                            // Change minor 3rd to major 3rd if present
                                            let minor_3rd_bit = 1u16 << 3;
                                            if (nc.relative_mask & minor_3rd_bit) != 0 {
                                                nc.relative_mask &= !minor_3rd_bit;
                                                nc.relative_mask |= 1u16 << 4;
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // If the notes aren't the same, do the switch
                        if old_chord.map_or(true, |old| {
                            new_chord.as_ref().map_or(true, |new| {
                                old.root != new.root || old.relative_mask != new.relative_mask
                            })
                        }) {
                            // Stop any playing notes that are not in the new chord
                            if let Some(new) = new_chord.as_ref() {
                                let notes_to_stop: Vec<u8> = active_notes
                                    .iter()
                                    .filter(|&&note| {
                                        let pc = note % 12;
                                        let rel =
                                            ((12 + pc as i32 - new.root as i32) % 12) as usize;
                                        (new.relative_mask & (1u16 << rel)) == 0
                                    })
                                    .cloned()
                                    .collect();
                                for note in notes_to_stop {
                                    stop_note(&mut midi_connection, note, &mut active_notes);
                                }
                            }
                            active_chord = new_chord;
                            window.request_redraw();
                        }
                    }

                    WindowEvent::Resized(physical_size) => {
                        surface
                            .resize(
                                NonZeroU32::new(physical_size.width).unwrap(),
                                NonZeroU32::new(physical_size.height).unwrap(),
                            )
                            .unwrap();
                        window_width = physical_size.width as f64;

                        // Redraw lines on resize
                        draw_strings(
                            &mut surface,
                            physical_size.width,
                            physical_size.height,
                            &active_chord,
                        );
                    }

                    WindowEvent::MouseInput { state, button, .. } => {
                        if button == winit::event::MouseButton::Left {
                            is_mouse_down = state == winit::event::ElementState::Pressed;
                        }
                    }

                    WindowEvent::CursorMoved { position, .. } => {
                        let curr_x = position.x;

                        if is_mouse_down {
                            if let Some(last_x) = prev_x {
                                // High-priority: Check for string crossings immediately
                                check_pluck(
                                    last_x,
                                    curr_x,
                                    window_width,
                                    &mut midi_connection,
                                    &active_chord,
                                    &mut active_notes,
                                );
                            }
                        }

                        prev_x = Some(curr_x);
                    }

                    WindowEvent::RedrawRequested => {
                        // Initial draw if needed, though Resized usually handles it on startup
                        let size = window.inner_size();
                        draw_strings(&mut surface, size.width, size.height, &active_chord);
                    }

                    _ => {}
                }
            }
            _ => {}
        }
    })?;

    Ok(())
}

/// Returns true if a string's note is in the provided chord.
fn is_note_in_chord(string_index: usize, chord: &Option<BuiltChord>) -> bool {
    if let Some(chord) = chord {
        let note = START_NOTE + string_index as u8;
        let pitch_class = note % 12;
        let rel = ((12 + pitch_class as i32 - chord.root as i32) % 12) as usize;
        (chord.relative_mask & (1u16 << rel)) != 0
    } else {
        // If no chord is active, all notes are "in"
        true
    }
}

// Decide chord from current chord_keys_down and previous chord state.
fn decide_chord_base(
    old_chord: Option<&BuiltChord>,
    chord_keys_down: &HashSet<&'static str>,
) -> Option<BuiltChord> {
    if chord_keys_down.contains(VII_BUTTON) {
        if let Some(old) = old_chord {
            if old.root == ROOT_VII {
                return Some(old.clone());
            }
        }
        return Some(diminished_tri(ROOT_VII));
    }

    if chord_keys_down.contains(III_BUTTON) {
        if let Some(old) = old_chord {
            if old.root == ROOT_III {
                return Some(old.clone());
            }
        }
        return Some(minor_tri(ROOT_III));
    }

    if chord_keys_down.contains(VI_BUTTON) {
        if let Some(old) = old_chord {
            if old.root == ROOT_VI {
                return Some(old.clone());
            }
        }
        return Some(minor_tri(ROOT_VI));
    }

    if chord_keys_down.contains(II_BUTTON) {
        // Preserve II7 if that was the previous chord
        if let Some(old) = old_chord {
            if old.root == ROOT_II {
                return Some(old.clone());
            }
        }
        return Some(minor_tri(ROOT_II));
    }

    if chord_keys_down.contains(V_BUTTON) {
        // Preserve V7 if it was previously active
        if let Some(old) = old_chord {
            if old.root == ROOT_V {
                return Some(old.clone());
            }
        }
        return Some(major_tri(ROOT_V));
    }

    if chord_keys_down.contains(I_BUTTON) {
        // Preserve I7 if it was previously active
        if let Some(old) = old_chord {
            if old.root == ROOT_I {
                return Some(old.clone());
            }
        }
        return Some(major_tri(ROOT_I));
    }

    if chord_keys_down.contains(IV_BUTTON) {
        if let Some(old) = old_chord {
            if old.root == ROOT_I {
                return Some(old.clone());
            }
        }
        return Some(major_tri(ROOT_IV));
    }

    if chord_keys_down.contains(VIIB_BUTTON) {
        return Some(major_tri(ROOT_VIIB));
    }

    // No keys down: preserve chord if we just went from 1 -> 0
    if let Some(old) = old_chord {
        return Some(old.clone());
    }

    None
}

/// Core Logic: Detects if the mouse cursor crossed any string boundaries.
/// We calculate the string positions dynamically based on window width.
fn check_pluck(
    x1: f64,
    x2: f64,
    width: f64,
    conn: &mut Option<MidiOutputConnection>,
    active_chord: &Option<BuiltChord>,
    active_notes: &mut HashSet<u8>,
) {
    if conn.is_none() {
        return;
    }

    // Divide width into NUM_STRINGS + 1 segments to evenly space them
    // Spacing logic:  |  s1  |  s2  | ...
    let spacing = width / (NUM_STRINGS as f64 + 1.0);

    // Determine the range of movement
    let min_x = x1.min(x2);
    let max_x = x1.max(x2);

    // Iterate through all string positions to see if one lies within the movement range
    for i in 0..NUM_STRINGS {
        let string_x = spacing * (i as f64 + 1.0);

        // Strict crossing check
        if string_x > min_x && string_x <= max_x {
            if is_note_in_chord(i, active_chord) {
                play_note(conn, i, active_notes);
            }
        }
    }
}

fn play_note(
    conn: &mut Option<MidiOutputConnection>,
    string_index: usize,
    active_notes: &mut HashSet<u8>,
) {
    if let Some(c) = conn {
        let note = START_NOTE + string_index as u8;
        // Send Note On (Channel 0)
        // 0x90 = Note On, Channel 1
        // note = 0-127
        let _ = c.send(&[0x90, note, VELOCITY]);
        active_notes.insert(note);
    }
}

fn stop_note(conn: &mut Option<MidiOutputConnection>, note: u8, active_notes: &mut HashSet<u8>) {
    if let Some(c) = conn {
        // Send Note Off (Channel 0)
        let _ = c.send(&[0x80, note, 0]);
        active_notes.remove(&note);
    }
}

/// Minimalist drawing function.
/// Fills buffer with black and draws white vertical lines.
fn draw_strings(
    surface: &mut Surface<Rc<Window>, Rc<Window>>,
    width: u32,
    height: u32,
    active_chord: &Option<BuiltChord>,
) {
    let mut buffer = surface.buffer_mut().unwrap();

    // Fill with black (0x000000)
    buffer.fill(0);

    let spacing = width as f64 / (NUM_STRINGS as f64 + 1.0);

    for i in 0..NUM_STRINGS {
        let x = (spacing * (i as f64 + 1.0)) as u32;

        let color = if active_chord.is_some() && !is_note_in_chord(i, active_chord) {
            0x404040 // Dark Grey for inactive strings
        } else {
            0xFFFFFF // White for active strings
        };

        // Simple vertical line drawing
        if x < width {
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
