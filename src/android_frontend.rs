use crate::android_audio::SquareSynth;
use crate::app_state::{AppEffects, NoteOn};
use crate::engine::Engine;
use crate::layout;
use crate::touch::{TouchEvent, TouchTracker};

/// Android-facing wrapper that owns the core Engine + audio synth.
///
/// Kept separate so JNI functions can be thin and avoid leaking core types into Java.
pub struct AndroidFrontend {
    engine: Engine,
    pending_play_notes: Vec<NoteOn>,
    pub synth: SquareSynth,
    touch: TouchTracker,
}

impl AndroidFrontend {
    pub fn new() -> Self {
        Self {
            engine: Engine::new(),
            pending_play_notes: Vec::new(),
            synth: SquareSynth::new(48_000),
            touch: TouchTracker::new(),
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

    pub fn set_sample_rate(&mut self, sample_rate_hz: u32) {
        self.synth = SquareSynth::new(sample_rate_hz);
    }

    pub fn handle_touch(&mut self, event: TouchEvent, width_px: f32) -> (AppEffects, bool) {
        let positions = layout::compute_note_positions(width_px);
        let mut effects = AppEffects {
            play_notes: Vec::new(),
            stop_notes: Vec::new(),
            redraw: false,
            change_key: None,
        };

        let crossings = self.touch.handle_event(event, &positions);
        let haptic = !crossings.is_empty();

        for crossing in crossings {
            for note in crossing.notes {
                let e = self.engine.handle_strum_crossing(note);
                effects.play_notes.extend(e.play_notes);
                effects.stop_notes.extend(e.stop_notes);
                effects.redraw |= e.redraw;
                if effects.change_key.is_none() {
                    effects.change_key = e.change_key;
                }
            }
        }

        (effects, haptic)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notes::UnkeyedNote;

    #[test]
    fn android_frontend_queues_play_notes_from_engine_effects() {
        let mut f = AndroidFrontend::new();
        let effects = f.engine_mut().handle_strum_crossing(UnkeyedNote(0));
        assert_eq!(effects.play_notes.len(), 1);

        f.push_effects(effects);
        assert!(f.has_pending_play_notes());

        let drained: Vec<_> = f.drain_play_notes().collect();
        assert_eq!(drained.len(), 1);
        assert!(!f.has_pending_play_notes());
    }
}
