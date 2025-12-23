use crate::app_state::{AppEffects, KeyState};
use crate::engine::Engine;
use crate::input_map::{self, UiButton, UiKey};
use crate::notes::{Transpose, UnkeyedNote};
use crate::touch::{TouchEvent, TouchTracker};

#[derive(Clone, Debug, PartialEq)]
pub enum UiEvent {
    Key { state: KeyState, key: UiKey },
    Button { state: KeyState, button: UiButton },
    Touch(TouchEvent),
    SetPlayOnTap(bool),
    SetTranspose(Transpose),
}

#[derive(Debug)]
pub struct UiOutput {
    pub effects: AppEffects,
    /// True when the UI should emit a haptic "tick" for this event.
    pub haptic: bool,
    /// Touch-triggered notes (strike + strum crossings), expressed as unkeyed string indices.
    pub touch_notes: Vec<UnkeyedNote>,
}

fn empty_effects() -> AppEffects {
    AppEffects {
        play_notes: Vec::new(),
        stop_notes: Vec::new(),
        redraw: false,
        change_key: None,
    }
}

fn merge_effects(a: &mut AppEffects, b: AppEffects) {
    a.redraw |= b.redraw;
    if a.change_key.is_none() {
        a.change_key = b.change_key;
    }
    a.stop_notes.extend(b.stop_notes);
    a.play_notes.extend(b.play_notes);
}

/// Platform-agnostic UI event processor.
///
/// Frontends (desktop, Android, future) can translate their raw input into `UiEvent`s,
/// and optionally record/replay those streams for regression testing.
pub struct UiSession {
    engine: Engine,
    touch: TouchTracker,
}

impl UiSession {
    pub fn new() -> Self {
        Self {
            engine: Engine::new(),
            touch: TouchTracker::new(),
        }
    }

    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    pub fn engine_mut(&mut self) -> &mut Engine {
        &mut self.engine
    }

    pub fn set_play_on_tap(&mut self, enabled: bool) {
        self.touch.set_play_on_tap(enabled);
    }

    pub fn handle(&mut self, event: UiEvent, note_positions: &[f32]) -> UiOutput {
        match event {
            UiEvent::SetPlayOnTap(enabled) => {
                self.touch.set_play_on_tap(enabled);
                UiOutput {
                    effects: empty_effects(),
                    haptic: false,
                    touch_notes: Vec::new(),
                }
            }
            UiEvent::SetTranspose(t) => UiOutput {
                effects: self.engine.set_transpose(t),
                haptic: false,
                touch_notes: Vec::new(),
            },
            UiEvent::Key { state, key } => {
                let mut effects = empty_effects();
                if let Some(ev) = input_map::key_event_from_ui(state, key) {
                    merge_effects(&mut effects, self.engine.handle_event(ev));
                }
                UiOutput {
                    effects,
                    haptic: false,
                    touch_notes: Vec::new(),
                }
            }
            UiEvent::Button { state, button } => {
                let mut effects = empty_effects();
                for ev in input_map::key_events_from_button(state, button) {
                    merge_effects(&mut effects, self.engine.handle_event(ev));
                }
                UiOutput {
                    effects,
                    haptic: false,
                    touch_notes: Vec::new(),
                }
            }
            UiEvent::Touch(te) => {
                let chord = *self.engine.active_chord();
                let out = self.touch.handle_event(te, note_positions, |n| match chord {
                    Some(c) => c.contains(n),
                    None => true,
                });

                let mut effects = empty_effects();
                let mut touch_notes = Vec::new();

                if let Some(note) = out.strike {
                    touch_notes.push(note);
                    merge_effects(&mut effects, self.engine.handle_strum_crossing(note));
                }
                for crossing in out.crossings {
                    for note in crossing.notes {
                        touch_notes.push(note);
                        merge_effects(&mut effects, self.engine.handle_strum_crossing(note));
                    }
                }

                UiOutput {
                    haptic: !touch_notes.is_empty(),
                    effects,
                    touch_notes,
                }
            }
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct UiEventLog {
    pub events: Vec<UiEvent>,
}

impl UiEventLog {
    pub fn record(&mut self, event: UiEvent) {
        self.events.push(event);
    }

    pub fn replay(&self, session: &mut UiSession, note_positions: &[f32]) -> AppEffects {
        let mut effects = empty_effects();
        for e in &self.events {
            let out = session.handle(e.clone(), note_positions);
            merge_effects(&mut effects, out.effects);
        }
        effects
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_state::KeyState;

    #[test]
    fn ui_event_log_replay_matches_state() {
        let positions: Vec<f32> = (0..12).map(|i| i as f32).collect();

        let mut s1 = UiSession::new();
        let mut log = UiEventLog::default();

        // Hold I chord, then tap near an inactive string (1) => should strike nearest active chord note.
        let e1 = UiEvent::Key {
            state: KeyState::Pressed,
            key: UiKey::Char('d'),
        };
        log.record(e1.clone());
        let _ = s1.handle(e1, &positions);

        let e2 = UiEvent::Touch(TouchEvent {
            id: crate::touch::PointerId(1),
            phase: crate::touch::TouchPhase::Down,
            x: 1.2,
        });
        log.record(e2.clone());
        let out2 = s1.handle(e2, &positions);
        assert_eq!(out2.touch_notes.len(), 1);
        assert_eq!(out2.touch_notes[0], UnkeyedNote(0));

        // Replay into a fresh session and compare key state snapshots.
        let mut s2 = UiSession::new();
        let _ = log.replay(&mut s2, &positions);

        assert_eq!(s1.engine().active_chord(), s2.engine().active_chord());
        let a1: Vec<_> = s1.engine().active_notes().collect();
        let a2: Vec<_> = s2.engine().active_notes().collect();
        assert_eq!(a1, a2);
    }
}
