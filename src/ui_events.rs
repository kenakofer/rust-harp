use crate::app_state::{AppEffects, KeyState};
use crate::engine::Engine;
use crate::input_map::{self, UiButton, UiKey};
use crate::notes::{NoteVolume, Transpose, UnkeyedNote};
use crate::rows::RowId;
use crate::touch::{TouchEvent, TouchTracker};

#[derive(Clone, Debug, PartialEq)]
pub enum UiEvent {
    Key { state: KeyState, key: UiKey },
    Button { state: KeyState, button: UiButton },
    Touch(TouchEvent),
    SetPlayOnTap(bool),
    SetTranspose(Transpose),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TouchNote {
    pub row: RowId,
    pub note: UnkeyedNote,
}

#[derive(Debug)]
pub struct UiOutput {
    pub effects: AppEffects,
    /// True when the UI should emit a haptic "tick" for this event.
    pub haptic: bool,
    /// Touch-triggered notes (strike + strum crossings), expressed as unkeyed string indices.
    pub touch_notes: Vec<TouchNote>,
}

fn empty_effects() -> AppEffects {
    AppEffects {
        play_notes: Vec::new(),
        stop_notes: Vec::new(),
        redraw: false,
        change_key: None,
    }
}

fn touch_volume(pressure: f32) -> NoteVolume {
    // Prototype mapping (tune these constants based on device feel).
    let p = pressure.clamp(0.0, 1.0);
    let min = 25.0;
    let max = 110.0;
    NoteVolume((min + (max - min) * p).round() as u8)
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
                let row = RowId::from_y_norm(te.y_norm);
                let vol = touch_volume(te.pressure);

                let out = self.touch.handle_event(te, note_positions, |r, n| {
                    match self.engine.active_chord_for_row(r) {
                        Some(c) => c.contains(n),
                        None => true,
                    }
                });

                let mut effects = empty_effects();
                let mut touch_notes = Vec::new();

                let active_chord = self.engine.active_chord_for_row(row);

                if let Some(note) = out.strike {
                    touch_notes.push(TouchNote { row, note });
                    merge_effects(
                        &mut effects,
                        self.engine.handle_strum_crossing(row, note, vol),
                    );
                }
                for crossing in out.crossings {
                    for note in crossing.notes {
                        // Chromatic "in-between" strings should only exist when active.
                        if crate::notes::is_black_key(note) {
                            if let Some(ch) = active_chord {
                                if !ch.contains(note) {
                                    continue;
                                }
                            } else {
                                continue;
                            }
                        }

                        touch_notes.push(TouchNote { row, note });
                        merge_effects(
                            &mut effects,
                            self.engine.handle_strum_crossing(row, note, vol),
                        );
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
            y_norm: 0.25,
            pressure: 1.0,
});
        log.record(e2.clone());
        let out2 = s1.handle(e2, &positions);
        assert_eq!(out2.touch_notes.len(), 1);
        assert_eq!(
            out2.touch_notes[0],
            TouchNote {
                row: RowId::Top,
                note: UnkeyedNote(0)
            }
        );

        // Replay into a fresh session and compare key state snapshots.
        let mut s2 = UiSession::new();
        let _ = log.replay(&mut s2, &positions);

        assert_eq!(s1.engine().active_chord(), s2.engine().active_chord());
        let a1: std::collections::BTreeSet<i16> = s1.engine().active_notes().map(|n| n.0).collect();
        let a2: std::collections::BTreeSet<i16> = s2.engine().active_notes().map(|n| n.0).collect();
        assert_eq!(a1, a2);
    }

    #[test]
    fn touch_move_emits_haptic_and_touch_notes() {
        let positions: Vec<f32> = (0..12).map(|i| i as f32).collect();
        let mut s = UiSession::new();

        // Disable tap-strike so we only test strum crossings here.
        let _ = s.handle(UiEvent::SetPlayOnTap(false), &positions);

        let _ = s.handle(
            UiEvent::Touch(TouchEvent {
                id: crate::touch::PointerId(1),
                phase: crate::touch::TouchPhase::Down,
                x: -1.0,
                y_norm: 0.25,
                pressure: 1.0,
}),
            &positions,
        );

        let out = s.handle(
            UiEvent::Touch(TouchEvent {
                id: crate::touch::PointerId(1),
                phase: crate::touch::TouchPhase::Move,
                x: 2.2,
                y_norm: 0.25,
                pressure: 1.0,
}),
            &positions,
        );

        assert!(out.haptic);
        assert_eq!(
            out.touch_notes,
            vec![
                TouchNote {
                    row: RowId::Top,
                    note: UnkeyedNote(0)
                },
                // Note 1 is a black key; with no active chord, we suppress chromatic strings.
                TouchNote {
                    row: RowId::Top,
                    note: UnkeyedNote(2)
                },
            ]
        );
    }

    #[test]
    fn ui_event_log_replay_preserves_transpose_and_play_on_tap() {
        let positions: Vec<f32> = (0..12).map(|i| i as f32).collect();

        let mut s1 = UiSession::new();
        let mut log = UiEventLog::default();

        let e1 = UiEvent::SetTranspose(crate::notes::Transpose(3));
        log.record(e1.clone());
        let _ = s1.handle(e1, &positions);
        assert_eq!(s1.engine().transpose().0, 3);

        // With play-on-tap disabled, touch-down should not strike.
        let e2 = UiEvent::SetPlayOnTap(false);
        log.record(e2.clone());
        let _ = s1.handle(e2, &positions);

        let e3 = UiEvent::Touch(TouchEvent {
            id: crate::touch::PointerId(1),
            phase: crate::touch::TouchPhase::Down,
            x: 0.1,
            y_norm: 0.25,
            pressure: 1.0,
});
        log.record(e3.clone());
        let out3 = s1.handle(e3, &positions);
        assert_eq!(out3.touch_notes, Vec::<TouchNote>::new());

        // Re-enable play-on-tap; now touch-down should strike the nearest allowed note.
        let e4 = UiEvent::SetPlayOnTap(true);
        log.record(e4.clone());
        let _ = s1.handle(e4, &positions);

        let e5 = UiEvent::Touch(TouchEvent {
            id: crate::touch::PointerId(2),
            phase: crate::touch::TouchPhase::Down,
            x: 0.1,
            y_norm: 0.25,
            pressure: 1.0,
});
        log.record(e5.clone());
        let out5 = s1.handle(e5, &positions);
        assert_eq!(
            out5.touch_notes,
            vec![TouchNote {
                row: RowId::Top,
                note: UnkeyedNote(0)
            }]
        );

        let mut s2 = UiSession::new();
        let _ = log.replay(&mut s2, &positions);

        assert_eq!(s1.engine().transpose(), s2.engine().transpose());
        let a1: std::collections::BTreeSet<i16> = s1.engine().active_notes().map(|n| n.0).collect();
        let a2: std::collections::BTreeSet<i16> = s2.engine().active_notes().map(|n| n.0).collect();
        assert_eq!(a1, a2);
    }
}
