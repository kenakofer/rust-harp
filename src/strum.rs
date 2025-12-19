use crate::notes::UnkeyedNote;

#[derive(Clone, Debug, PartialEq)]
pub struct StrumCrossing {
    pub x: f32,
    pub notes: Vec<UnkeyedNote>,
}

/// Given a segment from `x1` to `x2` and the x-positions for each note,
/// returns the distinct string boundaries crossed and which notes share that boundary.
///
/// Crossing rule matches the desktop implementation: `pos > min_x && pos <= max_x`.
pub fn detect_crossings(x1: f32, x2: f32, note_positions: &[f32]) -> Vec<StrumCrossing> {
    let min_x = x1.min(x2);
    let max_x = x1.max(x2);

    let mut out: Vec<StrumCrossing> = Vec::new();

    let mut i = 0usize;
    while i < note_positions.len() {
        let x = note_positions[i];
        let mut notes = vec![UnkeyedNote(i as i16)];

        i += 1;
        while i < note_positions.len() && note_positions[i] == x {
            notes.push(UnkeyedNote(i as i16));
            i += 1;
        }

        if x > min_x && x <= max_x {
            out.push(StrumCrossing { x, notes });
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_duplicate_positions_and_filters_by_range() {
        let pos = [10.0, 10.0, 20.0, 30.0, 30.0];

        let crossings = detect_crossings(12.0, 30.0, &pos);
        assert_eq!(
            crossings,
            vec![
                StrumCrossing {
                    x: 20.0,
                    notes: vec![UnkeyedNote(2)],
                },
                StrumCrossing {
                    x: 30.0,
                    notes: vec![UnkeyedNote(3), UnkeyedNote(4)],
                },
            ]
        );

        // strict: pos == min_x does not count
        assert!(detect_crossings(10.0, 10.0, &pos).is_empty());

        // but pos == max_x does count
        assert_eq!(
            detect_crossings(10.0, 9.0, &pos),
            vec![StrumCrossing {
                x: 10.0,
                notes: vec![UnkeyedNote(0), UnkeyedNote(1)],
            }]
        );
    }
}
