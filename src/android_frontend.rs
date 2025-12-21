use crate::android_audio::SquareSynth;
use crate::app_state::{AppEffects, NoteOn};
use crate::engine::Engine;
use crate::layout;
use crate::notes::{MidiNote, NoteVolume, Transpose};
use crate::touch::{TouchEvent, TouchTracker};

use std::sync::Mutex;

struct AudioState {
    pending_play_notes: Vec<NoteOn>,
    synth: SquareSynth,
}

/// Android-facing wrapper that owns the core Engine + audio synth.
///
/// Kept separate so JNI functions can be thin and avoid leaking core types into Java.
pub struct AndroidFrontend {
    engine: Engine,
    audio: Mutex<AudioState>,
    touch: TouchTracker,
}

impl AndroidFrontend {
    pub fn new() -> Self {
        Self {
            engine: Engine::new(),
            audio: Mutex::new(AudioState {
                pending_play_notes: Vec::new(),
                synth: SquareSynth::new(48_000),
            }),
            touch: TouchTracker::new(),
        }
    }

    pub fn engine_mut(&mut self) -> &mut Engine {
        &mut self.engine
    }

    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    pub fn push_effects(&self, effects: crate::app_state::AppEffects) {
        if effects.play_notes.is_empty() {
            return;
        }
        let mut a = self.audio.lock().unwrap();
        a.pending_play_notes.extend(effects.play_notes);
    }

    pub fn drain_play_notes(&self) -> Vec<NoteOn> {
        let mut a = self.audio.lock().unwrap();
        std::mem::take(&mut a.pending_play_notes)
    }

    pub fn has_pending_play_notes(&self) -> bool {
        let a = self.audio.lock().unwrap();
        !a.pending_play_notes.is_empty()
    }

    pub fn set_sample_rate(&self, sample_rate_hz: u32) {
        let mut a = self.audio.lock().unwrap();
        a.synth = SquareSynth::new(sample_rate_hz.max(1));
    }

    fn drain_into_synth(a: &mut AudioState) {
        // Match desktop's MIDI_BASE_TRANSPOSE (C2)
        const MIDI_BASE_TRANSPOSE: Transpose = Transpose(36);

        let drained: Vec<_> = a.pending_play_notes.drain(..).collect();
        for pn in drained {
            let MidiNote(m) = MIDI_BASE_TRANSPOSE + pn.note;
            let NoteVolume(v) = pn.volume;
            a.synth.note_on(MidiNote(m), v);
        }
    }

    pub fn render_audio_i16_interleaved(&self, out: &mut [i16], channels: usize) {
        let mut a = self.audio.lock().unwrap();
        Self::drain_into_synth(&mut a);
        a.synth.render_i16_interleaved(out, channels);
    }

    pub fn render_audio_f32_interleaved(&self, out: &mut [f32], channels: usize) {
        let mut a = self.audio.lock().unwrap();
        Self::drain_into_synth(&mut a);
        a.synth.render_f32_interleaved(out, channels);
    }

    pub fn render_audio_i16_mono(&self, out: &mut [i16]) {
        self.render_audio_i16_interleaved(out, 1);
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

        let drained: Vec<_> = f.drain_play_notes();
        assert_eq!(drained.len(), 1);
        assert!(!f.has_pending_play_notes());
    }
}
