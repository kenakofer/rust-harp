use crate::app_state::{
    ActionButton, Actions, AppEffects, ChordButton, KeyEvent, KeyState, ModButton,
};
use crate::chord::{Chord, Modifiers};
use crate::notes::{UnkeyedNote, UnmidiNote};

struct ChordButtonTableEntry {
    button: ChordButton,
    key_check: fn(&winit::keyboard::Key) -> bool,
}

const CHORD_BUTTON_TABLE: [ChordButtonTableEntry; 9] = [
    ChordButtonTableEntry {
        button: ChordButton::VIIB,
        key_check: |k| matches!(k, winit::keyboard::Key::Character(s) if s == "a"),
    },
    ChordButtonTableEntry {
        button: ChordButton::IV,
        key_check: |k| matches!(k, winit::keyboard::Key::Character(s) if s == "s"),
    },
    ChordButtonTableEntry {
        button: ChordButton::I,
        key_check: |k| matches!(k, winit::keyboard::Key::Character(s) if s == "d"),
    },
    ChordButtonTableEntry {
        button: ChordButton::V,
        key_check: |k| matches!(k, winit::keyboard::Key::Character(s) if s == "f"),
    },
    ChordButtonTableEntry {
        button: ChordButton::II,
        key_check: |k| matches!(k, winit::keyboard::Key::Character(s) if s == "z"),
    },
    ChordButtonTableEntry {
        button: ChordButton::VI,
        key_check: |k| matches!(k, winit::keyboard::Key::Character(s) if s == "x"),
    },
    ChordButtonTableEntry {
        button: ChordButton::III,
        key_check: |k| matches!(k, winit::keyboard::Key::Character(s) if s == "c"),
    },
    ChordButtonTableEntry {
        button: ChordButton::VII,
        key_check: |k| matches!(k, winit::keyboard::Key::Character(s) if s == "v"),
    },
    ChordButtonTableEntry {
        button: ChordButton::HeptatonicMajor,
        key_check: |k| {
            matches!(
                k,
                winit::keyboard::Key::Named(winit::keyboard::NamedKey::Control)
            )
        },
    },
];

struct ModButtonTableEntry {
    button: ModButton,
    key_check: fn(&winit::keyboard::Key) -> bool,
    modifiers: Modifiers,
}

const MOD_BUTTON_TABLE: [ModButtonTableEntry; 6] = [
    ModButtonTableEntry {
        button: ModButton::Major2,
        key_check: |k| matches!(k, winit::keyboard::Key::Character(s) if s == "5"),
        modifiers: Modifiers::AddMajor2,
    },
    ModButtonTableEntry {
        button: ModButton::Major7,
        key_check: |k| matches!(k, winit::keyboard::Key::Character(s) if s == "b"),
        modifiers: Modifiers::AddMajor7,
    },
    ModButtonTableEntry {
        button: ModButton::Minor7,
        key_check: |k| matches!(k, winit::keyboard::Key::Character(s) if s == "6"),
        modifiers: Modifiers::AddMinor7,
    },
    ModButtonTableEntry {
        button: ModButton::Sus4,
        key_check: |k| matches!(k, winit::keyboard::Key::Character(s) if s == "3"),
        modifiers: Modifiers::Sus4,
    },
    ModButtonTableEntry {
        button: ModButton::MinorMajor,
        key_check: |k| matches!(k, winit::keyboard::Key::Character(s) if s == "4"),
        modifiers: Modifiers::SwitchMinorMajor,
    },
    ModButtonTableEntry {
        button: ModButton::No3,
        key_check: |k| matches!(k, winit::keyboard::Key::Character(s) if s == "."),
        modifiers: Modifiers::No3,
    },
];

fn chord_button_for(key: &winit::keyboard::Key) -> Option<ChordButton> {
    CHORD_BUTTON_TABLE
        .iter()
        .find(|e| (e.key_check)(key))
        .map(|e| e.button)
}

fn mod_button_for(key: &winit::keyboard::Key) -> Option<(ModButton, Modifiers)> {
    MOD_BUTTON_TABLE
        .iter()
        .find(|e| (e.key_check)(key))
        .map(|e| (e.button, e.modifiers))
}

fn action_button_for(key: &winit::keyboard::Key) -> Option<(ActionButton, Actions)> {
    use winit::keyboard::Key::Character;
    use winit::keyboard::Key::Named;
    use winit::keyboard::NamedKey::Tab;

    match key {
        Character(s) if s == "1" => Some((ActionButton::ChangeKey, Actions::ChangeKey)),
        Named(Tab) => Some((ActionButton::Pulse, Actions::Pulse)),
        _ => None,
    }
}

fn key_event_from_winit(event: &winit::event::KeyEvent) -> Option<KeyEvent> {
    let state = match event.state {
        winit::event::ElementState::Pressed => KeyState::Pressed,
        winit::event::ElementState::Released => KeyState::Released,
    };

    let key = &event.logical_key;

    if let Some(button) = chord_button_for(key) {
        return Some(KeyEvent::Chord { state, button });
    }

    if let Some((button, modifiers)) = mod_button_for(key) {
        return Some(KeyEvent::Modifier {
            state,
            button,
            modifiers,
        });
    }

    if let Some((button, action)) = action_button_for(key) {
        return Some(KeyEvent::Action {
            state,
            button,
            action,
        });
    }

    None
}

pub struct AppAdapter {
    engine: crate::engine::Engine,
}

impl AppAdapter {
    pub fn new() -> Self {
        Self {
            engine: crate::engine::Engine::new(),
        }
    }

    pub fn handle_winit_key_event(
        &mut self,
        event: &winit::event::KeyEvent,
    ) -> Option<AppEffects> {
        let app_event = key_event_from_winit(event)?;
        Some(self.engine.handle_event(app_event))
    }

    pub fn handle_strum_crossing(&mut self, note: UnkeyedNote) -> AppEffects {
        self.engine.handle_strum_crossing(note)
    }

    pub fn active_chord(&self) -> &Option<Chord> {
        self.engine.active_chord()
    }

    pub fn active_notes(&self) -> impl Iterator<Item = UnmidiNote> + '_ {
        self.engine.active_notes()
    }
}
