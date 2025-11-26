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

use midir::{MidiOutput, MidiOutputConnection};
use midir::os::unix::VirtualOutput;
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

#[derive(PartialEq)]
struct Chord {
    name: &'static str,
    pitch_classes: &'static [u8],
}

const IV_MAJOR: Chord = Chord {
    name: "IV",
    pitch_classes: &[5, 9, 0],
};

const I_MAJOR: Chord = Chord {
    name: "I",
    pitch_classes: &[0, 4, 7],
};

const V_MAJOR: Chord = Chord {
    name: "V",
    pitch_classes: &[7, 11, 2],
};

const V7_MAJOR: Chord = Chord {
    name: "V7",
    pitch_classes: &[7, 11, 2, 5],
};

const II_MINOR: Chord = Chord {
    name: "ii",
    pitch_classes: &[2, 5, 9],
};

const II7_MAJOR: Chord = Chord {
    name: "II7",
    pitch_classes: &[2, 6, 9, 0],
};

const I7_MAJOR: Chord = Chord {
    name: "I7",
    pitch_classes: &[0, 4, 7, 10],
};

// Named button identifiers for key tracking
const IV_BUTTON: &str = "IV_BUTTON";
const I_BUTTON: &str = "I_BUTTON";
const V_BUTTON: &str = "V_BUTTON";
const II_BUTTON: &str = "II_BUTTON";

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
                println!("Virtual port failed. Connecting to first available hardware port: {}", midi_out.port_name(port)?);
                conn_out = Some(midi_out.connect(port, "Rust Harp Connection")?);
            } else {
                eprintln!("Warning: No MIDI ports found. Application will run visually but emit no sound.");
            }
        }
    }

    // 2. Setup Window
    let event_loop = EventLoop::new()?;
    let window = Rc::new(WindowBuilder::new()
        .with_title("Rust MIDI Harp")
        .with_inner_size(winit::dpi::LogicalSize::new(800.0, 600.0))
        .build(&event_loop)?);

    // 3. Setup Graphics Context
    let context = Context::new(window.clone()).expect("Failed to create graphics context");
    let mut surface = Surface::new(&context, window.clone()).expect("Failed to create surface");

    // Application State
    let mut prev_x: Option<f64> = None;
    let mut window_width = 800.0;
    let mut is_mouse_down = false;
    let mut active_chord: Option<&'static Chord> = None;
    let mut active_notes = HashSet::new();
    // Key tracking using named buttons
    let mut keys_down: HashSet<&'static str> = HashSet::new();
    let mut prev_keys_count: usize = 0;
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
                        // Map key presses/releases into named buttons set
                        if event.state == winit::event::ElementState::Pressed {
                            match event.logical_key.as_ref() {
                                winit::keyboard::Key::Character("a") => { keys_down.insert(IV_BUTTON); },
                                winit::keyboard::Key::Character("s") => { keys_down.insert(I_BUTTON); },
                                winit::keyboard::Key::Character("d") => { keys_down.insert(V_BUTTON); },
                                winit::keyboard::Key::Character("f") => { keys_down.insert(II_BUTTON); },
                                _ => {}
                            }
                        } else {
                            match event.logical_key.as_ref() {
                                winit::keyboard::Key::Character("a") => { keys_down.remove(IV_BUTTON); },
                                winit::keyboard::Key::Character("s") => { keys_down.remove(I_BUTTON); },
                                winit::keyboard::Key::Character("d") => { keys_down.remove(V_BUTTON); },
                                winit::keyboard::Key::Character("f") => { keys_down.remove(II_BUTTON); },
                                _ => {}
                            }
                        }

                        let old_chord = active_chord;
                        let new_chord = decide_chord(old_chord, &keys_down, prev_keys_count);

                        if old_chord.map(|c| c.name) != new_chord.map(|c| c.name) {
                            // Stop any playing notes that are not in the new chord
                            if let Some(new) = new_chord {
                                let notes_to_stop: Vec<u8> = active_notes
                                    .iter()
                                    .filter(|&&note| !new.pitch_classes.contains(&(note % 12)))
                                    .cloned()
                                    .collect();
                                for note in notes_to_stop {
                                    stop_note(&mut midi_connection, note, &mut active_notes);
                                }
                            }
                            active_chord = new_chord;
                            window.request_redraw();
                        }

                        prev_keys_count = keys_down.len();
                    }
                    
                    WindowEvent::Resized(physical_size) => {
                        surface.resize(
                            NonZeroU32::new(physical_size.width).unwrap(),
                            NonZeroU32::new(physical_size.height).unwrap(),
                        ).unwrap();
                        window_width = physical_size.width as f64;
                        
                        // Redraw lines on resize
                        draw_strings(&mut surface, physical_size.width, physical_size.height, &active_chord);
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
                                check_pluck(last_x, curr_x, window_width, &mut midi_connection, &active_chord, &mut active_notes);
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
fn is_note_in_chord(string_index: usize, chord: &Option<&'static Chord>) -> bool {
    if let Some(chord) = chord {
        let note = START_NOTE + string_index as u8;
        let pitch_class = note % 12;
        chord.pitch_classes.contains(&pitch_class)
    } else {
        // If no chord is active, all notes are "in"
        true
    }
}

// Decide chord from current keys_down and previous chord state.
fn decide_chord(old_chord: Option<&'static Chord>, keys_down: &HashSet<&'static str>, prev_keys_count: usize) -> Option<&'static Chord> {
    // Pair combos first (higher precedence)
    if keys_down.contains(IV_BUTTON) && keys_down.contains(I_BUTTON) {
        return Some(&I7_MAJOR);
    }
    if keys_down.contains(I_BUTTON) && keys_down.contains(V_BUTTON) {
        return Some(&V7_MAJOR);
    }
    // Existing combo: V + II => II7
    if keys_down.contains(V_BUTTON) && keys_down.contains(II_BUTTON) {
        return Some(&II7_MAJOR);
    }

    // Priority mapping: IV > I > V > ii
    if keys_down.contains(IV_BUTTON) {
        return Some(&IV_MAJOR);
    }
    if keys_down.contains(I_BUTTON) {
        // Preserve I7 if it was previously active
        if old_chord == Some(&I7_MAJOR) {
            return old_chord;
        } else {
            return Some(&I_MAJOR);
        }
    }
    if keys_down.contains(V_BUTTON) {
        // Preserve V7 if it was previously active
        if old_chord == Some(&V7_MAJOR) {
            return old_chord;
        } else {
            return Some(&V_MAJOR);
        }
    }
    if keys_down.contains(II_BUTTON) {
        // Preserve II7 if that was the previous chord
        if old_chord == Some(&II7_MAJOR) {
            return old_chord;
        } else {
            return Some(&II_MINOR);
        }
    }

    // No keys down: preserve chord if we just went from 1 -> 0
    if keys_down.len() == 0 && prev_keys_count == 1 {
        return old_chord;
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
    active_chord: &Option<&'static Chord>,
    active_notes: &mut HashSet<u8>,
) {
    if conn.is_none() { return; }
    
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

fn play_note(conn: &mut Option<MidiOutputConnection>, string_index: usize, active_notes: &mut HashSet<u8>) {
    if let Some(c) = conn {
        let note = START_NOTE + string_index as u8;
        // Send Note On (Channel 0)
        // 0x90 = Note On, Channel 1
        // note = 0-127
        // VELOCITY = 100
        let _ = c.send(&[0x90, note, VELOCITY]);
        active_notes.insert(note);
        
        // Note: We are not sending Note Off to keep logic lock-free and minimal latency.
        // Most "pluck" synth patches decay naturally. If you need Note Off,
        // it would require a timer or thread which adds complexity/latency overhead.
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
fn draw_strings(surface: &mut Surface<Rc<Window>, Rc<Window>>, width: u32, height: u32, active_chord: &Option<&'static Chord>) {
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
