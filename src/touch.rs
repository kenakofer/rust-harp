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
}

/// Tracks pointer movement and reports strum crossings per pointer.
///
/// This is platform-agnostic: desktop mouse-drag and Android multitouch can both feed it.
pub struct TouchTracker {
    last_x: HashMap<PointerId, f32>,
}

impl TouchTracker {
    pub fn new() -> Self {
        Self {
            last_x: HashMap::new(),
        }
    }

    pub fn handle_event(&mut self, event: TouchEvent, note_positions: &[f32]) -> Vec<StrumCrossing> {
        match event.phase {
            TouchPhase::Down => {
                self.last_x.insert(event.id, event.x);
                Vec::new()
            }
            TouchPhase::Move => {
                let Some(prev) = self.last_x.insert(event.id, event.x) else {
                    // No prior state; treat like Down.
                    return Vec::new();
                };
                strum::detect_crossings(prev, event.x, note_positions)
            }
            TouchPhase::Up | TouchPhase::Cancel => {
                self.last_x.remove(&event.id);
                Vec::new()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notes::UnkeyedNote;

    #[test]
    fn move_emits_crossings_and_up_clears_state() {
        let positions = [10.0, 20.0, 30.0];
        let mut t = TouchTracker::new();

        assert!(t
            .handle_event(
                TouchEvent {
                    id: PointerId(1),
                    phase: TouchPhase::Down,
                    x: 5.0,
                },
                &positions
            )
            .is_empty());

        let c = t.handle_event(
            TouchEvent {
                id: PointerId(1),
                phase: TouchPhase::Move,
                x: 25.0,
            },
            &positions,
        );
        assert_eq!(
            c,
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
            },
            &positions,
        );

        // No prior state after Up
        assert!(t
            .handle_event(
                TouchEvent {
                    id: PointerId(1),
                    phase: TouchPhase::Move,
                    x: 30.0,
                },
                &positions
            )
            .is_empty());
    }

    #[test]
    fn pointers_are_independent() {
        let positions = [10.0, 20.0, 30.0];
        let mut t = TouchTracker::new();

        t.handle_event(
            TouchEvent {
                id: PointerId(1),
                phase: TouchPhase::Down,
                x: 0.0,
            },
            &positions,
        );
        t.handle_event(
            TouchEvent {
                id: PointerId(2),
                phase: TouchPhase::Down,
                x: 100.0,
            },
            &positions,
        );

        let c1 = t.handle_event(
            TouchEvent {
                id: PointerId(1),
                phase: TouchPhase::Move,
                x: 15.0,
            },
            &positions,
        );
        assert_eq!(
            c1,
            vec![StrumCrossing {
                x: 10.0,
                notes: vec![UnkeyedNote(0)],
            }]
        );

        let c2 = t.handle_event(
            TouchEvent {
                id: PointerId(2),
                phase: TouchPhase::Move,
                x: 5.0,
            },
            &positions,
        );
        assert_eq!(
            c2,
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
}
