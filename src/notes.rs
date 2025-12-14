use std::ops::{Add, Sub};

#[repr(transparent)]
#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq, PartialOrd)]
pub struct MidiNote(pub u8);

impl Sub for MidiNote {
    type Output = Interval;
    fn sub(self, rhs: MidiNote) -> Interval {
        Interval(self.0 as i16 - rhs.0 as i16)
    }
}

#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct UnbottomedNote(i16); // Note before building on the BOTTOM_NOTE

#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Transpose(pub i16); // Basically an interval
                       //
impl Transpose {
    pub fn center_octave(self) -> Transpose {
        if self.0 > 6 {
            Transpose(self.0 - 12)
        } else {
            Transpose(self.0)
        }
    }
}

impl Add<UnkeyedNote> for Transpose {
    type Output = UnbottomedNote;
    fn add(self, rhs: UnkeyedNote) -> UnbottomedNote {
        let sum: i16 = (self.0 as i16) + (rhs.0 as i16);
        UnbottomedNote(sum)
    }
}

impl Add<UnbottomedNote> for Transpose {
    type Output = MidiNote;
    fn add(self, rhs: UnbottomedNote) -> MidiNote {
        let sum: i16 = (self.0 as i16) + (rhs.0 as i16);
        return MidiNote(sum.clamp(0, 127) as u8);
    }
}

impl Sub<Transpose> for UnbottomedNote {
    type Output = UnkeyedNote;
    fn sub(self, rhs: Transpose) -> UnkeyedNote {
        let diff: i16 = (self.0 as i16) - (rhs.0 as i16);
        UnkeyedNote(diff)
    }
}

impl Sub<Transpose> for MidiNote {
    type Output = UnbottomedNote;
    fn sub(self, rhs: Transpose) -> UnbottomedNote {
        let diff: i16 = (self.0 as i16) - rhs.0;
        UnbottomedNote(diff)
    }
}

// Note before transposing into the key or building on the BOTTOM_NOTE.
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
pub struct PitchClassSet(u16);

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

