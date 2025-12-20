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
}
