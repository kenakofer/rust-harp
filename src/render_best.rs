use crate::chord::Chord;
use crate::notes::UnkeyedNote;

#[derive(Clone, Copy)]
pub(crate) struct BestStyle<C: Copy> {
    pub(crate) root_prio: u8,
    pub(crate) root_color: C,
    pub(crate) chord_prio: u8,
    pub(crate) chord_color: C,
    pub(crate) inactive_prio: u8,
    pub(crate) inactive_color: C,
}

pub(crate) fn compute_best_per_x<C: Copy>(
    width: usize,
    positions: &[f32],
    chord: Option<Chord>,
    transpose_pc: i16,
    style: BestStyle<C>,
    include_inactive: bool,
) -> (Vec<u8>, Vec<C>, Vec<u8>) {
    let mut best_prio_per_x: Vec<u8> = vec![0; width];
    let mut best_color_per_x: Vec<C> = vec![style.inactive_color; width];
    let mut best_pc_per_x: Vec<u8> = vec![255; width];

    for (i, x) in positions.iter().enumerate() {
        let uknote = UnkeyedNote(i as i16);
        let xi = x.round() as i32;
        if xi < 0 || xi >= width as i32 {
            continue;
        }
        let xi = xi as usize;

        let (prio, color) = if let Some(ch) = chord {
            if ch.has_root(uknote) {
                (style.root_prio, style.root_color)
            } else if ch.contains(uknote) {
                (style.chord_prio, style.chord_color)
            } else if include_inactive {
                (style.inactive_prio, style.inactive_color)
            } else {
                continue;
            }
        } else if include_inactive {
            (style.inactive_prio, style.inactive_color)
        } else {
            continue;
        };

        if prio > best_prio_per_x[xi] {
            best_prio_per_x[xi] = prio;
            best_color_per_x[xi] = color;
            best_pc_per_x[xi] = (uknote.wrap_to_octave() + transpose_pc).rem_euclid(12) as u8;
        }
    }

    (best_prio_per_x, best_color_per_x, best_pc_per_x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn best_per_x_root_wins_on_collision_and_pc_transposes() {
        // Force root (i=0) and a chord tone (i=4) to round to the same xi.
        let mut positions = vec![-100.0f32; 8];
        positions[0] = 1.2;
        positions[4] = 1.3;

        let style = BestStyle {
            root_prio: 2,
            root_color: 10u32,
            chord_prio: 1,
            chord_color: 20u32,
            inactive_prio: 0,
            inactive_color: 0u32,
        };

        let chord = crate::chord::Chord::new_triad(UnkeyedNote(0));

        let (prio0, color0, pc0) = compute_best_per_x(10, &positions, Some(chord), 0, style, false);
        assert_eq!(prio0[1], 2);
        assert_eq!(color0[1], 10);
        assert_eq!(pc0[1], 0);

        let (_prio2, _color2, pc2) = compute_best_per_x(10, &positions, Some(chord), 2, style, false);
        assert_eq!(pc2[1], 2);
    }

    #[test]
    fn best_per_x_skips_inactive_when_disabled() {
        // i=1 is not in the C major triad.
        let mut positions = vec![-100.0f32; 8];
        positions[1] = 2.2; // rounds to 2

        let style = BestStyle {
            root_prio: 2,
            root_color: 1u8,
            chord_prio: 1,
            chord_color: 2u8,
            inactive_prio: 9,
            inactive_color: 3u8,
        };

        let chord = crate::chord::Chord::new_triad(UnkeyedNote(0));
        let (prio, _color, _pc) = compute_best_per_x(10, &positions, Some(chord), 0, style, false);
        assert_eq!(prio[2], 0);
    }

    #[test]
    fn best_per_x_emits_inactive_when_enabled() {
        // i=1 is not in the C major triad.
        let mut positions = vec![-100.0f32; 8];
        positions[1] = 2.2; // rounds to 2

        let style = BestStyle {
            root_prio: 3,
            root_color: 1u8,
            chord_prio: 2,
            chord_color: 2u8,
            inactive_prio: 1,
            inactive_color: 7u8,
        };

        let chord = crate::chord::Chord::new_triad(UnkeyedNote(0));
        let (prio, color, pc) = compute_best_per_x(10, &positions, Some(chord), 0, style, true);
        assert_eq!(prio[2], 1);
        assert_eq!(color[2], 7);
        assert_eq!(pc[2], 1);
    }
}
