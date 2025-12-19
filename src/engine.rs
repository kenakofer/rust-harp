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

    pub fn active_notes(&self) -> impl Iterator<Item = UnmidiNote> + '_ {
        self.state.active_notes.iter().cloned()
    }
}
