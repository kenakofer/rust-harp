use crate::app_state::{AppEffects, AppState, KeyEvent};
use crate::chord::Chord;
use crate::notes::{UnkeyedNote, UnmidiNote};

/// Platform-agnostic wrapper around `AppState`.
/// UI frontends translate their input into `KeyEvent` and feed it here.
pub struct Engine {
    state: AppState,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            state: AppState::new(),
        }
    }

    pub fn transpose(&self) -> crate::notes::Transpose {
        self.state.transpose
    }

    pub fn set_transpose(&mut self, transpose: crate::notes::Transpose) -> AppEffects {
        self.state.set_transpose(transpose)
    }

    pub fn handle_event(&mut self, event: KeyEvent) -> AppEffects {
        self.state.handle_key_event(event)
    }

    pub fn handle_strum_crossing(&mut self, note: UnkeyedNote) -> AppEffects {
        self.state
            .handle_key_event(KeyEvent::StrumCrossing { note })
    }

    pub fn active_chord(&self) -> &Option<Chord> {
        &self.state.active_chord
    }

    pub fn chord_button_down(&self, button: crate::app_state::ChordButton) -> bool {
        self.state.chord_button_down(button)
    }

    pub fn mod_button_down(&self, button: crate::app_state::ModButton) -> bool {
        self.state.mod_button_down(button)
    }

    pub fn active_notes(&self) -> impl Iterator<Item = UnmidiNote> + '_ {
        self.state.active_notes.iter().cloned()
    }
}
