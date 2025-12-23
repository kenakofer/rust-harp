use crate::app_state::{AppEffects, KeyState};
use crate::chord::Chord;
use crate::input_map::UiKey;
use crate::notes::{UnkeyedNote, UnmidiNote};
use crate::ui_events::{UiEvent, UiSession};

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

pub fn ui_event_from_winit(event: &winit::event::KeyEvent) -> Option<UiEvent> {
    let state = match event.state {
        winit::event::ElementState::Pressed => KeyState::Pressed,
        winit::event::ElementState::Released => KeyState::Released,
    };

    let key = ui_key_from_winit(&event.logical_key)?;
    Some(UiEvent::Key { state, key })
}

pub struct AppAdapter {
    ui: UiSession,
}

impl AppAdapter {
    pub fn new() -> Self {
        Self { ui: UiSession::new() }
    }

    pub fn handle_winit_key_event(
        &mut self,
        event: &winit::event::KeyEvent,
    ) -> Option<AppEffects> {
        let ui_event = ui_event_from_winit(event)?;
        Some(self.ui.handle(ui_event, &[]).effects)
    }

    pub fn handle_strum_crossing(&mut self, note: UnkeyedNote) -> AppEffects {
        self.ui.engine_mut().handle_strum_crossing(note)
    }

    pub fn active_chord(&self) -> &Option<Chord> {
        self.ui.engine().active_chord()
    }

    pub fn active_notes(&self) -> impl Iterator<Item = UnmidiNote> + '_ {
        self.ui.engine().active_notes()
    }
}
