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
//!     - Fix held pulse key triggering rapid repeat
//!     - Pulse in wonky in other keys
//!     - Explore additive chords (I + vi = vi7, I + iii = Imaj7)

use bitflags::bitflags;
use midir::os::unix::VirtualOutput;
use midir::{MidiOutput, MidiOutputConnection};
use softbuffer::{Context, Surface};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::num::NonZeroU32;
use std::ops::{Add, Sub};
use std::rc::Rc;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

#[repr(transparent)]
#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq, PartialOrd)]
struct MidiNote(u8);

impl Sub for MidiNote {
    type Output = Interval;
    fn sub(self, rhs: MidiNote) -> Interval {
        Interval(self.0 as i16 - rhs.0 as i16)
    }
}

#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq)]
struct UnbottomedNote(i16); // Note before building on the BOTTOM_NOTE

#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq)]
struct Transpose(i16); // Basically an interval
                       //
impl Transpose {
    fn center_octave(self) -> Transpose {
        if self.0 > 6 {
            Transpose(self.0 - 12)
        } else {
            Transpose(self.0)
        }
    }
}

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
        return MidiNote(sum.clamp(0, 127) as u8);
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
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
struct UnkeyedNote(i16);

impl Sub for UnkeyedNote {
    type Output = Interval;
    fn sub(self, rhs: UnkeyedNote) -> Interval {
        Interval(self.0 - rhs.0)
    }
}

impl UnkeyedNote {
    fn wrap_to_octave(self) -> i16 {
        self.0.rem_euclid(12)
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
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq)]
struct Interval(i16);

impl Interval {
    fn ratio(self, denom: Interval) -> f32 {
        self.0 as f32 / denom.0 as f32
    }
}

// MIDI Note 48 is C3. 48 strings = 4 octaves.
const LOWEST_NOTE: Transpose = Transpose(36); // Do in the active key
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

#[repr(transparent)]
#[derive(Clone, Copy, Eq, PartialEq)]
struct PitchClassSet(u16);

impl PitchClassSet {
    fn contains(&self, pc: UnrootedNote) -> bool {
        self.0 & (1 << pc.0) != 0
    }

    fn insert(&mut self, pc: UnrootedNote) {
        self.0 |= 1 << pc.0;
    }

    fn remove(&mut self, pc: UnrootedNote) {
        self.0 &= !(1 << pc.0);
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
struct Chord {
    // Disable name for now, since this will be better as a debugging tool rather than crucial logic
    //name: &'static str,
    root: UnkeyedNote,
    mask: PitchClassSet, // bits 0..11
}

trait ChordExt {
    fn contains(&self, note: UnkeyedNote) -> bool;
    fn has_root(&self, note: UnkeyedNote) -> bool;
}

impl ChordExt for Option<Chord> {
    fn contains(&self, note: UnkeyedNote) -> bool {
        match self {
            Some(chord) => chord.contains(note),
            None => true,
        }
    }

    fn has_root(&self, note: UnkeyedNote) -> bool {
        self.as_ref().is_some_and(|chord| chord.has_root(note))
    }
}
impl ChordExt for Chord {
    fn contains(&self, note: UnkeyedNote) -> bool {
        let rel = get_note_above_root(note, self.root);
        self.mask.contains(rel)
    }

    fn has_root(&self, note: UnkeyedNote) -> bool {
        note.wrap_to_octave() == self.root.wrap_to_octave()
    }
}

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct Modifiers: u16 {
        const AddMajor2 = 1 << 0;
        const AddMinor7 = 1 << 1;
        const AddMajor7 = 1 << 2;
        const Minor3ToMajor = 1 << 3;
        const RestorePerfect5 = 1 << 4;
        const AddSus4 = 1 << 5;
        const SwitchMinorMajor = 1 << 6;
        const No3 = 1 << 7;
        const ChangeKey = 1 << 8;
        const Pulse = 1 << 9;
    }
}

fn get_note_above_root(note: UnkeyedNote, root: UnkeyedNote) -> UnrootedNote {
    UnrootedNote::new(note - root)
}

fn build_with(root: UnkeyedNote, rels: &[u8]) -> Chord {
    let mut mask = PitchClassSet(0);
    for &r in rels.iter() {
        mask.insert(UnrootedNote(r));
    }
    Chord { root, mask: mask }
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
    VIIB,
    IV,
    I,
    V,
    II,
    VI,
    III,
    VII,
    HeptatonicMajor,
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

fn chord_button_for(key: &winit::keyboard::Key) -> Option<ChordButton> {
    use winit::keyboard::Key::Character;
    use winit::keyboard::Key::Named;
    use winit::keyboard::NamedKey::Control;

    match key {
        Character(s) if s == "a" => Some(ChordButton::VIIB),
        Character(s) if s == "s" => Some(ChordButton::IV),
        Character(s) if s == "d" => Some(ChordButton::I),
        Character(s) if s == "f" => Some(ChordButton::V),
        Character(s) if s == "z" => Some(ChordButton::II),
        Character(s) if s == "x" => Some(ChordButton::VI),
        Character(s) if s == "c" => Some(ChordButton::III),
        Character(s) if s == "v" => Some(ChordButton::VII),
        Named(Control) => Some(ChordButton::HeptatonicMajor),
        _ => None,
    }
}

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
    if let Some(conn) = conn_out.as_mut() {
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
    if let Some(nc) = active_chord.as_mut() {
        nc.mask.insert(UnrootedNote(2));
    }

    let mut active_notes = HashSet::new();
    // Key tracking using named buttons
    let mut chord_keys_down: HashSet<ChordButton> = HashSet::new();
    let mut mod_keys_down: HashSet<ModButton> = HashSet::new();
    // Modifier: modifiers queued and applied on next chord key press
    let mut modifier_stage = Modifiers::empty();
    // Transpose in semitones (0-11) applied to played notes
    let mut transpose: Transpose = Transpose(0);
    // We move conn_out into the event loop
    let mut midi_connection = conn_out;
    let mut note_positions: Vec<f32> = Vec::new();

    let mod_key_map: HashMap<winit::keyboard::Key, (ModButton, Modifiers)> = [
        (
            winit::keyboard::Key::Character("5".into()),
            (ModButton::Major2, Modifiers::AddMajor2),
        ),
        (
            winit::keyboard::Key::Character("b".into()),
            (ModButton::Major7, Modifiers::AddMajor7),
        ),
        (
            winit::keyboard::Key::Character("6".into()),
            (ModButton::Minor7, Modifiers::AddMinor7),
        ),
        (
            winit::keyboard::Key::Character("3".into()),
            (ModButton::Sus4, Modifiers::AddSus4),
        ),
        (
            winit::keyboard::Key::Character("4".into()),
            (ModButton::MinorMajor, Modifiers::SwitchMinorMajor),
        ),
        (
            winit::keyboard::Key::Character(".".into()),
            (ModButton::No3, Modifiers::No3),
        ),
        (
            winit::keyboard::Key::Character("1".into()),
            (ModButton::ChangeKey, Modifiers::ChangeKey),
        ),
        (
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::Tab),
            (ModButton::Pulse, Modifiers::Pulse),
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
                            if let Some(button) = chord_button_for(&event.logical_key) {
                                if !chord_keys_down.contains(&button) {
                                    chord_keys_down.insert(button);
                                    chord_was_pressed = true;
                                }
                            } else if let Some((button, modifier)) =
                                mod_key_map.get(&event.logical_key)
                            {
                                if !mod_keys_down.contains(button) {
                                    mod_keys_down.insert(*button);
                                    modifier_stage.insert(*modifier);
                                }
                            }
                        } else {
                            // Released
                            if let Some(button) = chord_button_for(&event.logical_key) {
                                chord_keys_down.remove(&button);
                            } else if let Some((button, _)) = mod_key_map.get(&event.logical_key) {
                                mod_keys_down.remove(button);
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
                                modifier_stage.insert(Modifiers::AddMinor7);
                                modifier_stage.insert(Modifiers::Minor3ToMajor);
                                modifier_stage.insert(Modifiers::RestorePerfect5);
                            }
                        }

                        // Inserting here supports held mods
                        if mod_keys_down.contains(&ModButton::Major2) {
                            modifier_stage.insert(Modifiers::AddMajor2);
                        }
                        if mod_keys_down.contains(&ModButton::Minor7) {
                            modifier_stage.insert(Modifiers::AddMinor7);
                        }
                        if mod_keys_down.contains(&ModButton::Sus4) {
                            modifier_stage.insert(Modifiers::AddSus4);
                        }
                        if mod_keys_down.contains(&ModButton::MinorMajor) {
                            modifier_stage.insert(Modifiers::SwitchMinorMajor);
                        }
                        if mod_keys_down.contains(&ModButton::No3) {
                            modifier_stage.insert(Modifiers::RestorePerfect5);
                        }
                        if mod_keys_down.contains(&ModButton::Major7) {
                            modifier_stage.insert(Modifiers::AddMajor7);
                        }
                        if mod_keys_down.contains(&ModButton::ChangeKey) {
                            modifier_stage.insert(Modifiers::ChangeKey);
                        }
                        if mod_keys_down.contains(&ModButton::Pulse) {
                            modifier_stage.insert(Modifiers::Pulse);
                        }

                        // If there are modifiers queued and a chord key is down, apply them now to
                        // the freshly constructed chord, then remove it.
                        if !modifier_stage.is_empty() {
                            if let Some(nc) = new_chord.as_mut() {
                                if modifier_stage.contains(Modifiers::AddMajor2) {
                                    nc.mask.insert(UnrootedNote(2));
                                }
                                if modifier_stage.contains(Modifiers::AddMinor7) {
                                    nc.mask.insert(UnrootedNote(10));
                                }
                                if modifier_stage.contains(Modifiers::Minor3ToMajor) {
                                    nc.mask.remove(UnrootedNote(3));
                                    nc.mask.insert(UnrootedNote(4));
                                }
                                if modifier_stage.contains(Modifiers::AddSus4) {
                                    // Remove major/minor third (bits 3 and 4) and add perfect 4th (bit 5)
                                    nc.mask.remove(UnrootedNote(3));
                                    nc.mask.remove(UnrootedNote(4));
                                    nc.mask.insert(UnrootedNote(5));
                                }
                                if modifier_stage.contains(Modifiers::AddMajor7) {
                                    // Add major 7th (interval 11)
                                    nc.mask.insert(UnrootedNote(11));
                                }
                                if modifier_stage.contains(Modifiers::SwitchMinorMajor) {
                                    // Based on root to be stable on multiple runs
                                    if nc.root == ROOT_II
                                        || nc.root == ROOT_III
                                        || nc.root == ROOT_VI
                                        || nc.root == ROOT_VII
                                    {
                                        // Change minor tri to major tri
                                        nc.mask.remove(UnrootedNote(3));
                                        nc.mask.insert(UnrootedNote(4));
                                    } else {
                                        // Change major tri to minor tri
                                        nc.mask.remove(UnrootedNote(4));
                                        nc.mask.insert(UnrootedNote(3));
                                    }
                                }
                                if modifier_stage.contains(Modifiers::No3) {
                                    // Remove both major and minor 3rd
                                    nc.mask.remove(UnrootedNote(3));
                                    nc.mask.remove(UnrootedNote(4));
                                }
                                if modifier_stage.contains(Modifiers::RestorePerfect5) {
                                    nc.mask.remove(UnrootedNote(6));
                                    nc.mask.remove(UnrootedNote(8));
                                    nc.mask.insert(UnrootedNote(7))
                                }
                                if modifier_stage.contains(Modifiers::ChangeKey) {
                                    transpose = Transpose(nc.root.0 as i16).center_octave()
                                }
                                if modifier_stage.contains(Modifiers::Pulse) {
                                    // Play the low root of the new chord
                                    play_note(
                                        &mut midi_connection,
                                        transpose + nc.root,
                                        &mut active_notes,
                                        VELOCITY,
                                    );
                                    // Play higher notes of the new chord
                                    for i in 12..NUM_STRINGS {
                                        let note = UnkeyedNote(i as i16);
                                        if nc.contains(note) {
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
                            }
                        }
                        modifier_stage = Modifiers::empty();

                        // If the notes aren't the same, do the switch
                        if old_chord != new_chord.as_ref() {
                            // Stop any playing notes that are not in the new chord
                            let notes_to_stop: Vec<MidiNote> = active_notes
                                .iter()
                                .filter(|&&note| {
                                    !new_chord.contains(note - LOWEST_NOTE - transpose)
                                })
                                .cloned()
                                .collect();
                            for note in notes_to_stop {
                                stop_note(&mut midi_connection, note, &mut active_notes);
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

                        recompute_note_positions(&mut note_positions, window_width);

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
                            &active_chord,
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
                    return old_chord.copied();
                }
            }
            return Some(builder(root));
        }
    }

    // No keys down: preserve chord if we just went from 1 -> 0
    if let Some(_) = old_chord {
        return old_chord.copied();
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
            if active_chord.contains(uknote) {
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
        if active_chord.contains(uknote) {
            let x = positions[i].round() as u32;
            if x >= width {
                continue;
            }

            let color = if active_chord.has_root(uknote) {
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
