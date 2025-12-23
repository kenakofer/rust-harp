use std::ops::{Add, Sub};

#[repr(transparent)]
#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq, PartialOrd)]
pub struct MidiNote(pub u8);

#[repr(transparent)]
#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq, PartialOrd)]
pub struct NoteVolume(pub u8); // 0..=127

impl Sub for MidiNote {
    type Output = Interval;
    fn sub(self, rhs: MidiNote) -> Interval {
        Interval(self.0 as i16 - rhs.0 as i16)
    }
}

#[repr(transparent)]
#[derive(Hash, Eq, Copy, Clone, Debug, PartialEq)]
pub struct UnmidiNote(pub i16); // Note before building on the MIDI_BASE_TRANSPOSE

#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Transpose(pub i16); // Basically an interval

impl Transpose {
    pub fn center_octave(self) -> Transpose {
        let in_octave = self.wrap_to_octave();
        if in_octave > 6 {
            Transpose(in_octave - 12)
        } else {
            Transpose(in_octave)
        }
    }

    pub fn wrap_to_octave(self) -> i16 {
        self.0.rem_euclid(12)
    }
}

impl Add<UnkeyedNote> for Transpose {
    type Output = UnmidiNote;
    fn add(self, rhs: UnkeyedNote) -> UnmidiNote {
        let sum: i16 = (self.0 as i16) + (rhs.0 as i16);
        UnmidiNote(sum)
    }
}

impl Add<UnmidiNote> for Transpose {
    type Output = MidiNote;
    fn add(self, rhs: UnmidiNote) -> MidiNote {
        let sum: i16 = (self.0 as i16) + (rhs.0 as i16);
        return MidiNote(sum.clamp(0, 127) as u8);
    }
}

impl Sub<Transpose> for UnmidiNote {
    type Output = UnkeyedNote;
    fn sub(self, rhs: Transpose) -> UnkeyedNote {
        let diff: i16 = (self.0 as i16) - (rhs.0 as i16);
        UnkeyedNote(diff)
    }
}

impl Sub<Transpose> for MidiNote {
    type Output = UnmidiNote;
    fn sub(self, rhs: Transpose) -> UnmidiNote {
        let diff: i16 = (self.0 as i16) - rhs.0;
        UnmidiNote(diff)
    }
}

// Note before transposing into the key or building on the MIDI_BASE_TRANSPOSE.
// This is basically solfege: Do = 0, Re = 2, etc. Can go beyond 12 or below 0
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct UnkeyedNote(pub i16);

impl Sub for UnkeyedNote {
    type Output = Interval;
    fn sub(self, rhs: UnkeyedNote) -> Interval {
        Interval(self.0 - rhs.0)
    }
}

impl UnkeyedNote {
    pub fn wrap_to_octave(self) -> i16 {
        self.0.rem_euclid(12)
    }

    // Allow coercing to i16
    pub fn as_i16(self) -> i16 {
        self.0
    }
}

// Position above the root of the chord
// "The fifth in the chord" would be 7 for example
//#[derive(Copy, Clone, Debug, PartialEq)]
pub struct UnrootedNote(pub u8);
impl UnrootedNote {
    pub fn new(i: Interval) -> Self {
        Self(i.0.rem_euclid(12) as u8)
    }
}

// Difference in half steps
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Interval(i16);

impl Interval {
    pub fn ratio(self, denom: Interval) -> f32 {
        self.0 as f32 / denom.0 as f32
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct PitchClassSet(pub u16);

impl PitchClassSet {
    pub const ROOT_ONLY: PitchClassSet = PitchClassSet(0b000000000001);
    pub const MAJOR_TRI: PitchClassSet = PitchClassSet(0b000010010001);
    pub const MINOR_TRI: PitchClassSet = PitchClassSet(0b000010001001);
    pub const DIMIN_TRI: PitchClassSet = PitchClassSet(0b000001001001);

    pub fn contains(&self, pc: UnrootedNote) -> bool {
        self.0 & (1 << pc.0) != 0
    }

    pub fn insert(&mut self, pc: UnrootedNote) {
        self.0 |= 1 << pc.0;
    }

    pub fn remove(&mut self, pc: UnrootedNote) {
        self.0 &= !(1 << pc.0);
    }
}

impl std::fmt::Debug for PitchClassSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PitchClassSet({:012b})", self.0)
    }
}

pub fn prefer_flats_for_key(key_pc: i16) -> bool {
    let k = key_pc.rem_euclid(12);
    matches!(k, 1 | 3 | 5 | 8 | 10) // Db, Eb, F, Ab, Bb
}

pub fn pitch_class_label(pc: i16, key_pc: i16) -> &'static str {
    let pc = pc.rem_euclid(12);
    match (prefer_flats_for_key(key_pc), pc) {
        (false, 0) => "C",
        (false, 1) => "C#",
        (false, 2) => "D",
        (false, 3) => "D#",
        (false, 4) => "E",
        (false, 5) => "F",
        (false, 6) => "F#",
        (false, 7) => "G",
        (false, 8) => "G#",
        (false, 9) => "A",
        (false, 10) => "A#",
        (false, 11) => "B",

        (true, 0) => "C",
        (true, 1) => "Db",
        (true, 2) => "D",
        (true, 3) => "Eb",
        (true, 4) => "E",
        (true, 5) => "F",
        (true, 6) => "Gb",
        (true, 7) => "G",
        (true, 8) => "Ab",
        (true, 9) => "A",
        (true, 10) => "Bb",
        (true, 11) => "B",
        _ => "?",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transpose_center_octave() {
        for pair in [
            (0, 0),
            (6, 6),
            (7, -5),
            (12, 0),
            (13, 1),
            (19, -5),
            (-5, -5),
            (-6, 6),
            (-7, 5),
        ] {
            assert_eq!(Transpose(pair.0).center_octave(), Transpose(pair.1));
        }
    }

    #[test]
    fn transpose_add_unkeyed_note() {
        for triple in [(0, 0, 0), (10, 5, 15), (-2, 4, 2), (-8, -10, -18)] {
            assert_eq!(
                Transpose(triple.0) + UnkeyedNote(triple.1),
                UnmidiNote(triple.2)
            );
        }
    }

    #[test]
    fn transpose_add_unmidi_note() {
        for triple in [(0, 60, 60), (10, 60, 70), (-55, 60, 5)] {
            assert_eq!(
                Transpose(triple.0) + UnmidiNote(triple.1),
                MidiNote(triple.2 as u8)
            );
        }
    }

    #[test]
    fn transpose_add_unmidi_note_clamped() {
        for triple in [(-61, 60, 0), (100, 200, 127), (200, 0, 127), (-200, 60, 0)] {
            assert_eq!(
                Transpose(triple.0) + UnmidiNote(triple.1),
                MidiNote(triple.2 as u8)
            );
        }
    }

    #[test]
    fn unkeyed_wraps_to_octave() {
        for pair in [(0, 0), (12, 0), (14, 2), (-1, 11), (-14, 10)] {
            assert_eq!(UnkeyedNote(pair.0).wrap_to_octave(), pair.1);
        }
    }

    #[test]
    fn interval_ratio() {
        for triple in [
            (6, 12, 0.5),
            (4, 12, 1.0 / 3.0),
            (7, 12, 7.0 / 12.0),
            (-3, 8, -0.375),
        ] {
            assert_eq!(Interval(triple.0).ratio(Interval(triple.1)), triple.2);
        }
    }
}
