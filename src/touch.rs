use crate::strum::{self, StrumCrossing};

use std::collections::HashMap;

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct PointerId(pub u64);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TouchPhase {
    Down,
    Move,
    Up,
    Cancel,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TouchEvent {
    pub id: PointerId,
    pub phase: TouchPhase,
    pub x: f32,
    /// Normalized vertical position in [0,1].
    pub y_norm: f32,
    /// Normalized finger pressure (best-effort), typically in [0,1].
    pub pressure: f32,
}

/// Result of processing a touch/mouse event.
#[derive(Clone, Debug, PartialEq)]
pub struct TouchOutput {
    /// If "play on tap" is enabled, a touch-down can immediately "strike" the nearest unstruck string.
    pub strike: Option<crate::notes::UnkeyedNote>,
    /// Boundaries crossed since the last sample.
    pub crossings: Vec<StrumCrossing>,
}

/// Tracks pointer movement and reports strum crossings per pointer.
///
/// This is platform-agnostic: desktop mouse-drag and Android multitouch can both feed it.
pub struct TouchTracker {
    last_pos: HashMap<PointerId, (crate::rows::RowId, f32)>,

    play_on_tap: bool,

    /// Which note (if any) each pointer has claimed via a strike.
    struck_by_pointer: HashMap<PointerId, (crate::rows::RowId, crate::notes::UnkeyedNote)>,
}

impl TouchTracker {
    pub fn new() -> Self {
        Self {
            last_pos: HashMap::new(),
            play_on_tap: true,
            struck_by_pointer: HashMap::new(),
        }
    }

    pub fn set_play_on_tap(&mut self, enabled: bool) {
        self.play_on_tap = enabled;
    }

    fn nearest_unstruck_note<F: Fn(crate::rows::RowId, crate::notes::UnkeyedNote) -> bool>(
        &self,
        row: crate::rows::RowId,
        x: f32,
        note_positions: &[f32],
        allowed: &F,
    ) -> Option<crate::notes::UnkeyedNote> {
        let mut best_i: Option<usize> = None;
        let mut best_d = f32::INFINITY;

        for (i, &nx) in note_positions.iter().enumerate() {
            let note = crate::notes::UnkeyedNote(i as i16);
            if self
                .struck_by_pointer
                .values()
                .any(|&(_r, n)| n == note)
            {
                continue;
            }
            if !allowed(row, note) {
                continue;
            }

            let d = (nx - x).abs();
            if d < best_d || (d == best_d && best_i.map_or(true, |bi| i < bi)) {
                best_d = d;
                best_i = Some(i);
            }
        }

        best_i.map(|i| crate::notes::UnkeyedNote(i as i16))
    }

    pub fn handle_event(
        &mut self,
        event: TouchEvent,
        note_positions: &[f32],
        allowed: impl Fn(crate::rows::RowId, crate::notes::UnkeyedNote) -> bool,
    ) -> TouchOutput {
        let row = crate::rows::RowId::from_y_norm(event.y_norm);

        match event.phase {
            TouchPhase::Down => {
                self.last_pos.insert(event.id, (row, event.x));

                let strike = if self.play_on_tap {
                    let s = self.nearest_unstruck_note(row, event.x, note_positions, &allowed);
                    if let Some(n) = s {
                        self.struck_by_pointer.insert(event.id, (row, n));
                    }
                    s
                } else {
                    None
                };

                TouchOutput {
                    strike,
                    crossings: Vec::new(),
                }
            }
            TouchPhase::Move => {
                let Some((prev_row, prev_x)) = self.last_pos.insert(event.id, (row, event.x)) else {
                    // No prior state; treat like Down.
                    return TouchOutput {
                        strike: None,
                        crossings: Vec::new(),
                    };
                };

                if prev_row != row {
                    // Pointer moved into a different row; reset state for clean crossing detection.
                    self.struck_by_pointer.remove(&event.id);
                    return TouchOutput {
                        strike: None,
                        crossings: Vec::new(),
                    };
                }

                // If a note is currently struck in this row, suppress re-strumming that note,
                // but still allow strumming other notes.
                let struck: Vec<crate::notes::UnkeyedNote> = self
                    .struck_by_pointer
                    .values()
                    .filter_map(|&(r, n)| if r == row { Some(n) } else { None })
                    .collect();

                let mut crossings = strum::detect_crossings(prev_x, event.x, note_positions);

                // If this pointer has a struck note, unlock it once this pointer strums ANY other note.
                // (Up/Cancel will also clear it.)
                if let Some(&(struck_row, struck_note)) = self.struck_by_pointer.get(&event.id) {
                    if struck_row == row {
                        let strummed_other_note = crossings
                            .iter()
                            .flat_map(|c| c.notes.iter().copied())
                            .any(|n| n != struck_note);
                        if strummed_other_note {
                            self.struck_by_pointer.remove(&event.id);
                        }
                    }
                }

                if !struck.is_empty() {
                    crossings.retain_mut(|c| {
                        c.notes.retain(|n| !struck.contains(n));
                        !c.notes.is_empty()
                    });
                }

                TouchOutput {
                    strike: None,
                    crossings,
                }
            }
            TouchPhase::Up | TouchPhase::Cancel => {
                self.last_pos.remove(&event.id);
                self.struck_by_pointer.remove(&event.id);
                TouchOutput {
                    strike: None,
                    crossings: Vec::new(),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notes::UnkeyedNote;

    const Y: f32 = 0.25;

    #[test]
    fn move_emits_crossings_and_up_clears_state() {
        let positions = [10.0, 20.0, 30.0];
        let mut t = TouchTracker::new();
        t.set_play_on_tap(false);

        assert_eq!(
            t.handle_event(
                TouchEvent {
                    id: PointerId(1),
                    phase: TouchPhase::Down,
                    x: 5.0,
                    y_norm: Y,
                    pressure: 1.0,
},
                &positions,
                |_, _| true,
            ),
            TouchOutput {
                strike: None,
                crossings: Vec::new(),
            }
        );

        let out = t.handle_event(
            TouchEvent {
                id: PointerId(1),
                phase: TouchPhase::Move,
                x: 25.0,
                y_norm: Y,
                pressure: 1.0,
},
            &positions,
            |_, _| true,
        );
        assert_eq!(
            out.crossings,
            vec![
                StrumCrossing {
                    x: 10.0,
                    notes: vec![UnkeyedNote(0)],
                },
                StrumCrossing {
                    x: 20.0,
                    notes: vec![UnkeyedNote(1)],
                },
            ]
        );

        t.handle_event(
            TouchEvent {
                id: PointerId(1),
                phase: TouchPhase::Up,
                x: 25.0,
                y_norm: Y,
                pressure: 1.0,
},
            &positions,
            |_, _| true,
        );

        // No prior state after Up
        assert_eq!(
            t.handle_event(
                TouchEvent {
                    id: PointerId(1),
                    phase: TouchPhase::Move,
                    x: 30.0,
                    y_norm: Y,
                    pressure: 1.0,
},
                &positions,
                |_, _| true,
            ),
            TouchOutput {
                strike: None,
                crossings: Vec::new(),
            }
        );
    }

    #[test]
    fn pointers_are_independent() {
        let positions = [10.0, 20.0, 30.0];
        let mut t = TouchTracker::new();
        t.set_play_on_tap(false);

        t.handle_event(
            TouchEvent {
                id: PointerId(1),
                phase: TouchPhase::Down,
                x: 0.0,
                y_norm: Y,
                pressure: 1.0,
},
            &positions,
            |_, _| true,
        );
        t.handle_event(
            TouchEvent {
                id: PointerId(2),
                phase: TouchPhase::Down,
                x: 100.0,
                y_norm: Y,
                pressure: 1.0,
},
            &positions,
            |_, _| true,
        );

        let out1 = t.handle_event(
            TouchEvent {
                id: PointerId(1),
                phase: TouchPhase::Move,
                x: 15.0,
                y_norm: Y,
                pressure: 1.0,
},
            &positions,
            |_, _| true,
        );
        assert_eq!(
            out1.crossings,
            vec![StrumCrossing {
                x: 10.0,
                notes: vec![UnkeyedNote(0)],
            }]
        );

        let out2 = t.handle_event(
            TouchEvent {
                id: PointerId(2),
                phase: TouchPhase::Move,
                x: 5.0,
                y_norm: Y,
                pressure: 1.0,
},
            &positions,
            |_, _| true,
        );
        assert_eq!(
            out2.crossings,
            vec![
                StrumCrossing {
                    x: 10.0,
                    notes: vec![UnkeyedNote(0)],
                },
                StrumCrossing {
                    x: 20.0,
                    notes: vec![UnkeyedNote(1)],
                },
                StrumCrossing {
                    x: 30.0,
                    notes: vec![UnkeyedNote(2)],
                },
            ]
        );
    }

    #[test]
    fn down_strikes_nearest_unstruck_and_suppresses_strum_for_struck_notes() {
        // Two notes share the same x position, to mirror "stacked" strings.
        let positions = [10.0, 10.0, 20.0];
        let mut t = TouchTracker::new();

        let out1 = t.handle_event(
            TouchEvent {
                id: PointerId(1),
                phase: TouchPhase::Down,
                x: 11.0,
                y_norm: Y,
                pressure: 1.0,
},
            &positions,
            |_, _| true,
        );
        assert_eq!(out1.strike, Some(UnkeyedNote(0)));

        let out2 = t.handle_event(
            TouchEvent {
                id: PointerId(2),
                phase: TouchPhase::Down,
                x: 9.0,
                y_norm: Y,
                pressure: 1.0,
},
            &positions,
            |_, _| true,
        );
        assert_eq!(out2.strike, Some(UnkeyedNote(1)));

        // Pointer 1 movement should not strum once it has struck.
        let out3 = t.handle_event(
            TouchEvent {
                id: PointerId(1),
                phase: TouchPhase::Move,
                x: 25.0,
                y_norm: Y,
                pressure: 1.0,
},
            &positions,
            |_, _| true,
        );
        assert_eq!(
            out3.crossings,
            vec![StrumCrossing {
                x: 20.0,
                notes: vec![UnkeyedNote(2)],
            }]
        );

        // After pointer 1 lifts, another pointer can strike its note again.
        t.handle_event(
            TouchEvent {
                id: PointerId(1),
                phase: TouchPhase::Up,
                x: 25.0,
                y_norm: Y,
                pressure: 1.0,
},
            &positions,
            |_, _| true,
        );
        let out4 = t.handle_event(
            TouchEvent {
                id: PointerId(3),
                phase: TouchPhase::Down,
                x: 10.0,
                y_norm: Y,
                pressure: 1.0,
},
            &positions,
            |_, _| true,
        );
        assert_eq!(out4.strike, Some(UnkeyedNote(0)));
    }

    #[test]
    fn down_strikes_nearest_allowed_note() {
        let positions = [0.0, 10.0, 20.0];
        let mut t = TouchTracker::new();

        // Only note 2 is allowed.
        let out = t.handle_event(
            TouchEvent {
                id: PointerId(1),
                phase: TouchPhase::Down,
                x: 1.0,
                y_norm: Y,
                pressure: 1.0,
},
            &positions,
            |_, n| n == UnkeyedNote(2),
        );
        assert_eq!(out.strike, Some(UnkeyedNote(2)));
    }

    #[test]
    fn struck_note_unlocks_after_strumming_another_note() {
        let positions = [10.0, 20.0];
        let mut t = TouchTracker::new();

        let out1 = t.handle_event(
            TouchEvent {
                id: PointerId(1),
                phase: TouchPhase::Down,
                x: 11.0,
                y_norm: Y,
                pressure: 1.0,
},
            &positions,
            |_, _| true,
        );
        assert_eq!(out1.strike, Some(UnkeyedNote(0)));

        // While note 0 is struck, strumming across it should suppress it.
        let out2 = t.handle_event(
            TouchEvent {
                id: PointerId(1),
                phase: TouchPhase::Move,
                x: 0.0,
                y_norm: Y,
                pressure: 1.0,
},
            &positions,
            |_, _| true,
        );
        assert_eq!(out2.crossings, Vec::<StrumCrossing>::new());

        // Now strum another note: this should unlock the struck note for subsequent strums.
        let out3 = t.handle_event(
            TouchEvent {
                id: PointerId(1),
                phase: TouchPhase::Move,
                x: 30.0,
                y_norm: Y,
                pressure: 1.0,
},
            &positions,
            |_, _| true,
        );
        assert_eq!(
            out3.crossings,
            vec![StrumCrossing {
                x: 20.0,
                notes: vec![UnkeyedNote(1)],
            }]
        );

        // Crossing back should now include note 0 again.
        let out4 = t.handle_event(
            TouchEvent {
                id: PointerId(1),
                phase: TouchPhase::Move,
                x: 0.0,
                y_norm: Y,
                pressure: 1.0,
},
            &positions,
            |_, _| true,
        );
        assert_eq!(
            out4.crossings,
            vec![
                StrumCrossing {
                    x: 10.0,
                    notes: vec![UnkeyedNote(0)],
                },
                StrumCrossing {
                    x: 20.0,
                    notes: vec![UnkeyedNote(1)],
                },
            ]
        );
    }
}
