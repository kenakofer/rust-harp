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
//   Why doesn't space work for input? Should we do input differently?

use midir::os::unix::VirtualOutput;
use midir::{MidiOutput, MidiOutputConnection};
use softbuffer::{Context, Surface};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::num::NonZeroU32;
use std::rc::Rc;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

const NUM_STRINGS: usize = 44;
// MIDI Note 48 is C3. 48 strings = 4 octaves.
const START_NOTE: u8 = 33; // La in the active key
const VELOCITY: u8 = 70;
const MICRO_STEPS_PER_OCTAVE: usize = 15;
const MICRO_CHANNEL: u8 = 3; // MIDI channel 2 (0-based)
const MICRO_PROGRAM: u8 = 115; // instrument program for micro-steps, 115 = Wood block
const MICRO_NOTE: u8 = 20; // middle C for micro-step trigger
const MICRO_VELOCITY: u8 = 50; // quiet click
const MAIN_PROGRAM: u8 = 25; // Steel String Guitar (zero-based)
const BASS_PROGRAM: u8 = 26; // Bass program
const BASS_CHANNEL: u8 = 2; // MIDI channel 3 (0-based)
                            // Float bass velocity
const BASS_VELOCITY_MULTIPLIER: f64 = 1.0;
const MAIN_BASS_BOTTOM: f64 = 35.0;
const MAIN_BASS_TOP: f64 = 80.0;

const PIANO_LAYOUT: [f64; 12] = [0.0, 0.5, 1.0, 1.5, 2.0, 3.0, 3.5, 4.0, 4.5, 5.0, 5.5, 6.0];
const OCTAVE_WIDTH: f64 = 7.0;

#[derive(Clone)]
struct BuiltChord {
    // Disable name for now, since this will be better as a debugging tool rather than crucial logic
    //name: &'static str,
    root: u8,
    relative_mask: u16, // bits 0..11
}

#[derive(Eq, Hash, PartialEq, Clone)]
enum Modifier {
    AddMajor2,
    AddMinor7,
    AddMajor7,
    Minor3ToMajor,
    RestorePerfect5,
    AddSus4,
    SwitchMinorMajor,
    No3,
    ChangeKey,
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
const HEPTATONIC_MAJOR_BUTTON: &str = "HEPTATONIC_MAJOR_BUTTON";

const MINOR_7_BUTTON: &str = "MINOR_7_BUTTON";
const MAJOR_2_BUTTON: &str = "MAJOR_2_BUTTON";
const MAJOR_7_BUTTON: &str = "MAJOR_7_BUTTON";
const SUS4_BUTTON: &str = "SUS4_BUTTON";
const MINOR_MAJOR_BUTTON: &str = "MINOR_MAJOR_BUTTON";
const NO_3_BUTTON: &str = "MINOR_MAJOR_BUTTON";
const CHANGE_KEY_BUTTON: &str = "CHANGE_KEY_BUTTON";

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

    // If we have a virtual/hardware connection, set the instruments
    if let Some(ref mut conn) = conn_out {
        // Set main channel (channel 0) to main program
        let _ = conn.send(&[0xC0 | 0x00, MAIN_PROGRAM]);
        // Set bass channel program
        let _ = conn.send(&[0xC0 | (BASS_CHANNEL & 0x0F), BASS_PROGRAM]);
        // Set micro-step instrument on MICRO_CHANNEL
        let _ = conn.send(&[0xC0 | (MICRO_CHANNEL & 0x0F), MICRO_PROGRAM]);
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
    let mut prev_pos: Option<(f64, f64)> = None;
    let mut window_width = 800.0;

    let mut is_mouse_down = false;
    let mut active_chord: Option<BuiltChord> = Some(major_tri(ROOT_I));
    if let Some(ref mut nc) = active_chord {
        nc.relative_mask |= 1u16 << 2; // AddMajor2
    }

    let mut active_notes = HashSet::new();
    // Key tracking using named buttons
    let mut chord_keys_down: HashSet<&'static str> = HashSet::new();
    let mut mod_keys_down: HashSet<&'static str> = HashSet::new();
    // Modifier queue: modifiers queued and applied on next chord key press
    let mut modifier_stage: HashSet<Modifier> = HashSet::new();
    // Transpose in semitones (0-11) applied to played notes
    let mut transpose: i32 = 0;
    // We move conn_out into the event loop
    let mut midi_connection = conn_out;

    let chord_key_map: HashMap<winit::keyboard::Key, &'static str> = [
        (winit::keyboard::Key::Character("a".into()), VIIB_BUTTON),
        (winit::keyboard::Key::Character("s".into()), IV_BUTTON),
        (winit::keyboard::Key::Character("d".into()), I_BUTTON),
        (winit::keyboard::Key::Character("f".into()), V_BUTTON),
        (winit::keyboard::Key::Character("z".into()), II_BUTTON),
        (winit::keyboard::Key::Character("x".into()), VI_BUTTON),
        (winit::keyboard::Key::Character("c".into()), III_BUTTON),
        (winit::keyboard::Key::Character("v".into()), VII_BUTTON),
    ]
    .iter()
    .cloned()
    .collect();

    let mod_key_map: HashMap<winit::keyboard::Key, (&'static str, Modifier)> = [
        (
            winit::keyboard::Key::Character("5".into()),
            (MAJOR_2_BUTTON, Modifier::AddMajor2),
        ),
        (
            winit::keyboard::Key::Character("b".into()),
            (MAJOR_7_BUTTON, Modifier::AddMajor7),
        ),
        (
            winit::keyboard::Key::Character("6".into()),
            (MINOR_7_BUTTON, Modifier::AddMinor7),
        ),
        (
            winit::keyboard::Key::Character("3".into()),
            (SUS4_BUTTON, Modifier::AddSus4),
        ),
        (
            winit::keyboard::Key::Character("4".into()),
            (MINOR_MAJOR_BUTTON, Modifier::SwitchMinorMajor),
        ),
        (
            winit::keyboard::Key::Character(".".into()),
            (NO_3_BUTTON, Modifier::No3),
        ),
        (
            winit::keyboard::Key::Character("1".into()),
            (CHANGE_KEY_BUTTON, Modifier::ChangeKey),
        ),
    ]
    .iter()
    .cloned()
    .collect();

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
                        let mut chord_was_pressed = false;

                        if event.state == winit::event::ElementState::Pressed {
                            if let Some(button) = chord_key_map.get(&event.logical_key) {
                                if !chord_keys_down.contains(button) {
                                    chord_keys_down.insert(button);
                                    chord_was_pressed = true;
                                }
                            } else if let Some((button, modifier)) = mod_key_map.get(&event.logical_key) {
                                if !mod_keys_down.contains(button) {
                                    mod_keys_down.insert(button);
                                    modifier_stage.insert(modifier.clone());
                                }
                            } else if let winit::keyboard::Key::Named(
                                winit::keyboard::NamedKey::Control,
                            ) = event.logical_key
                            {
                                if event.location == winit::keyboard::KeyLocation::Left {
                                    if !chord_keys_down.contains(HEPTATONIC_MAJOR_BUTTON) {
                                        chord_keys_down.insert(HEPTATONIC_MAJOR_BUTTON);
                                        chord_was_pressed = true;
                                    }
                                }
                            }
                        } else {
                            // Released
                            if let Some(button) = chord_key_map.get(&event.logical_key) {
                                chord_keys_down.remove(button);
                            } else if let Some((button, _)) = mod_key_map.get(&event.logical_key) {
                                mod_keys_down.remove(button);
                            } else if let winit::keyboard::Key::Named(
                                winit::keyboard::NamedKey::Control,
                            ) = event.logical_key
                            {
                                if event.location == winit::keyboard::KeyLocation::Left {
                                    chord_keys_down.remove(HEPTATONIC_MAJOR_BUTTON);
                                }
                            }
                        }

                        if chord_keys_down.is_empty() {
                            return;
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
                            if (chord_keys_down.contains(VI_BUTTON)
                                && chord_keys_down.contains(II_BUTTON))
                                || (chord_keys_down.contains(III_BUTTON)
                                    && chord_keys_down.contains(VI_BUTTON))
                                || (chord_keys_down.contains(VII_BUTTON)
                                    && chord_keys_down.contains(III_BUTTON))
                                || (chord_keys_down.contains(IV_BUTTON)
                                    && chord_keys_down.contains(I_BUTTON))
                                || (chord_keys_down.contains(IV_BUTTON)
                                    && chord_keys_down.contains(VIIB_BUTTON))
                                || (chord_keys_down.contains(I_BUTTON)
                                    && chord_keys_down.contains(V_BUTTON))
                                || (chord_keys_down.contains(V_BUTTON)
                                    && chord_keys_down.contains(II_BUTTON))
                            {
                                modifier_stage.insert(Modifier::AddMinor7);
                                modifier_stage.insert(Modifier::Minor3ToMajor);
                                modifier_stage.insert(Modifier::RestorePerfect5);
                            }
                        }

                        // Inserting here supports held mods
                        if mod_keys_down.contains(MAJOR_2_BUTTON) {
                            modifier_stage.insert(Modifier::AddMajor2);
                        }
                        if mod_keys_down.contains(MINOR_7_BUTTON) {
                            modifier_stage.insert(Modifier::AddMinor7);
                        }
                        if mod_keys_down.contains(SUS4_BUTTON) {
                            modifier_stage.insert(Modifier::AddSus4);
                        }
                        if mod_keys_down.contains(MINOR_MAJOR_BUTTON) {
                            modifier_stage.insert(Modifier::SwitchMinorMajor);
                        }
                        if mod_keys_down.contains(NO_3_BUTTON) {
                            modifier_stage.insert(Modifier::RestorePerfect5);
                        }
                        if mod_keys_down.contains(MAJOR_7_BUTTON) {
                            modifier_stage.insert(Modifier::AddMajor7);
                        }
                        if mod_keys_down.contains(CHANGE_KEY_BUTTON) {
                            modifier_stage.insert(Modifier::ChangeKey);
                        }

                        // If there are modifiers queued and a chord key is down, apply them now to
                        // the freshly constructed chord, then remove it.
                        if !modifier_stage.is_empty() {
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
                                        Modifier::AddSus4 => {
                                            // Remove major/minor third (bits 3 and 4) and add perfect 4th (bit 5)
                                            nc.relative_mask &= !(1u16 << 3);
                                            nc.relative_mask &= !(1u16 << 4);
                                            nc.relative_mask |= 1u16 << 5;
                                        }
                                        Modifier::AddMajor7 => {
                                            // Add major 7th (interval 11)
                                            nc.relative_mask |= 1u16 << 11;
                                        }
                                        Modifier::SwitchMinorMajor => {
                                            // Based on root to be stable on multiple runs
                                            if nc.root == ROOT_II
                                                || nc.root == ROOT_III
                                                || nc.root == ROOT_VI
                                                || nc.root == ROOT_VII
                                            {
                                                // Change minor tri to major tri
                                                let minor_3rd_bit = 1u16 << 3;
                                                nc.relative_mask &= !minor_3rd_bit;
                                                nc.relative_mask |= 1u16 << 4;
                                            } else {
                                                // Change major tri to minor tri
                                                let major_3rd_bit = 1u16 << 4;
                                                nc.relative_mask &= !major_3rd_bit;
                                                nc.relative_mask |= 1u16 << 3;
                                            }
                                        }
                                        Modifier::No3 => {
                                            // Remove both major and minor 3rd
                                            nc.relative_mask &= !(1u16 << 3);
                                            nc.relative_mask &= !(1u16 << 4);
                                        }
                                        Modifier::RestorePerfect5 => {
                                            // Change minor 3rd to major 3rd if present
                                            let p5_bit = 1u16 << 7;
                                            let dim5_bit = 1u16 << 6;
                                            let aug5_bit = 1u16 << 8;
                                            nc.relative_mask &= !dim5_bit;
                                            nc.relative_mask &= !aug5_bit;
                                            nc.relative_mask |= p5_bit;
                                        }
                                        Modifier::ChangeKey => {
                                            // Set transpose to the chord's root
                                            transpose = nc.root as i32;
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
                        let curr_y = position.y;

                        if is_mouse_down {
                            if let Some((last_x, _)) = prev_pos {
                                // High-priority: Check for string crossings immediately
                                check_pluck(
                                    last_x,
                                    curr_x,
                                    window_width,
                                    &mut midi_connection,
                                    &active_chord,
                                    &mut active_notes,
                                    transpose,
                                );
                            }
                        }

                        prev_pos = Some((curr_x, curr_y));
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
    if chord_keys_down.contains(HEPTATONIC_MAJOR_BUTTON) {
        return Some(build_with(ROOT_I, &[0, 2, 4, 5, 7, 9, 11]));
    }

    let chord_builders: Vec<(&'static str, u8, fn(u8) -> BuiltChord)> = vec![
        (VII_BUTTON, ROOT_VII, diminished_tri),
        (III_BUTTON, ROOT_III, minor_tri),
        (VI_BUTTON, ROOT_VI, minor_tri),
        (II_BUTTON, ROOT_II, minor_tri),
        (V_BUTTON, ROOT_V, major_tri),
        (I_BUTTON, ROOT_I, major_tri),
        (IV_BUTTON, ROOT_IV, major_tri),
        (VIIB_BUTTON, ROOT_VIIB, major_tri),
    ];

    for (button, root, builder) in chord_builders {
        if chord_keys_down.contains(button) {
            if let Some(old) = old_chord {
                if old.root == root {
                    return Some(old.clone());
                }
            }
            return Some(builder(root));
        }
    }

    // No keys down: preserve chord if we just went from 1 -> 0
    if let Some(old) = old_chord {
        return Some(old.clone());
    }

    None
}

/// Compute x positions for each string using a piano-like layout.

fn compute_string_positions(width: f64) -> Vec<f64> {

    let mut positions: Vec<f64> = vec![0.0; NUM_STRINGS];



    let get_unscaled_pos = |note: i32| -> f64 {

        let octave = (note / 12) as f64;

        let pitch_class = (note.rem_euclid(12)) as usize;

        octave * OCTAVE_WIDTH + PIANO_LAYOUT[pitch_class]

    };



    let first_note = START_NOTE as i32;

    let last_note = START_NOTE as i32 + (NUM_STRINGS - 1) as i32;



    let first_pos = get_unscaled_pos(first_note);

    let last_pos = get_unscaled_pos(last_note);



    let total_conceptual_width = last_pos - first_pos;



    if total_conceptual_width <= 0.0 {

        // Fallback for no width

        return positions;

    }



    let scale_factor = width / total_conceptual_width;



    for i in 0..NUM_STRINGS {

        let note = START_NOTE as i32 + i as i32;

        let unscaled_pos = get_unscaled_pos(note);

        positions[i] = (unscaled_pos - first_pos) * scale_factor;

    }



    positions

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
    transpose: i32,
) {
    if conn.is_none() {
        return;
    }

    // Use shared compute function to get positions
    let positions = compute_string_positions(width);

    // Determine the range of movement
    let min_x = x1.min(x2);
    let max_x = x1.max(x2);

    // Iterate through all string positions to see if one lies within the movement range
    for i in 0..NUM_STRINGS {
        let string_x = positions[i];

        // Strict crossing check
        if string_x > min_x && string_x <= max_x {
            if is_note_in_chord(i, active_chord) {
                let vel = VELOCITY as u8;
                play_note(conn, i, active_notes, transpose, vel);
            }
        }
    }

    // Micro-steps: map mouse movement to global micro indices across screen without loops
    let num_octaves = NUM_STRINGS / 12; // integer octaves covered
    if num_octaves > 0 {
        let micros_total = num_octaves * MICRO_STEPS_PER_OCTAVE;
        let micros_total_f = micros_total as f64;
        // Map x to micro index (0..micros_total-1) by projecting x across the screen
        let map_to_micro = |x: f64| -> isize {
            // clamp x to [0, width]
            let xc = if x < 0.0 {
                0.0
            } else if x > width {
                width
            } else {
                x
            };
            let frac = xc / width;
            let idx_f = (frac * micros_total_f).round();
            let mut idx = idx_f as isize;
            if idx < 0 {
                idx = 0
            }
            if idx >= micros_total as isize {
                idx = micros_total as isize - 1
            }
            idx
        };

        let prev_idx = map_to_micro(min_x);
        let curr_idx = map_to_micro(max_x);
        let diff = curr_idx - prev_idx;
        if diff != 0 {
            let count = diff.abs() as usize;
            if let Some(ref mut c) = conn {
                let on = 0x90 | (MICRO_CHANNEL & 0x0F);
                let off = 0x80 | (MICRO_CHANNEL & 0x0F);
                for _ in 0..count {
                    let _ = c.send(&[on, MICRO_NOTE, MICRO_VELOCITY]);
                    let _ = c.send(&[off, MICRO_NOTE, 0]);
                }
            }
        }
    }
}

fn play_note(
    conn: &mut Option<MidiOutputConnection>,
    string_index: usize,
    active_notes: &mut HashSet<u8>,
    transpose: i32,
    velocity: u8,
) {
    if let Some(c) = conn {
        let mut note = START_NOTE as i32 + string_index as i32 + transpose;
        if note < 0 {
            note = 0
        }
        if note > 127 {
            note = 127
        }
        let note_u = note as u8;

        // Crossfade between bass and main
        let main_factor = if note_u as f64 <= MAIN_BASS_BOTTOM {
            0.0
        } else if note_u as f64 >= MAIN_BASS_TOP {
            1.0
        } else {
            (note_u as f64 - MAIN_BASS_BOTTOM) / (MAIN_BASS_TOP - MAIN_BASS_BOTTOM)
        };
        let bass_factor = 1.0 - main_factor;

        // On second thought, lets give main_factor twice as long of a fade
        let main_factor = 1.0 - 0.5 * (1.0 - main_factor);

        // Scale velocities
        let main_vel = ((velocity as f64) * main_factor).round() as u8;
        let bass_vel = ((velocity as f64) * bass_factor * BASS_VELOCITY_MULTIPLIER).round() as u8;

        // Clamp velocities to max
        let main_vel = if main_vel > 127 { 127 } else { main_vel };
        let bass_vel = if bass_vel > 127 { 127 } else { bass_vel };

        // Clamp small nonzero factors to at least velocity 1 so they are audible
        let mut main_vel = main_vel;
        let mut bass_vel = bass_vel;
        if main_factor > 0.0 && main_vel == 0 {
            main_vel = 1
        }
        if bass_factor > 0.0 && bass_vel == 0 {
            bass_vel = 1
        }

        // Send to bass channel if bass_vel > 0 (Note On only)
        if bass_vel > 0 {
            let on_b = 0x90 | (BASS_CHANNEL & 0x0F);
            let off_bass = 0x80 | (BASS_CHANNEL & 0x0F);
            // Send an off first
            let _ = c.send(&[off_bass, note_u, 0]);
            let _ = c.send(&[on_b, note_u, bass_vel]);
        }

        // Send to main channel if main_vel > 0 (Note On only)
        if main_vel > 0 {
            let on_m = 0x90 | 0x00;
            let _ = c.send(&[on_m, note_u, main_vel]);
        }

        active_notes.insert(note_u);
    }
}

fn stop_note(conn: &mut Option<MidiOutputConnection>, note: u8, active_notes: &mut HashSet<u8>) {
    if let Some(c) = conn {
        // Send Note Off on both channels to ensure silence
        let off_main = 0x80 | 0x00;
        let off_bass = 0x80 | (BASS_CHANNEL & 0x0F);
        let _ = c.send(&[off_main, note, 0]);
        let _ = c.send(&[off_bass, note, 0]);
        active_notes.remove(&note);
    }
}

/// Minimalist drawing function.
/// Fills buffer with black and draws vertical lines for active strings.
fn draw_strings(
    surface: &mut Surface<Rc<Window>, Rc<Window>>,
    width: u32,
    height: u32,
    active_chord: &Option<BuiltChord>,
) {
    let mut buffer = surface.buffer_mut().unwrap();
    buffer.fill(0); // Fill with black

    let positions = compute_string_positions(width as f64);

    if active_chord.is_none() {
        buffer.present().unwrap();
        return;
    }

    for i in 0..NUM_STRINGS {
        if is_note_in_chord(i, active_chord) {
            let x = positions[i].round() as u32;
            if x >= width {
                continue;
            }

            let note = START_NOTE + i as u8;
            let color = if note % 12 == active_chord.as_ref().unwrap().root {
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
