use crate::app_state::{
    ActionButton, Actions, ChordButton, KeyEvent, KeyState, ModButton,
};
use crate::chord::Modifiers;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UiKey {
    Char(char),
    Control,
    Tab,
}

/// Virtual UI buttons for touchscreen frontends.
///
/// These intentionally map onto the same `KeyEvent` logic as keyboard input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UiButton {
    // Degree chords
    VIIB,
    IV,
    I,
    V,
    II,
    VI,
    III,
    VIIDim,

    // Modifiers
    Maj7,
    No3,
    Sus4,
    MinorMajor,
    Add2,
    Add7,

    // Special chord mode
    Hept,
}

pub fn key_event_from_ui(state: KeyState, key: UiKey) -> Option<KeyEvent> {
    use UiKey::*;

    match key {
        // Chords
        Char('a') => Some(KeyEvent::Chord {
            state,
            button: ChordButton::VIIB,
        }),
        Char('s') => Some(KeyEvent::Chord {
            state,
            button: ChordButton::IV,
        }),
        Char('d') => Some(KeyEvent::Chord {
            state,
            button: ChordButton::I,
        }),
        Char('f') => Some(KeyEvent::Chord {
            state,
            button: ChordButton::V,
        }),
        Char('z') => Some(KeyEvent::Chord {
            state,
            button: ChordButton::II,
        }),
        Char('x') => Some(KeyEvent::Chord {
            state,
            button: ChordButton::VI,
        }),
        Char('c') => Some(KeyEvent::Chord {
            state,
            button: ChordButton::III,
        }),
        Char('v') => Some(KeyEvent::Chord {
            state,
            button: ChordButton::VII,
        }),
        Control => Some(KeyEvent::Chord {
            state,
            button: ChordButton::HeptatonicMajor,
        }),

        // Modifiers
        Char('5') => Some(KeyEvent::Modifier {
            state,
            button: ModButton::Major2,
            modifiers: Modifiers::AddMajor2,
        }),
        Char('b') => Some(KeyEvent::Modifier {
            state,
            button: ModButton::Major7,
            modifiers: Modifiers::AddMajor7,
        }),
        Char('6') => Some(KeyEvent::Modifier {
            state,
            button: ModButton::Minor7,
            modifiers: Modifiers::AddMinor7,
        }),
        Char('3') => Some(KeyEvent::Modifier {
            state,
            button: ModButton::Sus4,
            modifiers: Modifiers::Sus4,
        }),
        Char('4') => Some(KeyEvent::Modifier {
            state,
            button: ModButton::MinorMajor,
            modifiers: Modifiers::SwitchMinorMajor,
        }),
        Char('.') => Some(KeyEvent::Modifier {
            state,
            button: ModButton::No3,
            modifiers: Modifiers::No3,
        }),

        // Actions
        Char('1') => Some(KeyEvent::Action {
            state,
            button: ActionButton::ChangeKey,
            action: Actions::ChangeKey,
        }),
        Tab => Some(KeyEvent::Action {
            state,
            button: ActionButton::Pulse,
            action: Actions::Pulse,
        }),

        _ => None,
    }
}

/// Convert a touchscreen UI button press/release into one or more `KeyEvent`s.
///
pub fn key_events_from_button(state: KeyState, button: UiButton) -> Vec<KeyEvent> {
    match button {
        // Chords
        UiButton::VIIB => vec![KeyEvent::Chord {
            state,
            button: ChordButton::VIIB,
        }],
        UiButton::IV => vec![KeyEvent::Chord {
            state,
            button: ChordButton::IV,
        }],
        UiButton::I => vec![KeyEvent::Chord {
            state,
            button: ChordButton::I,
        }],
        UiButton::V => vec![KeyEvent::Chord {
            state,
            button: ChordButton::V,
        }],
        UiButton::II => vec![KeyEvent::Chord {
            state,
            button: ChordButton::II,
        }],
        UiButton::VI => vec![KeyEvent::Chord {
            state,
            button: ChordButton::VI,
        }],
        UiButton::III => vec![KeyEvent::Chord {
            state,
            button: ChordButton::III,
        }],
        UiButton::VIIDim => vec![KeyEvent::Chord {
            state,
            button: ChordButton::VII,
        }],
        UiButton::Hept => vec![KeyEvent::Chord {
            state,
            button: ChordButton::HeptatonicMajor,
        }],

        // Modifiers
        UiButton::Add2 => vec![KeyEvent::Modifier {
            state,
            button: ModButton::Major2,
            modifiers: Modifiers::AddMajor2,
        }],
        UiButton::Maj7 => vec![KeyEvent::Modifier {
            state,
            button: ModButton::Major7,
            modifiers: Modifiers::AddMajor7,
        }],
        UiButton::Add7 => vec![KeyEvent::Modifier {
            state,
            button: ModButton::Minor7,
            modifiers: Modifiers::AddMinor7,
        }],
        UiButton::Sus4 => vec![KeyEvent::Modifier {
            state,
            button: ModButton::Sus4,
            modifiers: Modifiers::Sus4,
        }],
        UiButton::MinorMajor => vec![KeyEvent::Modifier {
            state,
            button: ModButton::MinorMajor,
            modifiers: Modifiers::SwitchMinorMajor,
        }],
        UiButton::No3 => vec![KeyEvent::Modifier {
            state,
            button: ModButton::No3,
            modifiers: Modifiers::No3,
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_key_map_examples() {
        assert_eq!(
            key_event_from_ui(KeyState::Pressed, UiKey::Char('d')),
            Some(KeyEvent::Chord {
                state: KeyState::Pressed,
                button: ChordButton::I,
            })
        );
        assert_eq!(
            key_event_from_ui(KeyState::Pressed, UiKey::Control),
            Some(KeyEvent::Chord {
                state: KeyState::Pressed,
                button: ChordButton::HeptatonicMajor,
            })
        );
        assert_eq!(
            key_event_from_ui(KeyState::Pressed, UiKey::Char('6')),
            Some(KeyEvent::Modifier {
                state: KeyState::Pressed,
                button: ModButton::Minor7,
                modifiers: Modifiers::AddMinor7,
            })
        );
        assert_eq!(
            key_event_from_ui(KeyState::Pressed, UiKey::Tab),
            Some(KeyEvent::Action {
                state: KeyState::Pressed,
                button: ActionButton::Pulse,
                action: Actions::Pulse,
            })
        );
    }

    #[test]
    fn ui_button_vi_maps_to_vi_chord() {
        let pressed = key_events_from_button(KeyState::Pressed, UiButton::VI);
        assert_eq!(
            pressed,
            vec![KeyEvent::Chord {
                state: KeyState::Pressed,
                button: ChordButton::VI,
            }]
        );
    }
}
