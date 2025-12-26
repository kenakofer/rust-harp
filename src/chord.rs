use crate::notes::{PitchClassSet, UnkeyedNote, UnrootedNote};

use bitflags::bitflags;

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct Modifiers: u16 {
        const MajorTri = 1 << 0;
        const MinorTri = 1 << 1;
        const DiminTri = 1 << 2;
        const AddMajor2 = 1 << 3;
        const AddMajor6 = 1 << 4;
        const AddMinor6 = 1 << 12;
        const AddMinor7 = 1 << 5;
        const AddMajor7 = 1 << 6;
        const Minor3ToMajor = 1 << 7;
        const RestorePerfect5 = 1 << 8;
        const SwitchMinorMajor = 1 << 10;
        const Add4 = 1 << 9;
        const No3 = 1 << 11;
        const Invert = 1 << 12;

        const Sus4 = Modifiers::Add4.bits() | Modifiers::No3.bits();
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Chord {
    // Disable name for now, since this will be better as a debugging tool rather than crucial logic
    //name: &'static str,
    root: UnkeyedNote,
    mods: Modifiers,
}

type ModifierFn = fn(&mut PitchClassSet);
impl Chord {
    // Set of the major chord roots
    const MAJOR_ROOTS: [i16; 3] = [0, 5, 7];
    const MINOR_ROOTS: [i16; 3] = [2, 4, 9];
    const DIMIN_ROOTS: [i16; 1] = [11];

    const ORDERED_MOD_APPLICATIONS: [(Modifiers, ModifierFn); 14] = [
        // Destructive, initializer modifiers, should be first
        (Modifiers::MajorTri, |m| *m = PitchClassSet::MAJOR_TRI),
        (Modifiers::MinorTri, |m| *m = PitchClassSet::MINOR_TRI),
        (Modifiers::DiminTri, |m| *m = PitchClassSet::DIMIN_TRI),
        // Constructive modifiers
        (Modifiers::AddMajor2, |m| m.insert(UnrootedNote(2))),
        (Modifiers::AddMajor6, |m| m.insert(UnrootedNote(9))),
        (Modifiers::AddMinor6, |m| m.insert(UnrootedNote(8))),
        (Modifiers::AddMinor7, |m| m.insert(UnrootedNote(10))),
        (Modifiers::AddMajor7, |m| m.insert(UnrootedNote(11))),
        (Modifiers::Minor3ToMajor, |m| {
            m.remove(UnrootedNote(3));
            m.insert(UnrootedNote(4))
        }),
        (Modifiers::RestorePerfect5, |m| {
            m.remove(UnrootedNote(6));
            m.remove(UnrootedNote(8));
            m.insert(UnrootedNote(7))
        }),
        (Modifiers::Add4, |m| m.insert(UnrootedNote(5))),
        (Modifiers::SwitchMinorMajor, |m| {
            if m.contains(UnrootedNote(4)) {
                //Previously Major -> Minor
                m.remove(UnrootedNote(4));
                m.insert(UnrootedNote(3));
            } else if m.contains(UnrootedNote(6)) {
                //Previously Diminished -> Major
                m.remove(UnrootedNote(6));
                m.remove(UnrootedNote(3));
                m.insert(UnrootedNote(4));
            } else {
                //Probably previously Minor -> Major
                m.remove(UnrootedNote(3));
                m.insert(UnrootedNote(4));
            }
        }),
        (Modifiers::No3, |m| {
            m.remove(UnrootedNote(3));
            m.remove(UnrootedNote(4));
        }),
        (Modifiers::Invert, |m| {
            // Binary ! every note
            for pc in 0..12 {
                let unrooted = UnrootedNote(pc);
                if m.contains(unrooted) {
                    m.remove(unrooted);
                } else {
                    m.insert(unrooted);
                }
            }
        }),
    ];

    pub fn new(rt: UnkeyedNote, mods: Modifiers) -> Self {
        Self {
            root: rt,
            mods: mods,
        }
    }
    pub fn new_triad(rt: UnkeyedNote) -> Self {
        let mods = match rt.wrap_to_octave() {
            x if Chord::MAJOR_ROOTS.contains(&x) => Modifiers::MajorTri,
            x if Chord::MINOR_ROOTS.contains(&x) => Modifiers::MinorTri,
            x if Chord::DIMIN_ROOTS.contains(&x) => Modifiers::DiminTri,
            _ => Modifiers::MajorTri,
        };
        Self::new(rt, mods)
    }

    pub fn get_root(&self) -> UnkeyedNote {
        self.root
    }

    pub fn invert(&self) -> Self {
        if self.mods.contains(Modifiers::Invert) {
            return Self::new(self.root, self.mods & !Modifiers::Invert);
        }
        Self::new(self.root, self.mods | Modifiers::Invert)

    }

    pub fn add_mods_now(&mut self, mods: Modifiers) {
        self.mods |= mods;
    }

    pub fn _set_mods_now(&mut self, mods: Modifiers) {
        self.mods = mods;
    }

    pub fn get_note_above_root(&self, note: UnkeyedNote) -> UnrootedNote {
        UnrootedNote::new(note - self.root)
    }

    // Crucial to call this immediately after every change to self.mods
    fn get_mask(&self) -> PitchClassSet {
        let mut mask = PitchClassSet::ROOT_ONLY;
        for (modifier, func) in Self::ORDERED_MOD_APPLICATIONS {
            if self.mods.contains(modifier) {
                func(&mut mask);
            }
        }
        mask
    }

    pub fn contains(&self, note: UnkeyedNote) -> bool {
        let rel = self.get_note_above_root(note);
        self.get_mask().contains(rel)
    }

    pub fn has_root(&self, note: UnkeyedNote) -> bool {
        note.wrap_to_octave() == self.root.wrap_to_octave()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    trait CheckRoots {
        fn verify_triads(self, exp_mods: Modifiers, exp_pcs: PitchClassSet);
    }

    impl<'a> CheckRoots for std::slice::Iter<'a, i16> {
        fn verify_triads(self, exp_mods: Modifiers, exp_pcs: PitchClassSet) {
            self.map(|&r| Chord::new_triad(UnkeyedNote(r)))
                .for_each(|c| {
                    assert_eq!((c.root, c.mods), (c.root, exp_mods));
                    assert_eq!((c.root, c.get_mask()), (c.root, exp_pcs));
                });
        }
    }

    #[test]
    fn major_triads() {
        const EXP_MODS: Modifiers = Modifiers::MajorTri;
        const EXP_PCS: PitchClassSet = PitchClassSet::MAJOR_TRI;
        assert_eq!(EXP_PCS, PitchClassSet(0b000010010001));

        [-17, -12, -7, -5, 0, 5, 7, 12, 17]
            .iter()
            .verify_triads(EXP_MODS, EXP_PCS);
    }

    #[test]
    fn minor_triads() {
        const EXP_MODS: Modifiers = Modifiers::MinorTri;
        const EXP_PCS: PitchClassSet = PitchClassSet::MINOR_TRI;
        assert_eq!(EXP_PCS, PitchClassSet(0b000010001001));

        [-15, -10, -8, -3, 2, 4, 9, 14]
            .iter()
            .verify_triads(EXP_MODS, EXP_PCS);
    }

    #[test]
    fn diminished_triads() {
        const EXP_MODS: Modifiers = Modifiers::DiminTri;
        const EXP_PCS: PitchClassSet = PitchClassSet::DIMIN_TRI;
        assert_eq!(EXP_PCS, PitchClassSet(0b000001001001));

        [-13, -1, 11, 23].iter().verify_triads(EXP_MODS, EXP_PCS);
    }

    #[test]
    fn leftover_triads_major() {
        const EXP_MODS: Modifiers = Modifiers::MajorTri;
        const EXP_PCS: PitchClassSet = PitchClassSet::MAJOR_TRI;
        assert_eq!(EXP_PCS, PitchClassSet(0b000010010001));

        [-11, -9, -6, -4, -2, 1, 3, 6, 8, 10, 13]
            .iter()
            .verify_triads(EXP_MODS, EXP_PCS);
    }

    #[test]
    fn major_triad_contains() {
        let c = Chord::new_triad(UnkeyedNote(0)); // C major

        assert!(c.contains(UnkeyedNote(0))); // root
        assert!(c.contains(UnkeyedNote(4))); // major third
        assert!(c.contains(UnkeyedNote(7))); // fifth

        assert!(!c.contains(UnkeyedNote(6))); // diminished fifth
        assert!(!c.contains(UnkeyedNote(3))); // minor third
    }

    #[test]
    fn minor_triad_contains() {
        let c = Chord::new_triad(UnkeyedNote(2)); // D minor

        assert!(c.contains(UnkeyedNote(2))); // root
        assert!(c.contains(UnkeyedNote(5))); // minor third
        assert!(c.contains(UnkeyedNote(9))); // fifth

        assert!(!c.contains(UnkeyedNote(6))); // diminished fifth
        assert!(!c.contains(UnkeyedNote(4))); // major third
    }

    #[test]
    fn diminished_triad_contains() {
        let c = Chord::new_triad(UnkeyedNote(11)); // B diminished

        assert!(c.contains(UnkeyedNote(11))); // root
        assert!(c.contains(UnkeyedNote(2))); // minor third
        assert!(c.contains(UnkeyedNote(5))); // diminished fifth

        assert!(!c.contains(UnkeyedNote(6))); // perfect fifth
        assert!(!c.contains(UnkeyedNote(4))); // major third
    }

    #[test]
    fn add_minor7_modifier() {
        let mut c = Chord::new_triad(UnkeyedNote(0));
        c.add_mods_now(Modifiers::AddMinor7);

        assert!(c.contains(UnkeyedNote(10))); // minor 7
    }

    #[test]
    fn sus4_modifier() {
        let mut c = Chord::new_triad(UnkeyedNote(0));
        c.add_mods_now(Modifiers::Sus4);

        assert!(c.contains(UnkeyedNote(5))); // fourth
        assert!(!c.contains(UnkeyedNote(4))); // no major third
        assert!(!c.contains(UnkeyedNote(3))); // no minor third
    }

    #[test]
    fn add_major7_modifier() {
        let mut c = Chord::new_triad(UnkeyedNote(4));
        c.add_mods_now(Modifiers::AddMajor7);

        assert!(c.contains(UnkeyedNote(11))); // major 7
    }

    #[test]
    fn add_major2_modifier() {
        let mut c = Chord::new_triad(UnkeyedNote(4)); // iii
        c.add_mods_now(Modifiers::AddMajor2);

        assert!(c.contains(UnkeyedNote(6))); // major 2
    }

    #[test]
    fn add_major6_modifier() {
        let mut c = Chord::new_triad(UnkeyedNote(4));
        c.add_mods_now(Modifiers::AddMajor6);

        assert!(c.contains(UnkeyedNote(13)));
    }

    #[test]
    fn minor3_to_major_modifier() {
        let mut c = Chord::new_triad(UnkeyedNote(4)); // iii
        c.add_mods_now(Modifiers::Minor3ToMajor);

        assert!(!c.contains(UnkeyedNote(7))); // no minor third
        assert!(c.contains(UnkeyedNote(8))); // major third

        let mut c = Chord::new_triad(UnkeyedNote(0)); // I
        c.add_mods_now(Modifiers::Minor3ToMajor);

        assert!(!c.contains(UnkeyedNote(3))); // no minor third
        assert!(c.contains(UnkeyedNote(4))); // major third
    }
}
