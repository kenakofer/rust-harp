use crate::notes::{MidiNote, NoteVolume, Transpose, UnmidiNote};
use crate::output_midir::MidiBackend;
use crate::ui_adapter::AppAdapter;
use crate::ui_settings::UiAudioBackend;

#[cfg(feature = "synth")]
use crate::output_synth::SynthBackend;

#[cfg(feature = "desktop")]
use winit::window::Window;

use crate::strum;

pub(crate) const MIDI_BASE_TRANSPOSE: Transpose = Transpose(48); // Add with UnmidiNote to get MidiNote. MIDI Note 48 is C3

pub(crate) const MICRO_CHANNEL: u8 = 3; // MIDI channel 2 (0-based)
pub(crate) const MICRO_PROGRAM: u8 = 115; // instrument program for micro-steps, 115 = Wood block
pub(crate) const MICRO_NOTE: MidiNote = MidiNote(20); // middle C for micro-step trigger
pub(crate) const MICRO_VELOCITY: u8 = 50; // quiet click

pub(crate) trait BackendAudio {
    fn stop_note(&mut self, backend: UiAudioBackend, midi_note: MidiNote);
    fn play_note(&mut self, backend: UiAudioBackend, midi_note: MidiNote, volume: NoteVolume);
}

pub(crate) struct DesktopAudio {
    midi: MidiBackend,
    #[cfg(feature = "synth")]
    synth: Option<SynthBackend>,
}

impl DesktopAudio {
    pub(crate) fn new(midi: MidiBackend) -> Self {
        #[cfg(feature = "synth")]
        let synth = SynthBackend::new().ok();

        Self {
            midi,
            #[cfg(feature = "synth")]
            synth,
        }
    }

    pub(crate) fn set_a4_tuning_hz(&mut self, a4_tuning_hz: u16) {
        #[cfg(feature = "synth")]
        if let Some(s) = &self.synth {
            s.set_a4_tuning_hz(a4_tuning_hz);
        }
    }

    pub(crate) fn has_synth(&self) -> bool {
        #[cfg(feature = "synth")]
        {
            return self.synth.is_some();
        }
        #[cfg(not(feature = "synth"))]
        {
            return false;
        }
    }

    pub(crate) fn stop_note(&mut self, backend: UiAudioBackend, midi_note: MidiNote) {
        match backend {
            UiAudioBackend::Midi => self.midi.stop_note(midi_note),
            UiAudioBackend::Synth => {
                #[cfg(feature = "synth")]
                if let Some(s) = &self.synth {
                    s.stop_note(midi_note);
                    return;
                }
                self.midi.stop_note(midi_note);
            }
            _ => self.midi.stop_note(midi_note),
        }
    }

    pub(crate) fn play_note(&mut self, backend: UiAudioBackend, midi_note: MidiNote, volume: NoteVolume) {
        match backend {
            UiAudioBackend::Midi => self.midi.play_note(midi_note, volume),
            UiAudioBackend::Synth => {
                #[cfg(feature = "synth")]
                if let Some(s) = &self.synth {
                    s.play_note(midi_note, volume);
                    return;
                }
                self.midi.play_note(midi_note, volume);
            }
            _ => self.midi.play_note(midi_note, volume),
        }
    }

    pub(crate) fn stop_note_all_backends(&mut self, midi_note: MidiNote) {
        self.midi.stop_note(midi_note);
        #[cfg(feature = "synth")]
        if let Some(s) = &self.synth {
            s.stop_note(midi_note);
        }
    }

    pub(crate) fn stop_all_notes_all_backends(&mut self, notes: impl Iterator<Item = UnmidiNote>) {
        for n in notes {
            self.stop_note_all_backends(MIDI_BASE_TRANSPOSE + n);
        }
    }

    pub(crate) fn midi_available(&self) -> bool {
        self.midi.is_available()
    }

    pub(crate) fn midi_mut(&mut self) -> &mut MidiBackend {
        &mut self.midi
    }
}

impl BackendAudio for DesktopAudio {
    fn stop_note(&mut self, backend: UiAudioBackend, midi_note: MidiNote) {
        DesktopAudio::stop_note(self, backend, midi_note)
    }

    fn play_note(&mut self, backend: UiAudioBackend, midi_note: MidiNote, volume: NoteVolume) {
        DesktopAudio::play_note(self, backend, midi_note, volume)
    }
}

pub(crate) fn process_app_effects(
    effects: crate::app_state::AppEffects,
    audio: &mut impl BackendAudio,
    audio_backend: UiAudioBackend,
    window: Option<&Window>,
) -> bool {
    if effects.redraw {
        if let Some(w) = window {
            w.request_redraw();
        }
    }
    if let Some(transpose) = effects.change_key {
        log::info!("Changed key: {:?}", transpose);
    }

    effects.apply_stop_then_play(
        audio,
        |a, un| a.stop_note(audio_backend, MIDI_BASE_TRANSPOSE + un),
        |a, pn| a.play_note(audio_backend, MIDI_BASE_TRANSPOSE + pn.note, pn.volume),
    )
}

#[allow(dead_code)]
pub(crate) fn check_pluck(
    x1: f32,
    x2: f32,
    audio: &mut DesktopAudio,
    audio_backend: UiAudioBackend,
    app: &mut AppAdapter,
    note_positions: &[f32],
) {
    if audio_backend == UiAudioBackend::Midi && !audio.midi_available() {
        return;
    }

    for crossing in strum::detect_crossings(x1, x2, note_positions) {
        let mut played_any = false;

        for note in crossing.notes {
            let effects = app.handle_strum_crossing(note);
            if process_app_effects(effects, audio, audio_backend, None) {
                played_any = true;
            }
        }

        if !played_any && audio_backend == UiAudioBackend::Midi {
            // Damped string sound (MIDI only)
            if let Some(conn) = audio.midi_mut().conn_mut() {
                let on = 0x90 | (MICRO_CHANNEL & 0x0F);
                let off = 0x80 | (MICRO_CHANNEL & 0x0F);
                let _ = conn.send(&[on, MICRO_NOTE.0, MICRO_VELOCITY]);
                let _ = conn.send(&[off, MICRO_NOTE.0, 0]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_state::{AppEffects, NoteOn};
    use crate::notes::{NoteVolume, UnmidiNote};

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Call {
        Stop(MidiNote),
        Play(MidiNote, NoteVolume),
    }

    #[derive(Default)]
    struct MockAudio {
        calls: Vec<Call>,
    }

    impl BackendAudio for MockAudio {
        fn stop_note(&mut self, _backend: UiAudioBackend, midi_note: MidiNote) {
            self.calls.push(Call::Stop(midi_note));
        }

        fn play_note(&mut self, _backend: UiAudioBackend, midi_note: MidiNote, volume: NoteVolume) {
            self.calls.push(Call::Play(midi_note, volume));
        }
    }

    #[test]
    fn process_app_effects_applies_stop_before_play_and_returns_played() {
        let effects = AppEffects {
            stop_notes: vec![UnmidiNote(1), UnmidiNote(2)],
            play_notes: vec![NoteOn {
                note: UnmidiNote(2),
                volume: NoteVolume(70),
            }],
            redraw: false,
            change_key: None,
        };

        let mut audio = MockAudio::default();
        let played = process_app_effects(effects, &mut audio, UiAudioBackend::Midi, None);
        assert!(played);

        assert_eq!(
            audio.calls,
            vec![
                Call::Stop(MIDI_BASE_TRANSPOSE + UnmidiNote(1)),
                Call::Stop(MIDI_BASE_TRANSPOSE + UnmidiNote(2)),
                Call::Play(MIDI_BASE_TRANSPOSE + UnmidiNote(2), NoteVolume(70)),
            ]
        );
    }

    #[test]
    fn process_app_effects_returns_false_when_no_play_notes() {
        let effects = AppEffects {
            stop_notes: vec![UnmidiNote(1)],
            play_notes: vec![],
            redraw: false,
            change_key: None,
        };

        let mut audio = MockAudio::default();
        let played = process_app_effects(effects, &mut audio, UiAudioBackend::Midi, None);
        assert!(!played);
        assert_eq!(audio.calls, vec![Call::Stop(MIDI_BASE_TRANSPOSE + UnmidiNote(1))]);
    }
}
