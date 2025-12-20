use crate::app_state::{AppEffects, KeyState};
use crate::chord::Chord;
use crate::input_map::{self, UiKey};
use crate::notes::{UnkeyedNote, UnmidiNote};

fn ui_key_from_winit(key: &winit::keyboard::Key) -> Option<UiKey> {
    use winit::keyboard::Key::Character;
    use winit::keyboard::Key::Named;

    match key {
        Character(s) => s.chars().next().map(UiKey::Char),
        Named(winit::keyboard::NamedKey::Control) => Some(UiKey::Control),
        Named(winit::keyboard::NamedKey::Tab) => Some(UiKey::Tab),
        _ => None,
    }
}

fn key_event_from_winit(event: &winit::event::KeyEvent) -> Option<crate::app_state::KeyEvent> {
    let state = match event.state {
        winit::event::ElementState::Pressed => KeyState::Pressed,
        winit::event::ElementState::Released => KeyState::Released,
    };

    let key = ui_key_from_winit(&event.logical_key)?;
    input_map::key_event_from_ui(state, key)
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
