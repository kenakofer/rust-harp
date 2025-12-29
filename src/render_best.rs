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
