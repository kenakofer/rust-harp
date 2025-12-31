mod effects;
mod events;

pub use effects::{AppEffects, BendNote, NoteOn};
pub use events::{
    ActionButton, Actions, ChordButton, KeyEvent, KeyState, ModButton, DEFAULT_STRUM_VOLUME,
};

use crate::chord::{Chord, Modifiers};
use crate::notes::{NoteVolume, Transpose, UnkeyedNote, UnmidiNote};
use std::collections::HashSet;

const PULSE_VOLUME: NoteVolume = NoteVolume(50);

const ROOT_VIIB: UnkeyedNote = UnkeyedNote(10);
const ROOT_IV: UnkeyedNote = UnkeyedNote(5);
const ROOT_I: UnkeyedNote = UnkeyedNote(0);
const ROOT_V: UnkeyedNote = UnkeyedNote(7);
const ROOT_II: UnkeyedNote = UnkeyedNote(2);
const ROOT_VI: UnkeyedNote = UnkeyedNote(9);
const ROOT_III: UnkeyedNote = UnkeyedNote(4);
const ROOT_VII: UnkeyedNote = UnkeyedNote(11);

const TOP_ROW: usize = 0;

pub struct AppState {
    row_specs: Vec<crate::layout::RowSpec>,

    pub active_chord: Chord, // Top row chord. TODO privatize
    pub active_notes: HashSet<UnmidiNote>,
    active_notes_by_row: Vec<HashSet<UnmidiNote>>,

    chord_keys_down: HashSet<ChordButton>,
    mod_keys_down: HashSet<ModButton>,
    action_keys_down: HashSet<ActionButton>,

    // Android app uses a swipe-wheel to set these modifiers persistently.
    wheel_modifiers: Modifiers,
    wheel_modifiers_dirty: bool,

    allow_implied_sevenths: bool,

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
        let row_specs = crate::layout::default_row_specs();
        let nrows = row_specs.len();
        Self {
            row_specs,
            active_chord: Chord::new_triad(ROOT_I),
            active_notes: HashSet::new(),
            active_notes_by_row: (0..nrows).map(|_| HashSet::new()).collect(),

            chord_keys_down: HashSet::new(),
            mod_keys_down: HashSet::new(),
            action_keys_down: HashSet::new(),

            wheel_modifiers: Modifiers::empty(),
            wheel_modifiers_dirty: false,

            allow_implied_sevenths: true,

            modifier_stage: Modifiers::empty(),
            action_stage: Actions::empty(),

            transpose: Transpose(0),
        }
    }

    pub fn set_allow_implied_sevenths(&mut self, enabled: bool) {
        self.allow_implied_sevenths = enabled;
    }

    pub fn set_wheel_modifiers(&mut self, modifiers: Modifiers) {
        if self.wheel_modifiers != modifiers {
            self.wheel_modifiers = modifiers;
            self.wheel_modifiers_dirty = true;
        }
    }

    pub fn toggle_wheel_minor_major(&mut self) {
        let mm = Modifiers::SwitchMinorMajor;
        if self.wheel_modifiers.contains(mm) {
            self.set_wheel_modifiers(self.wheel_modifiers - mm);
        } else {
            self.set_wheel_modifiers(self.wheel_modifiers | mm);
        }
    }

    pub fn set_transpose(&mut self, transpose: Transpose) -> AppEffects {
        let t = transpose.center_octave();
        let effects = AppEffects {
            redraw: true,
            change_key: Some(t),
            stop_notes: self.active_notes.iter().cloned().collect(),
            stop_bend_pointers: Vec::new(),
            play_notes: Vec::new(),
            bend_notes: Vec::new(),
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

    pub fn row_specs(&self) -> &[crate::layout::RowSpec] {
        &self.row_specs
    }

    pub fn chord_for_row(&self, row: crate::rows::RowIndex) -> Chord {
        crate::layout::chord_for_row(&self.row_specs, row, self.active_chord)
    }

    pub fn row_chords(&self) -> Vec<Chord> {
        (0..self.row_specs.len())
            .map(|i| crate::layout::chord_for_row(&self.row_specs, i, self.active_chord))
            .collect()
    }

    pub fn mod_button_down(&self, button: ModButton) -> bool {
        self.mod_keys_down.contains(&button)
    }

    pub fn handle_key_event(&mut self, event: KeyEvent) -> AppEffects {
        let mut effects = AppEffects {
            redraw: true,
            change_key: None,
            stop_notes: Vec::new(),
            stop_bend_pointers: Vec::new(),
            play_notes: Vec::new(),
            bend_notes: Vec::new(),
        };

        if let KeyEvent::StrumCrossing { row, note, volume } = event {
            effects.redraw = false;
            let chord = self.chord_for_row(row);
            if chord.contains(note) {
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
                if let Some(s) = self.active_notes_by_row.get_mut(row) {
                    s.insert(un);
                }
                effects.play_notes.push(NoteOn {
                    note: un,
                    volume,
                    pointer: None,
                });
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

        let venerated_old_chord = if chord_was_pressed || self.wheel_modifiers_dirty {
            None
        } else {
            Some(self.active_chord)
        };
        let mut new_chord = decide_chord_base(
            venerated_old_chord.as_ref(),
            &self.chord_keys_down,
            self.allow_implied_sevenths,
        );
        self.wheel_modifiers_dirty = false;

        // Apply held modifiers
        for entry in MOD_BUTTON_TABLE.iter() {
            if self.mod_keys_down.contains(&entry.button) {
                self.modifier_stage.insert(entry.modifiers);
            }
        }

        // Apply persistent wheel modifiers (Android chord swipe-wheel).
        if !self.chord_keys_down.contains(&ChordButton::HeptatonicMajor) {
            self.modifier_stage.insert(self.wheel_modifiers);
        }

        if !self.modifier_stage.is_empty() {
            new_chord.add_mods_now(self.modifier_stage);
        }

        let chord_changed = venerated_old_chord.map_or(true, |old| old != new_chord);
        if chord_changed {
            effects.redraw = true;
            self.active_chord = new_chord;

            effects.stop_notes = (0..128)
                .map(|i| UnmidiNote(i))
                .filter(|un| !self.active_chord.contains(*un - self.transpose))
                .filter(|un| self
                    .active_notes_by_row
                    .get(TOP_ROW)
                    .map_or(false, |s| s.contains(un)))
                .collect();

            for un in effects.stop_notes.iter() {
                self.active_notes.remove(un);
                if let Some(s) = self.active_notes_by_row.get_mut(TOP_ROW) {
                    s.remove(un);
                }
            }
        }

        let chord = &mut self.active_chord;
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
                    if let Some(s) = self.active_notes_by_row.get_mut(TOP_ROW) {
                        s.insert(un);
                    }
                    effects.play_notes.push(NoteOn {
                        note: un,
                        volume: PULSE_VOLUME,
                        pointer: None,
                    });
                });
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

// Decide chord from current chord_keys_down and previous chord state.
fn decide_chord_base(
    venerated_old_chord: Option<&Chord>,
    chord_keys_down: &HashSet<ChordButton>,
    allow_implied_sevenths: bool,
) -> Chord {
    if chord_keys_down.contains(&ChordButton::HeptatonicMajor) {
        return heptatonic_major_chord();
    }

    // Check/apply double-held-chord sevenths
    if allow_implied_sevenths {
        if let Some(root) = detect_implied_minor7_root(chord_keys_down) {
            return Chord::new(root, Modifiers::MajorTri | Modifiers::AddMinor7);
        }
    }

    for entry in CHORD_BUTTON_TABLE.iter() {
        if chord_keys_down.contains(&entry.button) {
            if let Some(old) = venerated_old_chord {
                if old.get_root() == entry.root {
                    return *old;
                }
            }
            return Chord::new_triad(entry.root);
        }
    }

    // Should be unreachable (we return early when no chord keys are down), but keep a safe fallback.
    if let Some(old) = venerated_old_chord {
        return *old;
    }

    Chord::chromatic(ROOT_I)
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

        let chord = state.active_chord;
        assert_eq!(chord.get_root(), ROOT_V);
    }

    #[test]
    fn modifier_applies_to_next_chord() {
        let mut state = AppState::new();

        press_modifier(&mut state, ModButton::Minor7, Modifiers::AddMinor7);
        press_chord(&mut state, ChordButton::I);

        let chord = state.active_chord;
        assert!(chord.contains(UnkeyedNote(10))); // minor 7
    }

    #[test]
    fn fixed_rows_have_expected_note_sets() {
        let state = AppState::new();

        let major = state.chord_for_row(1);
        for pc in 0..12 {
            let n = UnkeyedNote(pc);
            let exp = matches!(pc, 0 | 2 | 4 | 5 | 7 | 9 | 11);
            assert_eq!(major.contains(n), exp);
        }

        let comp = state.chord_for_row(2);
        for pc in 0..12 {
            let n = UnkeyedNote(pc);
            assert_eq!(comp.contains(n), !major.contains(n));
        }

        let chrom = state.chord_for_row(3);
        for pc in 0..12 {
            assert!(chrom.contains(UnkeyedNote(pc)));
        }
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
            row: 0,
            note: UnkeyedNote(4),
            volume: DEFAULT_STRUM_VOLUME,
        });

        assert_eq!(
            effects.play_notes,
            vec![NoteOn {
                note: UnmidiNote(16),
                volume: DEFAULT_STRUM_VOLUME,
                pointer: None,
            }]
        );
        assert!(state.active_notes.contains(&UnmidiNote(16)));
    }

    #[test]
    fn strum_crossing_outside_chord_is_filtered_out() {
        let mut state = AppState::new();
        state.transpose = Transpose(12);

        let effects = state.handle_key_event(KeyEvent::StrumCrossing {
            row: 0,
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
            row: 0,
            note: UnkeyedNote(0),
            volume: DEFAULT_STRUM_VOLUME,
        });
        let effects2 = state.handle_key_event(KeyEvent::StrumCrossing {
            row: 0,
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
            row: 0,
            note: UnkeyedNote(0),
            volume: DEFAULT_STRUM_VOLUME,
        });
        state.handle_key_event(KeyEvent::StrumCrossing {
            row: 0,
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
