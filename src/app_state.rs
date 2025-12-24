use crate::chord::{Chord, Modifiers};
use crate::notes::{NoteVolume, Transpose, UnkeyedNote, UnmidiNote};
use std::collections::HashSet;

use bitflags::bitflags;

pub const DEFAULT_STRUM_VOLUME: NoteVolume = NoteVolume(70);
const PULSE_VOLUME: NoteVolume = NoteVolume(50);

const ROOT_VIIB: UnkeyedNote = UnkeyedNote(10);
const ROOT_IV: UnkeyedNote = UnkeyedNote(5);
const ROOT_I: UnkeyedNote = UnkeyedNote(0);
const ROOT_V: UnkeyedNote = UnkeyedNote(7);
const ROOT_II: UnkeyedNote = UnkeyedNote(2);
const ROOT_VI: UnkeyedNote = UnkeyedNote(9);
const ROOT_III: UnkeyedNote = UnkeyedNote(4);
const ROOT_VII: UnkeyedNote = UnkeyedNote(11);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyState {
    Pressed,
    Released,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyEvent {
    Chord {
        state: KeyState,
        button: ChordButton,
    },
    Modifier {
        state: KeyState,
        button: ModButton,
        modifiers: Modifiers,
    },
    Action {
        state: KeyState,
        button: ActionButton,
        action: Actions,
    },
    StrumCrossing {
        row: crate::rows::RowId,
        note: UnkeyedNote,
        /// Touch/strum intensity snapshot at note-on.
        volume: NoteVolume,
    },
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ChordButton {
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
pub enum ModButton {
    Major2,
    Minor7,
    Major7,
    Sus4,
    MinorMajor,
    No3,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ActionButton {
    ChangeKey,
    Pulse,
}

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct Actions: u16 {
        const Pulse = 1 << 0;
        const ChangeKey = 1 << 1;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoteOn {
    pub note: UnmidiNote,
    pub volume: NoteVolume,
}

#[derive(Debug)]
pub struct AppEffects {
    pub play_notes: Vec<NoteOn>,
    pub stop_notes: Vec<UnmidiNote>,
    pub redraw: bool,
    pub change_key: Option<Transpose>,
}

pub struct AppState {
    pub active_chord: Option<Chord>, // Top row chord. TODO privatize
    pub active_notes: HashSet<UnmidiNote>,
    active_notes_by_row: [HashSet<UnmidiNote>; 3],

    bottom_chord: Chord,

    chord_keys_down: HashSet<ChordButton>,
    mod_keys_down: HashSet<ModButton>,
    action_keys_down: HashSet<ActionButton>,

    modifier_stage: Modifiers,
    action_stage: Actions,

    pub transpose: Transpose, //TODO privatize
}

struct ChordButtonTableEntry {
    root: UnkeyedNote,
    button: ChordButton,
}

const CHORD_BUTTON_TABLE: [ChordButtonTableEntry; 9] = [
    ChordButtonTableEntry {
        root: ROOT_VIIB,
        button: ChordButton::VIIB,
    },
    ChordButtonTableEntry {
        root: ROOT_IV,
        button: ChordButton::IV,
    },
    ChordButtonTableEntry {
        root: ROOT_I,
        button: ChordButton::I,
    },
    ChordButtonTableEntry {
        root: ROOT_V,
        button: ChordButton::V,
    },
    ChordButtonTableEntry {
        root: ROOT_II,
        button: ChordButton::II,
    },
    ChordButtonTableEntry {
        root: ROOT_VI,
        button: ChordButton::VI,
    },
    ChordButtonTableEntry {
        root: ROOT_III,
        button: ChordButton::III,
    },
    ChordButtonTableEntry {
        root: ROOT_VII,
        button: ChordButton::VII,
    },
    ChordButtonTableEntry {
        root: ROOT_I,
        button: ChordButton::HeptatonicMajor,
    },
];

struct ModButtonTableEntry {
    button: ModButton,
    modifiers: Modifiers,
}

const MOD_BUTTON_TABLE: [ModButtonTableEntry; 6] = [
    ModButtonTableEntry {
        button: ModButton::Major2,
        modifiers: Modifiers::AddMajor2,
    },
    ModButtonTableEntry {
        button: ModButton::Major7,
        modifiers: Modifiers::AddMajor7,
    },
    ModButtonTableEntry {
        button: ModButton::Minor7,
        modifiers: Modifiers::AddMinor7,
    },
    ModButtonTableEntry {
        button: ModButton::Sus4,
        modifiers: Modifiers::Sus4,
    },
    ModButtonTableEntry {
        button: ModButton::MinorMajor,
        modifiers: Modifiers::SwitchMinorMajor,
    },
    ModButtonTableEntry {
        button: ModButton::No3,
        modifiers: Modifiers::No3,
    },
];

impl AppState {
    pub fn new() -> Self {
        Self {
            active_chord: Some(Chord::new_triad(ROOT_I)),
            active_notes: HashSet::new(),
            active_notes_by_row: std::array::from_fn(|_| HashSet::new()),

            bottom_chord: heptatonic_major_chord(),

            chord_keys_down: HashSet::new(),
            mod_keys_down: HashSet::new(),
            action_keys_down: HashSet::new(),

            modifier_stage: Modifiers::empty(),
            action_stage: Actions::empty(),

            transpose: Transpose(0),
        }
    }

    pub fn set_transpose(&mut self, transpose: Transpose) -> AppEffects {
        let t = transpose.center_octave();
        let effects = AppEffects {
            redraw: true,
            change_key: Some(t),
            stop_notes: self.active_notes.iter().cloned().collect(),
            play_notes: Vec::new(),
        };

        self.active_notes.clear();
        for s in self.active_notes_by_row.iter_mut() {
            s.clear();
        }
        self.transpose = t;

        effects
    }

    pub fn chord_button_down(&self, button: ChordButton) -> bool {
        self.chord_keys_down.contains(&button)
    }

    pub fn active_chord_for_row(&self, row: crate::rows::RowId) -> Option<Chord> {
        match row {
            crate::rows::RowId::Top => self.active_chord,
            crate::rows::RowId::Middle => Some(self.bottom_chord),
            // Bottom row is chromatic: all 12 notes are enabled.
            // Represent this as None so filtering allows everything.
            crate::rows::RowId::Bottom => None,
        }
    }

    pub fn mod_button_down(&self, button: ModButton) -> bool {
        self.mod_keys_down.contains(&button)
    }

    pub fn handle_key_event(&mut self, event: KeyEvent) -> AppEffects {
        let mut effects = AppEffects {
            redraw: true,
            change_key: None,
            stop_notes: Vec::new(),
            play_notes: Vec::new(),
        };

        if let KeyEvent::StrumCrossing { row, note, volume } = event {
            effects.redraw = false;
            let chord = match row {
                crate::rows::RowId::Top => self.active_chord,
                crate::rows::RowId::Middle => Some(self.bottom_chord),
                crate::rows::RowId::Bottom => None,
            };
            if chord.map_or(true, |c| c.contains(note)) {
                let un = self.transpose + note;

                // If this note is already active, stop it first so we only ever have one
                // instance playing at a time.
                if self.active_notes.remove(&un) {
                    effects.stop_notes.push(un);
                    for s in self.active_notes_by_row.iter_mut() {
                        s.remove(&un);
                    }
                }

                self.active_notes.insert(un);
                self.active_notes_by_row[row.index()].insert(un);
                effects.play_notes.push(NoteOn { note: un, volume });
            }
            return effects;
        }

        let mut chord_was_pressed = false;

        match event {
            KeyEvent::Chord { state, button } => match state {
                KeyState::Pressed => {
                    if self.chord_keys_down.insert(button) {
                        chord_was_pressed = true;
                    }
                }
                KeyState::Released => {
                    self.chord_keys_down.remove(&button);
                }
            },

            KeyEvent::Modifier {
                state,
                button,
                modifiers,
            } => match state {
                KeyState::Pressed => {
                    if self.mod_keys_down.insert(button) {
                        self.modifier_stage.insert(modifiers);
                    }
                }
                KeyState::Released => {
                    self.mod_keys_down.remove(&button);
                }
            },

            KeyEvent::Action {
                state,
                button,
                action,
            } => match state {
                KeyState::Pressed => {
                    if self.action_keys_down.insert(button) {
                        self.action_stage.insert(action);
                    }
                }
                KeyState::Released => {
                    self.action_keys_down.remove(&button);
                }
            },
            KeyEvent::StrumCrossing { .. } => unreachable!(),
        }

        if self.chord_keys_down.is_empty() {
            return effects;
        }

        let venerated_old_chord = if chord_was_pressed {
            None
        } else {
            self.active_chord
        };
        let mut new_chord = decide_chord_base(venerated_old_chord.as_ref(), &self.chord_keys_down);

        // Apply held modifiers
        for entry in MOD_BUTTON_TABLE.iter() {
            if self.mod_keys_down.contains(&entry.button) {
                self.modifier_stage.insert(entry.modifiers);
            }
        }

        if let Some(ref mut chord) = new_chord {
            if !self.modifier_stage.is_empty() {
                chord.add_mods_now(self.modifier_stage);
            }
        }

        if venerated_old_chord != new_chord {
            effects.redraw = true;
            self.active_chord = new_chord;

            if let Some(chord) = new_chord {
                self.bottom_chord = dynamic_heptatonic_for_active_chord(&chord);

                effects.stop_notes = (0..128)
                    .map(|i| UnmidiNote(i))
                    .filter(|un| !chord.contains(*un - self.transpose))
                    .filter(|un| self.active_notes_by_row[crate::rows::RowId::Top.index()].contains(un))
                    .collect();

                for un in effects.stop_notes.iter() {
                    self.active_notes.remove(un);
                    self.active_notes_by_row[crate::rows::RowId::Top.index()].remove(un);
                }
            }
        }

        if let Some(ref mut chord) = self.active_chord {
            if self.action_stage.contains(Actions::ChangeKey) {
                self.transpose = Transpose(chord.get_root().as_i16()).center_octave();
                effects.change_key = Some(self.transpose);
            }
            if self.action_stage.contains(Actions::Pulse) {
                (-12..70)
                    .map(|i| UnmidiNote(i))
                    .filter(|un| chord.contains(*un - self.transpose))
                    .for_each(|un| {
                        if self.active_notes.remove(&un) {
                            effects.stop_notes.push(un);
                            for s in self.active_notes_by_row.iter_mut() {
                                s.remove(&un);
                            }
                        }
                        self.active_notes.insert(un);
                        self.active_notes_by_row[crate::rows::RowId::Top.index()].insert(un);
                        effects.play_notes.push(NoteOn {
                            note: un,
                            volume: PULSE_VOLUME,
                        });
                    });
            }
        }

        self.modifier_stage = Modifiers::empty();
        self.action_stage = Actions::empty();

        effects
    }
}

fn chord_root_for(button: ChordButton) -> Option<UnkeyedNote> {
    CHORD_BUTTON_TABLE
        .iter()
        .find(|e| e.button == button)
        .map(|e| e.root)
}

fn detect_implied_minor7_root(chord_keys_down: &HashSet<ChordButton>) -> Option<UnkeyedNote> {
    use ChordButton::*;

    let pairs = [
        (VI, II),
        (III, VI),
        (VII, III),
        (I, IV),
        (IV, VIIB),
        (V, I),
        (II, V),
    ];

    for (a, b) in pairs {
        if chord_keys_down.contains(&a) && chord_keys_down.contains(&b) {
            //Set the root
            return chord_root_for(a);
        }
    }
    None
}

fn heptatonic_major_chord_root(root: UnkeyedNote) -> Chord {
    Chord::new(
        root,
        Modifiers::MajorTri
            | Modifiers::AddMajor2
            | Modifiers::Add4
            | Modifiers::AddMajor6
            | Modifiers::AddMajor7,
    )
}

fn heptatonic_major_chord() -> Chord {
    heptatonic_major_chord_root(ROOT_I)
}

fn chord_pitch_classes(chord: &Chord) -> [bool; 12] {
    let mut pcs = [false; 12];
    for pc in 0..12 {
        pcs[pc] = chord.contains(UnkeyedNote(pc as i16));
    }
    pcs
}

/// Choose a major heptatonic (diatonic) scale root such that the resulting
/// heptatonic contains all notes in the active chord.
///
/// We search outward in both directions around the circle of fifths from the
/// current key (which is always `UnkeyedNote(0)` in our unkeyed coordinate space).
fn choose_heptatonic_root_for_active_chord(active: &Chord) -> UnkeyedNote {
    let needed = chord_pitch_classes(active);

    for k in 0..12 {
        let offset = if k == 0 {
            0
        } else {
            let n = ((k + 1) / 2) as i16;
            let sign = if k % 2 == 1 { 1 } else { -1 };
            sign * 7 * n
        };

        let root_pc = offset.rem_euclid(12);
        let root = UnkeyedNote(root_pc);
        let hept = heptatonic_major_chord_root(root);

        let mut ok = true;
        for pc in 0..12 {
            if needed[pc] && !hept.contains(UnkeyedNote(pc as i16)) {
                ok = false;
                break;
            }
        }
        if ok {
            return root;
        }
    }

    ROOT_I
}

fn dynamic_heptatonic_for_active_chord(active: &Chord) -> Chord {
    heptatonic_major_chord_root(choose_heptatonic_root_for_active_chord(active))
}

// Decide chord from current chord_keys_down and previous chord state.
fn decide_chord_base(
    venerated_old_chord: Option<&Chord>,
    chord_keys_down: &HashSet<ChordButton>,
) -> Option<Chord> {
    if chord_keys_down.contains(&ChordButton::HeptatonicMajor) {
        return Some(heptatonic_major_chord());
    }

    // Check/apply double-held-chord sevenths
    if let Some(root) = detect_implied_minor7_root(chord_keys_down) {
        return Some(Chord::new(root, Modifiers::MajorTri | Modifiers::AddMinor7));
    }

    for entry in CHORD_BUTTON_TABLE.iter() {
        if chord_keys_down.contains(&entry.button) {
            if let Some(old) = venerated_old_chord {
                if old.get_root() == entry.root {
                    return venerated_old_chord.copied();
                }
            }
            return Some(Chord::new_triad(entry.root));
        }
    }

    // No keys down: preserve chord if we just went from 1 -> 0
    if let Some(_) = venerated_old_chord {
        return venerated_old_chord.copied();
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notes::UnmidiNote;

    fn press_chord(state: &mut AppState, button: ChordButton) -> AppEffects {
        state.handle_key_event(KeyEvent::Chord {
            state: KeyState::Pressed,
            button,
        })
    }

    fn press_modifier(state: &mut AppState, button: ModButton, modifiers: Modifiers) {
        state.handle_key_event(KeyEvent::Modifier {
            state: KeyState::Pressed,
            button,
            modifiers,
        });
    }

    #[test]
    fn pressing_chord_sets_active_chord() {
        let mut state = AppState::new();

        press_chord(&mut state, ChordButton::V);

        let chord = state.active_chord.unwrap();
        assert_eq!(chord.get_root(), ROOT_V);
    }

    #[test]
    fn modifier_applies_to_next_chord() {
        let mut state = AppState::new();

        press_modifier(
            &mut state,
            ModButton::Minor7,
            Modifiers::AddMinor7,
        );
        press_chord(&mut state, ChordButton::I);

        let chord = state.active_chord.unwrap();
        assert!(chord.contains(UnkeyedNote(10))); // minor 7
    }

    #[test]
    fn dynamic_heptatonic_contains_active_chord() {
        // E major triad: contains G# (pc 8), which is outside the default C-major heptatonic.
        let chord = Chord::new(UnkeyedNote(4), Modifiers::MajorTri);
        let hept = dynamic_heptatonic_for_active_chord(&chord);

        for pc in 0..12 {
            let n = UnkeyedNote(pc);
            if chord.contains(n) {
                assert!(hept.contains(n));
            }
        }

        // Deterministic choice: the closest-by-fifths heptatonic that contains E major is A major.
        assert_eq!(hept.get_root().wrap_to_octave(), 9);
    }

    #[test]
    fn change_key_sets_transpose() {
        let mut state = AppState::new();

        let effects = state.handle_key_event(KeyEvent::Action {
            state: KeyState::Pressed,
            button: ActionButton::ChangeKey,
            action: Actions::ChangeKey,
        });

        // No chord yet, no key change
        assert!(effects.change_key.is_none());

        // Now key change has been enqueued, the next chord button will change it:
        let effects = press_chord(&mut state, ChordButton::V);
        assert_eq!(effects.change_key, Some(Transpose(-5)));
        assert_eq!(state.transpose, Transpose(-5));

        // Reset all keypresses
        let mut state = AppState::new();

        // Chord first, no key change
        let effects = press_chord(&mut state, ChordButton::III);
        assert!(effects.change_key.is_none());

        // Changekey button, key change
        let effects = state.handle_key_event(KeyEvent::Action {
            state: KeyState::Pressed,
            button: ActionButton::ChangeKey,
            action: Actions::ChangeKey,
        });
        assert_eq!(effects.change_key, Some(Transpose(4)));
        assert_eq!(state.transpose, Transpose(4));
    }

    #[test]
    fn stop_notes_only_returns_active_notes() {
        let mut state = AppState::new();

        state.active_notes.insert(UnmidiNote(0));
        state.active_notes.insert(UnmidiNote(1));

        press_chord(&mut state, ChordButton::I);

        let effects = state.handle_key_event(KeyEvent::Chord {
            state: KeyState::Pressed,
            button: ChordButton::V,
        });

        assert!(effects.stop_notes.len() <= 2);
    }

    #[test]
    fn strum_crossing_in_chord_returns_note_and_records_active() {
        let mut state = AppState::new();
        state.transpose = Transpose(12);

        let effects = state.handle_key_event(KeyEvent::StrumCrossing {
            row: crate::rows::RowId::Top,
            note: UnkeyedNote(4),
            volume: DEFAULT_STRUM_VOLUME,
        });

        assert_eq!(
            effects.play_notes,
            vec![NoteOn {
                note: UnmidiNote(16),
                volume: DEFAULT_STRUM_VOLUME,
            }]
        );
        assert!(state.active_notes.contains(&UnmidiNote(16)));
    }

    #[test]
    fn strum_crossing_outside_chord_is_filtered_out() {
        let mut state = AppState::new();
        state.transpose = Transpose(12);

        let effects = state.handle_key_event(KeyEvent::StrumCrossing {
            row: crate::rows::RowId::Top,
            note: UnkeyedNote(3),
            volume: DEFAULT_STRUM_VOLUME,
        });

        assert!(effects.play_notes.is_empty());
        assert!(state.active_notes.is_empty());
    }

    #[test]
    fn repeated_strum_does_not_duplicate_active_notes() {
        let mut state = AppState::new();

        let effects1 = state.handle_key_event(KeyEvent::StrumCrossing {
            row: crate::rows::RowId::Top,
            note: UnkeyedNote(0),
            volume: DEFAULT_STRUM_VOLUME,
        });
        let effects2 = state.handle_key_event(KeyEvent::StrumCrossing {
            row: crate::rows::RowId::Top,
            note: UnkeyedNote(0),
            volume: DEFAULT_STRUM_VOLUME,
        });

        assert_eq!(effects1.play_notes.len(), 1);
        assert!(effects1.stop_notes.is_empty());

        // Retrigger: we should stop then play, so there is still only one active instance.
        assert_eq!(effects2.play_notes.len(), 1);
        assert_eq!(effects2.stop_notes, vec![UnmidiNote(0)]);
        assert_eq!(state.active_notes.len(), 1); // HashSet: no duplicates
    }

    #[test]
    fn chord_change_stops_and_clears_active_notes() {
        let mut state = AppState::new();

        state.handle_key_event(KeyEvent::StrumCrossing {
            row: crate::rows::RowId::Top,
            note: UnkeyedNote(0),
            volume: DEFAULT_STRUM_VOLUME,
        });
        state.handle_key_event(KeyEvent::StrumCrossing {
            row: crate::rows::RowId::Top,
            note: UnkeyedNote(4),
            volume: DEFAULT_STRUM_VOLUME,
        });

        assert!(state.active_notes.contains(&UnmidiNote(0)));
        assert!(state.active_notes.contains(&UnmidiNote(4)));

        let effects = press_chord(&mut state, ChordButton::V);

        assert!(effects.stop_notes.contains(&UnmidiNote(0)));
        assert!(effects.stop_notes.contains(&UnmidiNote(4)));
        assert!(!state.active_notes.contains(&UnmidiNote(0)));
        assert!(!state.active_notes.contains(&UnmidiNote(4)));
    }
}

