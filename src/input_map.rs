use crate::app_state::{ActionButton, Actions, ChordButton, KeyEvent, KeyState, ModButton};
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
///
/// IMPORTANT: the numeric values here are the single source of truth for Android's button IDs
/// and bit positions in `rustGetUiButtonsMask()`.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UiButton {
    // Degree chords
    V = 0,
    I = 1,
    IV = 2,
    VIIB = 3,
    II = 4,
    VI = 5,
    III = 6,
    VIIDim = 7,

    // Modifiers
    Maj7 = 8,
    No3 = 9,
    Sus4 = 10,
    MinorMajor = 11,
    Add2 = 12,
    Add7 = 13,

    // Special chord mode
    Hept = 14,
}

impl UiButton {
    pub const COUNT: usize = 15;

    pub const ORDER: [UiButton; UiButton::COUNT] = [
        UiButton::V,
        UiButton::I,
        UiButton::IV,
        UiButton::VIIB,
        UiButton::II,
        UiButton::VI,
        UiButton::III,
        UiButton::VIIDim,
        UiButton::Maj7,
        UiButton::No3,
        UiButton::Sus4,
        UiButton::MinorMajor,
        UiButton::Add2,
        UiButton::Add7,
        UiButton::Hept,
    ];

    pub fn index(self) -> usize {
        self as u8 as usize
    }

    pub fn from_index(idx: usize) -> Option<Self> {
        match idx as u8 {
            0 => Some(UiButton::V),
            1 => Some(UiButton::I),
            2 => Some(UiButton::IV),
            3 => Some(UiButton::VIIB),
            4 => Some(UiButton::II),
            5 => Some(UiButton::VI),
            6 => Some(UiButton::III),
            7 => Some(UiButton::VIIDim),
            8 => Some(UiButton::Maj7),
            9 => Some(UiButton::No3),
            10 => Some(UiButton::Sus4),
            11 => Some(UiButton::MinorMajor),
            12 => Some(UiButton::Add2),
            13 => Some(UiButton::Add7),
            14 => Some(UiButton::Hept),
            _ => None,
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            UiButton::V => "v",
            UiButton::I => "i",
            UiButton::IV => "iv",
            UiButton::VIIB => "viib",
            UiButton::II => "ii",
            UiButton::VI => "vi",
            UiButton::III => "iii",
            UiButton::VIIDim => "vii_dim",
            UiButton::Maj7 => "maj7",
            UiButton::No3 => "no3",
            UiButton::Sus4 => "sus4",
            UiButton::MinorMajor => "minor_major",
            UiButton::Add2 => "add2",
            UiButton::Add7 => "add7",
            UiButton::Hept => "hept",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id.trim().to_ascii_lowercase().as_str() {
            "v" => Some(UiButton::V),
            "i" => Some(UiButton::I),
            "iv" => Some(UiButton::IV),
            "viib" => Some(UiButton::VIIB),
            "ii" => Some(UiButton::II),
            "vi" => Some(UiButton::VI),
            "iii" => Some(UiButton::III),
            "vii_dim" => Some(UiButton::VIIDim),
            "maj7" => Some(UiButton::Maj7),
            "no3" => Some(UiButton::No3),
            "sus4" => Some(UiButton::Sus4),
            "minor_major" => Some(UiButton::MinorMajor),
            "add2" => Some(UiButton::Add2),
            "add7" => Some(UiButton::Add7),
            "hept" => Some(UiButton::Hept),
            _ => None,
        }
    }
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
