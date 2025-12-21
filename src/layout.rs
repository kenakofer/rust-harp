/// Pre-calculated unscaled relative x-positions for each string, ranging from 0.0 to 1.0.
///
/// Kept in a core module so desktop + Android can share the same layout.
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

pub const NOTE_TO_STRING_IN_OCTAVE: [u16; 12] = [0, 0, 1, 1, 2, 3, 3, 4, 4, 5, 6, 6];

pub const NUM_STRINGS: usize = UNSCALED_RELATIVE_X_POSITIONS.len();

pub fn compute_string_positions(width: f32) -> impl Iterator<Item = f32> {
    UNSCALED_RELATIVE_X_POSITIONS.iter().map(move |rel| rel * width)
}

/// Positions for each chromatic note (UnkeyedNote 0..N) mapped onto the physical strings.
///
/// This intentionally contains duplicates: multiple notes can map to the same physical string.
pub fn compute_note_positions(width: f32) -> Vec<f32> {
    let mut positions = Vec::new();

    for octave in 0.. {
        for uknote in 0..12 {
            let string_in_octave = NOTE_TO_STRING_IN_OCTAVE[uknote as usize] as usize;
            let string = octave * 7 + string_in_octave;
            if string >= NUM_STRINGS {
                return positions;
            }
            positions.push(UNSCALED_RELATIVE_X_POSITIONS[string] * width);
        }
    }

    positions
}

/// Android-specific note positions.
///
/// Compared to `compute_note_positions`, this:
/// - drops one low octave (7 strings) to give each remaining string more screen space
/// - evenly spaces the remaining strings across the full width
pub fn compute_note_positions_android(width: f32) -> Vec<f32> {
    compute_note_positions_evenly_spaced(width, 1)
}

fn compute_note_positions_evenly_spaced(width: f32, drop_low_octaves: usize) -> Vec<f32> {
    let start_string = drop_low_octaves.saturating_mul(7);
    let remaining_strings = NUM_STRINGS.saturating_sub(start_string);
    if remaining_strings == 0 {
        return Vec::new();
    }

    let width = width.max(1.0);
    let scale = if remaining_strings > 1 {
        (width - 1.0) / (remaining_strings as f32 - 1.0)
    } else {
        0.0
    };

    let mut positions = Vec::new();

    for octave in drop_low_octaves.. {
        for uknote in 0..12 {
            let string_in_octave = NOTE_TO_STRING_IN_OCTAVE[uknote as usize] as usize;
            let string = octave * 7 + string_in_octave;
            if string >= NUM_STRINGS {
                return positions;
            }
            let idx = string - start_string;
            positions.push(idx as f32 * scale);
        }
    }

    positions
}
