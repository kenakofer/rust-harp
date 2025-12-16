use crate::chord::{Chord, Modifiers};
use crate::notes::{Transpose, UnkeyedNote, UnmidiNote};
use std::collections::HashSet;

use bitflags::bitflags;

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

pub struct AppEffects {
    // pub play_notes: Vec<(UnmidiNote, u8)>,
    pub stop_notes: Vec<UnmidiNote>,
    pub redraw: bool,
    pub change_key: Option<Transpose>,
}

pub struct AppState {
    pub active_chord: Option<Chord>, //TODO privatize
    pub active_notes: HashSet<UnmidiNote>,

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

            chord_keys_down: HashSet::new(),
            mod_keys_down: HashSet::new(),
            action_keys_down: HashSet::new(),

            modifier_stage: Modifiers::empty(),
            action_stage: Actions::empty(),

            transpose: Transpose(0),
        }
    }

    pub fn handle_key_event(&mut self, event: KeyEvent) -> AppEffects {
        let mut effects = AppEffects {
            redraw: true,
            change_key: None,
            stop_notes: Vec::new(),
        };
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

            if self.action_stage.contains(Actions::ChangeKey) {
                self.transpose = Transpose(chord.get_root().as_i16()).center_octave();
                effects.change_key = Some(self.transpose);
            }
        }

        self.modifier_stage = Modifiers::empty();
        self.action_stage = Actions::empty();

        if venerated_old_chord != new_chord {
            effects.redraw = true;
            self.active_chord = new_chord;

            if let Some(chord) = new_chord {
                effects.stop_notes = (0..128)
                    .map(|i| UnmidiNote(i))
                    .filter(|un| !chord.contains(*un - self.transpose))
                    .filter(|un| self.active_notes.contains(un))
                    .collect();
            }
        }

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

// Decide chord from current chord_keys_down and previous chord state.
fn decide_chord_base(
    venerated_old_chord: Option<&Chord>,
    chord_keys_down: &HashSet<ChordButton>,
) -> Option<Chord> {
    if chord_keys_down.contains(&ChordButton::HeptatonicMajor) {
        return Some(Chord::new(
            ROOT_I,
            Modifiers::MajorTri
                | Modifiers::AddMajor2
                | Modifiers::Add4
                | Modifiers::AddMajor6
                | Modifiers::AddMajor7,
        ));
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

    fn press_chord(state: &mut AppState, button: ChordButton) {
        state.handle_key_event(KeyEvent::Chord {
            state: KeyState::Pressed,
            button,
        });
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
    fn change_key_sets_transpose() {
        let mut state = AppState::new();

        let effects = state.handle_key_event(KeyEvent::Action {
            state: KeyState::Pressed,
            button: ActionButton::ChangeKey,
            action: Actions::ChangeKey,
        });

        // No chord yet → no key change
        assert!(effects.change_key.is_none());

        press_chord(&mut state, ChordButton::V);
        let effects = state.handle_key_event(KeyEvent::Action {
            state: KeyState::Pressed,
            button: ActionButton::ChangeKey,
            action: Actions::ChangeKey,
        });

        assert_eq!(effects.change_key, Some(state.transpose));
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
}

