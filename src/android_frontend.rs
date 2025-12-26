use crate::synth::SquareSynth;
use crate::app_state::{AppEffects, NoteOn};
use crate::layout;
use crate::touch::TouchEvent;
use crate::ui_events::{UiEvent, UiSession};

use std::collections::HashSet;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub enum AudioMsg {
    NoteOn(NoteOn),
    NoteOff(crate::notes::UnmidiNote),
    SetSampleRate(u32),
    SetA4Tuning(u16),
}

pub const NOTE_STRIKE_VIS_MS: u64 = 220;
pub const NOTE_STRUM_VIS_MS: u64 = 160;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoteVisualKind {
    Strike,
    Strum,
}

#[derive(Clone, Copy, Debug)]
pub struct NoteVisualEvent {
    pub at: Instant,
    pub row: crate::rows::RowId,
    pub note: crate::notes::UnkeyedNote,
    pub kind: NoteVisualKind,
}

struct DeferredStopNotes {
    due: Option<Instant>,
    notes: Vec<crate::notes::UnmidiNote>,
}

/// Android-facing wrapper that owns the core Engine + touch tracker.
///
/// Audio is fed to the realtime audio thread via a channel so the AAudio callback can
/// avoid taking locks.
pub struct AndroidFrontend {
    ui: UiSession,

    audio_tx: Sender<AudioMsg>,
    audio_rx: Mutex<Option<Receiver<AudioMsg>>>,

    // Legacy fallback path (RustAudio/AudioTrack) renders from a Java thread, so a Mutex is OK.
    legacy_synth: Mutex<SquareSynth>,

    // While true, we suppress chord-change note-offs so chord selection doesn't "chatter".
    chord_wheel_hold_active: bool,
    chord_release_note_off_delay: Duration,
    deferred_stop_notes: Mutex<DeferredStopNotes>,

    note_visuals: Mutex<Vec<NoteVisualEvent>>,

    show_note_names: bool,
    a4_tuning_hz: u16,
}

impl AndroidFrontend {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            ui: UiSession::new(),
            audio_tx: tx,
            audio_rx: Mutex::new(Some(rx)),
            legacy_synth: Mutex::new(SquareSynth::new(48_000)),
            chord_wheel_hold_active: false,
            chord_release_note_off_delay: Duration::from_millis(350),
            deferred_stop_notes: Mutex::new(DeferredStopNotes {
                due: None,
                notes: Vec::new(),
            }),
            note_visuals: Mutex::new(Vec::new()),
            show_note_names: false,
            a4_tuning_hz: 440,
        }
    }

    pub fn engine_mut(&mut self) -> &mut crate::engine::Engine {
        self.ui.engine_mut()
    }

    pub fn set_show_note_names(&mut self, show: bool) {
        self.show_note_names = show;
    }

    pub fn set_play_on_tap(&mut self, enabled: bool) {
        self.ui.set_play_on_tap(enabled);
    }

    pub fn set_chord_hold_active(&mut self, active: bool) {
        self.chord_wheel_hold_active = active;
    }

    pub fn chord_hold_active(&self) -> bool {
        self.chord_wheel_hold_active
    }

    pub fn set_chord_release_note_off_delay_ms(&mut self, ms: u32) {
        self.chord_release_note_off_delay = Duration::from_millis(ms as u64);
    }

    pub fn defer_stop_notes(&self, notes: Vec<crate::notes::UnmidiNote>) {
        if notes.is_empty() {
            return;
        }
        let mut d = self.deferred_stop_notes.lock().unwrap();
        d.notes.extend(notes);
    }

    pub fn arm_deferred_stop_notes(&self) {
        let due = Instant::now() + self.chord_release_note_off_delay;
        let mut d = self.deferred_stop_notes.lock().unwrap();
        d.due = Some(match d.due {
            Some(old) => old.max(due),
            None => due,
        });
    }

    pub fn flush_deferred_stop_notes(&self) {
        let mut notes: Vec<crate::notes::UnmidiNote> = Vec::new();
        {
            let mut d = self.deferred_stop_notes.lock().unwrap();
            if let Some(due) = d.due {
                if Instant::now() < due {
                    return;
                }
            } else {
                return;
            }
            d.due = None;
            notes.append(&mut d.notes);
        }

        if notes.is_empty() {
            return;
        }

        // Avoid sending a late NoteOff for a note that has since been retriggered.
        let active: HashSet<crate::notes::UnmidiNote> = self.engine().active_notes().collect();

        let mut seen = HashSet::new();
        for un in notes {
            if active.contains(&un) {
                continue;
            }
            if seen.insert(un) {
                let _ = self.audio_tx.send(AudioMsg::NoteOff(un));
            }
        }
    }

    pub fn handle_ui_event(&mut self, event: UiEvent) -> AppEffects {
        self.ui.handle(event, &[]).effects
    }

    pub fn show_note_names(&self) -> bool {
        self.show_note_names
    }

    pub fn engine(&self) -> &crate::engine::Engine {
        self.ui.engine()
    }

    pub fn push_effects(&self, effects: AppEffects) {
        self.flush_deferred_stop_notes();

        // Stop before play so retriggering works correctly.
        for un in effects.stop_notes {
            let _ = self.audio_tx.send(AudioMsg::NoteOff(un));
        }
        for pn in effects.play_notes {
            let _ = self.audio_tx.send(AudioMsg::NoteOn(pn));
        }
    }

    pub fn set_sample_rate(&self, sample_rate_hz: u32) {
        let sr = sample_rate_hz.max(1);
        let _ = self.audio_tx.send(AudioMsg::SetSampleRate(sr));

        // Keep the legacy path in sync too.
        let mut s = self.legacy_synth.lock().unwrap();
        let a4 = s.a4_tuning_hz();
        *s = SquareSynth::with_tuning(sr, a4);
    }

    pub fn set_a4_tuning_hz(&mut self, a4_tuning_hz: u16) {
        self.a4_tuning_hz = a4_tuning_hz.clamp(430, 450);
        let _ = self.audio_tx.send(AudioMsg::SetA4Tuning(self.a4_tuning_hz));

        let mut s = self.legacy_synth.lock().unwrap();
        s.set_a4_tuning_hz(self.a4_tuning_hz);
    }

    pub fn take_audio_rx(&self) -> Option<Receiver<AudioMsg>> {
        self.audio_rx.lock().unwrap().take()
    }

    /// Recreate the audio message channel.
    ///
    /// This is used when switching between AAudio (callback owns the Receiver) and the legacy
    /// AudioTrack path.
    pub fn reset_audio_channel(&mut self) {
        let (tx, rx) = mpsc::channel();
        self.audio_tx = tx;
        *self.audio_rx.lock().unwrap() = Some(rx);
    }

    /// Legacy fallback (AudioTrack) fill: drain any queued messages then render mono i16.
    pub fn render_audio_i16_mono(&self, out: &mut [i16]) {
        self.flush_deferred_stop_notes();

        // Match desktop's MIDI_BASE_TRANSPOSE (C2)
        use crate::notes::{MidiNote, NoteVolume, Transpose};
        const MIDI_BASE_TRANSPOSE: Transpose = Transpose(36);

        if let Some(rx) = self.audio_rx.lock().unwrap().as_ref() {
            let mut s = self.legacy_synth.lock().unwrap();
            loop {
                match rx.try_recv() {
                    Ok(AudioMsg::NoteOn(pn)) => {
                        let MidiNote(m) = MIDI_BASE_TRANSPOSE + pn.note;
                        let NoteVolume(v) = pn.volume;
                        s.note_on(MidiNote(m), v);
                    }
                    Ok(AudioMsg::NoteOff(un)) => {
                        let MidiNote(m) = MIDI_BASE_TRANSPOSE + un;
                        s.note_off(MidiNote(m));
                    }
                    Ok(AudioMsg::SetSampleRate(sr)) => {
                        let a4 = s.a4_tuning_hz();
                        *s = SquareSynth::with_tuning(sr, a4);
                    }
                    Ok(AudioMsg::SetA4Tuning(a4)) => {
                        s.set_a4_tuning_hz(a4);
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => break,
                }
            }

            s.render_i16_mono(out);
            return;
        }

        out.fill(0);
    }

    pub fn has_active_note_visuals(&self) -> bool {
        let now = Instant::now();
        let mut v = self.note_visuals.lock().unwrap();
        v.retain(|e| {
            let age = now.saturating_duration_since(e.at);
            let max = match e.kind {
                NoteVisualKind::Strike => Duration::from_millis(NOTE_STRIKE_VIS_MS),
                NoteVisualKind::Strum => Duration::from_millis(NOTE_STRUM_VIS_MS),
            };
            age < max
        });
        !v.is_empty()
    }

    pub fn note_visuals_snapshot(&self) -> Vec<NoteVisualEvent> {
        let now = Instant::now();
        let mut v = self.note_visuals.lock().unwrap();
        v.retain(|e| {
            let age = now.saturating_duration_since(e.at);
            let max = match e.kind {
                NoteVisualKind::Strike => Duration::from_millis(NOTE_STRIKE_VIS_MS),
                NoteVisualKind::Strum => Duration::from_millis(NOTE_STRUM_VIS_MS),
            };
            age < max
        });
        v.clone()
    }

    pub fn handle_touch(&mut self, event: TouchEvent, width_px: f32) -> (AppEffects, bool) {
        let positions = layout::compute_note_positions_android(width_px);
        let out = self.ui.handle(UiEvent::Touch(event), &positions);

        let kind = match event.phase {
            crate::touch::TouchPhase::Down => Some(NoteVisualKind::Strike),
            crate::touch::TouchPhase::Move => Some(NoteVisualKind::Strum),
            _ => None,
        };
        if let Some(kind) = kind {
            if !out.touch_notes.is_empty() {
                let now = Instant::now();
                let mut v = self.note_visuals.lock().unwrap();
                for tn in &out.touch_notes {
                    v.push(NoteVisualEvent {
                        at: now,
                        row: tn.row,
                        note: tn.note,
                        kind,
                    });
                }
            }
        }

        (out.effects, out.haptic)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notes::UnkeyedNote;
    use crate::rows::RowId;

    #[test]
    fn android_frontend_emits_note_on_messages() {
        let mut f = AndroidFrontend::new();
        let rx = f.take_audio_rx().expect("expected audio rx");

        let effects = f
            .engine_mut()
            .handle_strum_crossing(RowId::Top, UnkeyedNote(0), crate::app_state::DEFAULT_STRUM_VOLUME);
        assert_eq!(effects.play_notes.len(), 1);

        f.push_effects(effects);

        match rx.try_recv() {
            Ok(AudioMsg::NoteOn(_)) => {}
            other => panic!("expected NoteOn msg, got {other:?}"),
        }
    }

    #[test]
    fn android_frontend_emits_note_off_before_retrigger_note_on() {
        let mut f = AndroidFrontend::new();
        let rx = f.take_audio_rx().expect("expected audio rx");

        f.push_effects(
            f.engine_mut().handle_strum_crossing(
                RowId::Top,
                UnkeyedNote(0),
                crate::app_state::DEFAULT_STRUM_VOLUME,
            ),
        );
        let _ = rx.try_recv();

        f.push_effects(
            f.engine_mut().handle_strum_crossing(
                RowId::Top,
                UnkeyedNote(0),
                crate::app_state::DEFAULT_STRUM_VOLUME,
            ),
        );

        match rx.try_recv() {
            Ok(AudioMsg::NoteOff(_)) => {}
            other => panic!("expected NoteOff msg, got {other:?}"),
        }
        match rx.try_recv() {
            Ok(AudioMsg::NoteOn(_)) => {}
            other => panic!("expected NoteOn msg, got {other:?}"),
        }
    }
}
