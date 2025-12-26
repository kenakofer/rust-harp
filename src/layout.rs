/// Desktop pre-calculated unscaled relative x-positions for each string, ranging from 0.0 to 1.0.
///
/// Android uses a separate layout (see `compute_note_positions_android*`).
pub const UNSCALED_RELATIVE_X_POSITIONS: &[f32] = &[
    2.03124999999999972e-02,
    5.19531250000000028e-02,
    9.02343750000000056e-02,
    1.31445312499999994e-01,
    1.63281249999999989e-01,
    1.96289062500000000e-01,
    2.33203125000000011e-01,
    2.66015624999999978e-01,
    3.05859374999999989e-01,
    3.38867187500000000e-01,
    3.75000000000000000e-01,
    4.05468749999999989e-01,
    4.49414062499999989e-01,
    4.85546874999999989e-01,
    5.20312499999999956e-01,
    5.55273437499999911e-01,
    5.92578124999999956e-01,
    6.29687499999999956e-01,
    6.65429687500000089e-01,
    6.99999999999999956e-01,
    7.34960937500000022e-01,
    7.71289062499999956e-01,
    8.07617187500000000e-01,
    8.42773437500000000e-01,
    8.80664062499999956e-01,
    9.18359374999999978e-01,
    9.49999999999999956e-01,
    9.91796875000000022e-01,
];

pub const NUM_STRINGS: usize = UNSCALED_RELATIVE_X_POSITIONS.len();

fn note_x_from_strings(pc: i32, string_x: &[f32; 7]) -> Option<f32> {
    Some(match pc {
        0 => string_x[0],
        1 => (string_x[0] + string_x[1]) * 0.5,
        2 => string_x[1],
        3 => (string_x[1] + string_x[2]) * 0.5,
        4 => string_x[2],
        5 => string_x[3],
        6 => (string_x[3] + string_x[4]) * 0.5,
        7 => string_x[4],
        8 => (string_x[4] + string_x[5]) * 0.5,
        9 => string_x[5],
        10 => (string_x[5] + string_x[6]) * 0.5,
        11 => string_x[6],
        _ => return None,
    })
}

fn required_string_indices(pc: i32) -> Option<&'static [usize]> {
    Some(match pc {
        0 => &[0],
        1 => &[0, 1],
        2 => &[1],
        3 => &[1, 2],
        4 => &[2],
        5 => &[3],
        6 => &[3, 4],
        7 => &[4],
        8 => &[4, 5],
        9 => &[5],
        10 => &[5, 6],
        11 => &[6],
        _ => return None,
    })
}

pub fn compute_string_positions(width: f32) -> impl Iterator<Item = f32> {
    UNSCALED_RELATIVE_X_POSITIONS
        .iter()
        .map(move |rel| rel * width)
}

/// Positions for each chromatic note (UnkeyedNote 0..N).
///
/// We keep the 7 "white key" strings per octave exactly where they are, and place the 5
/// chromatic notes halfway between their neighbors.
pub fn compute_note_positions(width: f32) -> Vec<f32> {
    let mut positions = Vec::new();

    for octave in 0.. {
        for pc in 0..12 {
            let Some(req) = required_string_indices(pc) else {
                continue;
            };

            let base = octave * 7;
            if req.iter().any(|&s| base + s >= NUM_STRINGS) {
                return positions;
            }

            let mut string_x = [0.0f32; 7];
            for &s in req {
                string_x[s] = UNSCALED_RELATIVE_X_POSITIONS[base + s] * width;
            }
            // Fill any other needed indices for midpoint math.
            for s in 0..7 {
                if base + s < NUM_STRINGS {
                    string_x[s] = UNSCALED_RELATIVE_X_POSITIONS[base + s] * width;
                }
            }

            if let Some(x) = note_x_from_strings(pc, &string_x) {
                positions.push(x);
            }
        }
    }

    positions
}

/// Android layout config.
///
/// We intentionally keep Android layout separate from desktop: the phone needs fewer,
/// wider-spaced strings for reliable touch.
pub const ANDROID_NUM_STRINGS: usize = 22;

/// Controls how much horizontal space chromatic (black-key) semitone steps consume
/// relative to the two diatonic semitone steps (E→F and B→C).
///
/// - 1.0: all 12 semitones per octave are evenly spaced.
/// - 0.5: matches our prior Android behavior (chromatic steps are half as wide as the
///        diatonic semitone steps), which keeps the 7 diatonic notes per octave evenly spaced.
pub const CHROMATIC_SPACING_RATIO: f32 = 0.75;

/// Android-only: which `UnkeyedNote` should map to the first (left-most) physical string.
///
/// This is intentionally independent from desktop. TODO things get weird if this isn't a multiple
/// of 12
pub const ANDROID_LOWEST_NOTE: i16 = 24;

/// Android-specific note positions.
///
/// - Uses a fixed 22 physical strings
/// - Evenly spaces them across the screen
/// - Allows shifting the lowest playable note via `ANDROID_LOWEST_NOTE`
///
/// The returned vector is indexed by `UnkeyedNote` (0..N). Notes below
/// `ANDROID_LOWEST_NOTE` are represented with non-finite x positions so touch + render
/// logic can ignore them without needing special casing.
pub fn compute_note_positions_android(width: f32) -> Vec<f32> {
    compute_note_positions_android_with_lowest(width, ANDROID_LOWEST_NOTE)
}

pub fn compute_note_positions_android_with_lowest(width: f32, lowest_note: i16) -> Vec<f32> {
    let width = width.max(1.0);

    let strings = ANDROID_NUM_STRINGS;
    if strings == 0 {
        return Vec::new();
    }

    // Small padding so the first/last string isn't clipped by the edge.
    let pad = 5.0f32;
    let usable = (width - 2.0 * pad).max(1.0);

    fn semitone_step(pc: i32) -> f32 {
        // In major diatonic (heptatonic) layout, E→F (4→5) and B→C (11→0) are the two
        // half-step gaps; everything else is a whole-step region.
        if pc == 4 || pc == 11 {
            1.0
        } else {
            CHROMATIC_SPACING_RATIO
        }
    }

    // Offsets within an octave (pitch class 0..11).
    let mut pc_units = [0.0f32; 12];
    for pc in 1..12 {
        pc_units[pc] = pc_units[pc - 1] + semitone_step((pc - 1) as i32);
    }
    let octave_units: f32 = (0..12).map(semitone_step).sum();

    // Keep indices aligned with UnkeyedNote (i as i16). Notes below lowest_note are dummy.
    let dummy_len = lowest_note.max(0) as usize;
    let mut positions: Vec<f32> = vec![f32::NEG_INFINITY; dummy_len];

    // Determine which chromatic notes exist based on the number of diatonic "anchor" strings,
    // matching the previous range/termination behavior.
    let mut units: Vec<f32> = Vec::new();
    for rel_note in 0.. {
        let octave = rel_note / 12;
        let pc = rel_note % 12;

        let Some(req) = required_string_indices(pc as i32) else {
            continue;
        };

        let base = octave * 7;
        if req.iter().any(|&s| base + s >= strings) {
            break;
        }

        units.push(octave as f32 * octave_units + pc_units[pc]);
    }

    let max_units = units.last().copied().unwrap_or(0.0).max(1.0);
    let scale = usable / max_units;
    positions.extend(units.into_iter().map(|u| pad + u * scale));

    positions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_black_keys_are_midpoints() {
        let w = 1000.0f32;
        let pos = compute_note_positions(w);

        // C# is between C and D.
        assert!((pos[1] - (pos[0] + pos[2]) * 0.5).abs() < 0.0001);
        // D# is between D and E.
        assert!((pos[3] - (pos[2] + pos[4]) * 0.5).abs() < 0.0001);
        // F# is between F and G.
        assert!((pos[6] - (pos[5] + pos[7]) * 0.5).abs() < 0.0001);
        // G# is between G and A.
        assert!((pos[8] - (pos[7] + pos[9]) * 0.5).abs() < 0.0001);
        // A# is between A and B.
        assert!((pos[10] - (pos[9] + pos[11]) * 0.5).abs() < 0.0001);
    }

    #[test]
    fn android_chromatic_spacing_ratio_is_applied() {
        let w = 1000.0f32;
        let pos = compute_note_positions_android_with_lowest(w, 0);

        // Regardless of ratio, black keys should be centered between the adjacent diatonic notes
        // within a whole-step region.
        assert!((pos[1] - (pos[0] + pos[2]) * 0.5).abs() < 0.0001);
        assert!((pos[3] - (pos[2] + pos[4]) * 0.5).abs() < 0.0001);
        assert!((pos[6] - (pos[5] + pos[7]) * 0.5).abs() < 0.0001);
        assert!((pos[8] - (pos[7] + pos[9]) * 0.5).abs() < 0.0001);
        assert!((pos[10] - (pos[9] + pos[11]) * 0.5).abs() < 0.0001);

        // But the gap across a whole-step region (C→D) should scale with the ratio.
        // In our model, C→D spans two "chromatic" semitone steps.
        let cd = pos[2] - pos[0];
        let ef = pos[5] - pos[4];
        assert!(cd > ef); // whole-step region is wider than the diatonic semitone gap
    }
}
