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
//!
//!
//!     TODO:
//!     - rearrange mods/4 doesn't remove 3
//!     - Pedal toggle/mod?
//!     - Phone app version?
//!     - Check bass balance on keychange
//!     - Refactor to avoid mod logic duplication
//!     - Fix held pulse key triggering rapid repeat
//!     - Pulse in wonky in other keys

use midir::os::unix::VirtualOutput;
use midir::{MidiOutput, MidiOutputConnection};
use softbuffer::{Context, Surface};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::num::NonZeroU32;
use std::ops::{Add, Rem, Sub};
use std::rc::Rc;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq, PartialOrd)]
struct MidiNote(u8);

impl Rem<u8> for MidiNote {
    type Output = u8;
    fn rem(self, rhs: u8) -> u8 {
        self.0 % rhs
    }
}

impl Sub for MidiNote {
    type Output = Interval;
    fn sub(self, rhs: MidiNote) -> Interval {
        Interval(self.0 as i16 - rhs.0 as i16)
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
struct UnbottomedNote(i16); // Note before building on the BOTTOM_NOTE

#[derive(Copy, Clone, Debug, PartialEq)]
struct Transpose(i16); // Basically an interval

impl Add<UnkeyedNote> for Transpose {
    type Output = UnbottomedNote;
    fn add(self, rhs: UnkeyedNote) -> UnbottomedNote {
        let sum: i16 = (self.0 as i16) + (rhs.0 as i16);
        UnbottomedNote(sum)
    }
}

impl Add<UnbottomedNote> for Transpose {
    type Output = MidiNote;
    fn add(self, rhs: UnbottomedNote) -> MidiNote {
        let sum: i16 = (self.0 as i16) + (rhs.0 as i16);
        if sum < 0 {
            return MidiNote(0);
        } else if sum > 127 {
            return MidiNote(127);
        }
        MidiNote(sum as u8)
    }
}

impl Sub<Transpose> for UnbottomedNote {
    type Output = UnkeyedNote;
    fn sub(self, rhs: Transpose) -> UnkeyedNote {
        let diff: i16 = (self.0 as i16) - (rhs.0 as i16);
        UnkeyedNote(diff)
    }
}

impl Sub<Transpose> for MidiNote {
    type Output = UnbottomedNote;
    fn sub(self, rhs: Transpose) -> UnbottomedNote {
        let diff: i16 = (self.0 as i16) - rhs.0;
        UnbottomedNote(diff)
    }
}

// Note before transposing into the key or building on the BOTTOM_NOTE.
// This is basically solfege: Do = 0, Re = 2, etc. Can go beyond 12 or below 0
#[derive(Copy, Clone, Debug, PartialEq)]
struct UnkeyedNote(i16);

impl Sub for UnkeyedNote {
    type Output = Interval;
    fn sub(self, rhs: UnkeyedNote) -> Interval {
        Interval(self.0 - rhs.0)
    }
}

// Position above the root of the chord
// "The fifth in the chord" would be 7 for example
//#[derive(Copy, Clone, Debug, PartialEq)]
struct UnrootedNote(u8);
impl UnrootedNote {
    pub fn new(i: Interval) -> Self {
        Self(i.0.rem_euclid(12) as u8)
    }
}

// Difference in half steps
#[derive(Copy, Clone, Debug, PartialEq)]
struct Interval(i16);

impl Interval {
    fn ratio(self, denom: Interval) -> f32 {
        self.0 as f32 / denom.0 as f32
    }
}

// MIDI Note 48 is C3. 48 strings = 4 octaves.
const LOWEST_NOTE: Transpose = Transpose(24); // Do in the active key
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

#[derive(Clone, Copy)]
struct Chord {
    // Disable name for now, since this will be better as a debugging tool rather than crucial logic
    //name: &'static str,
    root: UnkeyedNote,
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
    Pulse,
}

fn get_note_above_root(note: UnkeyedNote, root: UnkeyedNote) -> UnrootedNote {
    UnrootedNote::new(note - root)
}

fn is_note_in_chord(note: UnkeyedNote, chord: &Option<Chord>) -> bool {
    if let Some(chord) = chord {
        let rel = get_note_above_root(note, chord.root);
        chord.relative_mask & (1u16 << (rel.0 as usize)) != 0
    } else {
        // If no chord is active, all notes are "in"
        true
    }
}

fn is_note_root_of_chord(note: UnkeyedNote, chord: &Option<Chord>) -> bool {
    if let Some(chord) = chord {
        // Make a new chord with only the one note
        let chord_of_one = build_with(chord.root, &[0]);
        is_note_in_chord(note, &Some(chord_of_one))
    } else {
        false
    }
}

fn build_with(root: UnkeyedNote, rels: &[u8]) -> Chord {
    let mut mask: u16 = 0;
    for &r in rels.iter() {
        let rel = (r as usize) % 12;
        mask |= 1u16 << rel;
    }
    Chord {
        root,
        relative_mask: mask,
    }
}

const ROOT_VIIB: UnkeyedNote = UnkeyedNote(10);
const ROOT_IV: UnkeyedNote = UnkeyedNote(5);
const ROOT_I: UnkeyedNote = UnkeyedNote(0);
const ROOT_V: UnkeyedNote = UnkeyedNote(7);
const ROOT_II: UnkeyedNote = UnkeyedNote(2);
const ROOT_VI: UnkeyedNote = UnkeyedNote(9);
const ROOT_III: UnkeyedNote = UnkeyedNote(4);
const ROOT_VII: UnkeyedNote = UnkeyedNote(11);

fn major_tri(root: UnkeyedNote) -> Chord {
    build_with(root, &[0, 4, 7])
}
fn minor_tri(root: UnkeyedNote) -> Chord {
    build_with(root, &[0, 3, 7])
}
fn diminished_tri(root: UnkeyedNote) -> Chord {
    build_with(root, &[0, 3, 6])
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
enum ChordButton {
  VIIB, IV, I, V, II, VI, III, VII, HeptatonicMajor,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
enum ModButton {
    Major2,
    Minor7,
    Major7,
    Sus4,
    MinorMajor,
    No3,
    ChangeKey,
    Pulse,
}
/*const MINOR_7_BUTTON: &str = "MINOR_7_BUTTON";
const MAJOR_2_BUTTON: &str = "MAJOR_2_BUTTON";
const MAJOR_7_BUTTON: &str = "MAJOR_7_BUTTON";
const SUS4_BUTTON: &str = "SUS4_BUTTON";
const MINOR_MAJOR_BUTTON: &str = "MINOR_MAJOR_BUTTON";
const NO_3_BUTTON: &str = "MINOR_MAJOR_BUTTON";
const CHANGE_KEY_BUTTON: &str = "CHANGE_KEY_BUTTON";
const PULSE_BUTTON: &str = "PULSE_BUTTON";*/

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
        let _ = conn.send(&[0xC0 | MAIN_CHANNEL, MAIN_PROGRAM]);
        // Set bass channel program
        let _ = conn.send(&[0xC0 | BASS_CHANNEL, BASS_PROGRAM]);
        // Set micro-step instrument on MICRO_CHANNEL
        let _ = conn.send(&[0xC0 | MICRO_CHANNEL, MICRO_PROGRAM]);
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
    let mut prev_pos: Option<(f32, f32)> = None;

    let mut is_mouse_down = false;
    let mut active_chord: Option<Chord> = Some(major_tri(ROOT_I));
    if let Some(ref mut nc) = active_chord {
        nc.relative_mask |= 1u16 << 2; // AddMajor2
    }

    let mut active_notes = HashSet::new();
    // Key tracking using named buttons
    let mut chord_keys_down: HashSet<ChordButton> = HashSet::new();
    let mut mod_keys_down: HashSet<ModButton> = HashSet::new();
    // Modifier queue: modifiers queued and applied on next chord key press
    let mut modifier_stage: HashSet<Modifier> = HashSet::new();
    // Transpose in semitones (0-11) applied to played notes
    let mut transpose: Transpose = Transpose(0);
    // We move conn_out into the event loop
    let mut midi_connection = conn_out;
    let mut note_positions: [f32; NUM_STRINGS * 2] = [0.0; NUM_STRINGS * 2];

    let chord_key_map: HashMap<winit::keyboard::Key, ChordButton> = [
        (winit::keyboard::Key::Character("a".into()), ChordButton::VIIB),
        (winit::keyboard::Key::Character("s".into()), ChordButton::IV),
        (winit::keyboard::Key::Character("d".into()), ChordButton::I),
        (winit::keyboard::Key::Character("f".into()), ChordButton::V),
        (winit::keyboard::Key::Character("z".into()), ChordButton::II),
        (winit::keyboard::Key::Character("x".into()), ChordButton::VI),
        (winit::keyboard::Key::Character("c".into()), ChordButton::III),
        (winit::keyboard::Key::Character("v".into()), ChordButton::VII),
    ]
    .iter()
    .cloned()
    .collect();

    let mod_key_map: HashMap<winit::keyboard::Key, (ModButton, Modifier)> = [
        (
            winit::keyboard::Key::Character("5".into()),
            (ModButton::Major2, Modifier::AddMajor2),
        ),
        (
            winit::keyboard::Key::Character("b".into()),
            (ModButton::Major7, Modifier::AddMajor7),
        ),
        (
            winit::keyboard::Key::Character("6".into()),
            (ModButton::Minor7, Modifier::AddMinor7),
        ),
        (
            winit::keyboard::Key::Character("3".into()),
            (ModButton::Sus4, Modifier::AddSus4),
        ),
        (
            winit::keyboard::Key::Character("4".into()),
            (ModButton::MinorMajor, Modifier::SwitchMinorMajor),
        ),
        (
            winit::keyboard::Key::Character(".".into()),
            (ModButton::No3, Modifier::No3),
        ),
        (
            winit::keyboard::Key::Character("1".into()),
            (ModButton::ChangeKey, Modifier::ChangeKey),
        ),
        (
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::Tab),
            (ModButton::Pulse, Modifier::Pulse),
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
                        let notes_to_stop: Vec<MidiNote> = active_notes.iter().cloned().collect();
                        for note in notes_to_stop {
                            stop_note(&mut midi_connection, note, &mut active_notes);
                        }
                        elwt.exit();
                    }

                    WindowEvent::KeyboardInput { event, .. } => {
                        let mut chord_was_pressed = false;

                        if event.state == winit::event::ElementState::Pressed {
                            if let Some(&button) = chord_key_map.get(&event.logical_key) {
                                if !chord_keys_down.contains(&button) {
                                    chord_keys_down.insert(button);
                                    chord_was_pressed = true;
                                }
                            } else if let Some((button, modifier)) =
                                mod_key_map.get(&event.logical_key)
                            {
                                if !mod_keys_down.contains(button) {
                                    mod_keys_down.insert(*button);
                                    modifier_stage.insert(modifier.clone());
                                }
                            } else if let winit::keyboard::Key::Named(
                                winit::keyboard::NamedKey::Control,
                            ) = event.logical_key
                            {
                                if event.location == winit::keyboard::KeyLocation::Left {
                                    if !chord_keys_down.contains(&ChordButton::HeptatonicMajor) {
                                        chord_keys_down.insert(ChordButton::HeptatonicMajor);
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
                                    chord_keys_down.remove(&ChordButton::HeptatonicMajor);
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
                            if (chord_keys_down.contains(&ChordButton::VI)
                                && chord_keys_down.contains(&ChordButton::II))
                                || (chord_keys_down.contains(&ChordButton::III)
                                    && chord_keys_down.contains(&ChordButton::VI))
                                || (chord_keys_down.contains(&ChordButton::VII)
                                    && chord_keys_down.contains(&ChordButton::III))
                                || (chord_keys_down.contains(&ChordButton::IV)
                                    && chord_keys_down.contains(&ChordButton::I))
                                || (chord_keys_down.contains(&ChordButton::IV)
                                    && chord_keys_down.contains(&ChordButton::VIIB))
                                || (chord_keys_down.contains(&ChordButton::I)
                                    && chord_keys_down.contains(&ChordButton::V))
                                || (chord_keys_down.contains(&ChordButton::V)
                                    && chord_keys_down.contains(&ChordButton::II))
                            {
                                modifier_stage.insert(Modifier::AddMinor7);
                                modifier_stage.insert(Modifier::Minor3ToMajor);
                                modifier_stage.insert(Modifier::RestorePerfect5);
                            }
                        }

                        // Inserting here supports held mods
                        if mod_keys_down.contains(&ModButton::Major2) {
                            modifier_stage.insert(Modifier::AddMajor2);
                        }
                        if mod_keys_down.contains(&ModButton::Minor7) {
                            modifier_stage.insert(Modifier::AddMinor7);
                        }
                        if mod_keys_down.contains(&ModButton::Sus4) {
                            modifier_stage.insert(Modifier::AddSus4);
                        }
                        if mod_keys_down.contains(&ModButton::MinorMajor) {
                            modifier_stage.insert(Modifier::SwitchMinorMajor);
                        }
                        if mod_keys_down.contains(&ModButton::No3) {
                            modifier_stage.insert(Modifier::RestorePerfect5);
                        }
                        if mod_keys_down.contains(&ModButton::Major7) {
                            modifier_stage.insert(Modifier::AddMajor7);
                        }
                        if mod_keys_down.contains(&ModButton::ChangeKey) {
                            modifier_stage.insert(Modifier::ChangeKey);
                        }
                        if mod_keys_down.contains(&ModButton::Pulse) {
                            modifier_stage.insert(Modifier::Pulse);
                        }

                        let mut pulse = false;

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
                                            transpose = Transpose(nc.root.0 as i16);
                                            if transpose.0 > 6 {
                                                transpose.0 -= 12;
                                            }
                                        }
                                        Modifier::Pulse => {
                                            pulse = true;
                                        }
                                    }
                                }
                            }
                        }

                        // Delayed so as to happen after all note adjustments have been made
                        if pulse {
                            // Play the low root of the new chord
                            play_note(
                                &mut midi_connection,
                                transpose + new_chord.as_ref().unwrap().root,
                                &mut active_notes,
                                VELOCITY,
                            );
                            // Play higher notes of the new chord
                            for i in 12..NUM_STRINGS {
                                let note = UnkeyedNote(i as i16);
                                if is_note_in_chord(note, &new_chord) {
                                    let vel = (VELOCITY * 2 / 3) as u8;
                                    play_note(
                                        &mut midi_connection,
                                        transpose + note,
                                        &mut active_notes,
                                        vel,
                                    );
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
                            if let Some(new) = new_chord {
                                let notes_to_stop: Vec<MidiNote> = active_notes
                                    .iter()
                                    .filter(|&&note| {
                                        is_note_in_chord(note - LOWEST_NOTE - transpose, &Some(new))
                                            == false
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

                        let window_width = physical_size.width as f32;

                        compute_note_positions(&mut note_positions, window_width);

                        // Redraw lines on resize
                        draw_strings(
                            &mut surface,
                            physical_size.width,
                            physical_size.height,
                            &active_chord,
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
                                    &active_chord,
                                    &mut active_notes,
                                    transpose,
                                    &note_positions
                                );
                            }
                        }

                        prev_pos = Some((curr_x, curr_y));
                    }

                    WindowEvent::RedrawRequested => {
                        // Initial draw if needed, though Resized usually handles it on startup
                        let size = window.inner_size();
                        draw_strings(&mut surface, size.width, size.height, &active_chord, &note_positions);
                    }

                    _ => {}
                }
            }
            _ => {}
        }
    })?;

    Ok(())
}

// Decide chord from current chord_keys_down and previous chord state.
fn decide_chord_base(
    old_chord: Option<&Chord>,
    chord_keys_down: &HashSet<ChordButton>,
) -> Option<Chord> {
    if chord_keys_down.contains(&ChordButton::HeptatonicMajor) {
        return Some(build_with(ROOT_I, &[0, 2, 4, 5, 7, 9, 11]));
    }

    const CHORD_BUILDERS: [(ChordButton, UnkeyedNote, fn(UnkeyedNote) -> Chord); 8] = [
        (ChordButton::VII, ROOT_VII, diminished_tri),
        (ChordButton::III, ROOT_III, minor_tri),
        (ChordButton::VI, ROOT_VI, minor_tri),
        (ChordButton::II, ROOT_II, minor_tri),
        (ChordButton::V, ROOT_V, major_tri),
        (ChordButton::I, ROOT_I, major_tri),
        (ChordButton::IV, ROOT_IV, major_tri),
        (ChordButton::VIIB, ROOT_VIIB, major_tri),
    ];

    for (button, root, builder) in CHORD_BUILDERS {
        if chord_keys_down.contains(&button) {
            if let Some(old) = old_chord {
                if old.root == root {
                    return old_chord.copied()
                }
            }
            return Some(builder(root));
        }
    }

    // No keys down: preserve chord if we just went from 1 -> 0
    if let Some(_) = old_chord {
        return old_chord.copied()
    }

    None
}

fn _compute_string_positions(width: f32) -> Vec<f32> {
    let mut positions: Vec<f32> = vec![0.0; NUM_STRINGS];

    for i in 0..NUM_STRINGS {
        positions[i] = UNSCALED_RELATIVE_X_POSITIONS[i] * width;
    }

    positions
}

fn compute_note_positions(positions: &mut [f32], width: f32) {
    let mut i = 0;

    // Add as many notes til we go off the right side of the screen.
    for octave in 0.. {
        for uknote in 0..12 {
            let string_in_octave = NOTE_TO_STRING_IN_OCTAVE[uknote as usize] as usize;
            let string = octave * 7 + string_in_octave;
            if string >= NUM_STRINGS.into() {
                return
            }
            let x = UNSCALED_RELATIVE_X_POSITIONS[string] * width;
            positions[i] = x;
            i+=1;
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
                if let Some(ref mut c) = conn {
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
            if is_note_in_chord(uknote, active_chord) {
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

        // Crossfade between bass and main
        let main_factor = if midi_note <= MAIN_BASS_BOTTOM {
            0.0
        } else if midi_note >= MAIN_BASS_TOP {
            1.0
        } else {
            (midi_note - MAIN_BASS_BOTTOM).ratio(MAIN_BASS_TOP - MAIN_BASS_BOTTOM)
        };
        let bass_factor = 1.0 - main_factor;

        // On second thought, lets give main_factor twice as long of a fade
        let main_factor = 1.0 - 0.5 * (1.0 - main_factor);

        // Scale velocities
        let main_vel = ((velocity as f32) * main_factor).round() as u8;
        let bass_vel = ((velocity as f32) * bass_factor * BASS_VELOCITY_MULTIPLIER).round() as u8;

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

        // Send to main channel and bass
        send_note_on(c, MAIN_CHANNEL, midi_note, main_vel);

        // Send an off to bass first to get a solid rearticulation
        send_note_off(c, BASS_CHANNEL, midi_note);
        send_note_on(c, BASS_CHANNEL, midi_note, bass_vel);

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
        if is_note_in_chord(uknote, active_chord) {
            let x = positions[i].round() as u32;
            if x >= width {
                continue;
            }

            let color = if is_note_root_of_chord(uknote, active_chord) {
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
