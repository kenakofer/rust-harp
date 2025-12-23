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

/// Android layout config.
///
/// We intentionally keep Android layout separate from desktop: the phone needs fewer,
/// wider-spaced strings for reliable touch.
pub const ANDROID_NUM_STRINGS: usize = 22;

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
    let pad = 2.0f32;
    let usable = (width - 2.0 * pad).max(1.0);
    let step = if strings > 1 {
        usable / (strings as f32 - 1.0)
    } else {
        0.0
    };

    // Keep indices aligned with UnkeyedNote (i as i16). Notes below lowest_note are dummy.
    let dummy_len = lowest_note.max(0) as usize;
    let mut positions: Vec<f32> = vec![f32::NEG_INFINITY; dummy_len];

    // Build chromatic notes in order, mapping them onto a 7-strings-per-octave physical layout,
    // but capped by `ANDROID_NUM_STRINGS`.
    for rel_note in 0.. {
        let octave = rel_note / 12;
        let pc = rel_note % 12;
        let string_in_octave = NOTE_TO_STRING_IN_OCTAVE[pc as usize] as usize;
        let string = octave * 7 + string_in_octave;
        if string >= strings {
            break;
        }

        positions.push(pad + string as f32 * step);
    }

    positions
}
