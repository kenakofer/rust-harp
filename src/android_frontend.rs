use crate::app_state::NoteOn;
use crate::engine::Engine;

/// Android-facing wrapper that owns the core Engine.
///
/// Kept separate so JNI functions can be thin and avoid leaking core types into Java.
pub struct AndroidFrontend {
    engine: Engine,
    pending_play_notes: Vec<NoteOn>,
}

impl AndroidFrontend {
    pub fn new() -> Self {
        Self {
            engine: Engine::new(),
            pending_play_notes: Vec::new(),
        }
    }

    pub fn engine_mut(&mut self) -> &mut Engine {
        &mut self.engine
    }

    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    pub fn push_effects(&mut self, effects: crate::app_state::AppEffects) {
        self.pending_play_notes.extend(effects.play_notes);
    }

    pub fn drain_play_notes(&mut self) -> impl Iterator<Item = NoteOn> + '_ {
        self.pending_play_notes.drain(..)
    }

    pub fn has_pending_play_notes(&self) -> bool {
        !self.pending_play_notes.is_empty()
    }
}
