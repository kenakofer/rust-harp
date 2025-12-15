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
        const AddMinor7 = 1 << 5;
        const AddMajor7 = 1 << 6;
        const Minor3ToMajor = 1 << 7;
        const RestorePerfect5 = 1 << 8;
        const SwitchMinorMajor = 1 << 10;
        const Add4 = 1 << 9;
        const No3 = 1 << 11;

        const Sus4 = Modifiers::Add4.bits() | Modifiers::No3.bits();
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
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

    const ORDERED_MOD_APPLICATIONS: [(Modifiers, ModifierFn); 12] = [
        // Destructive, initializer modifiers, should be first
        (Modifiers::MajorTri, |m| *m = PitchClassSet::MAJOR_TRI),
        (Modifiers::MinorTri, |m| *m = PitchClassSet::MINOR_TRI),
        (Modifiers::DiminTri, |m| *m = PitchClassSet::DIMIN_TRI),

        // Constructive modifiers
        (Modifiers::AddMajor2, |m| m.insert(UnrootedNote(2))),
        (Modifiers::AddMajor6, |m| m.insert(UnrootedNote(9))),
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
            if m.contains(UnrootedNote(4)) { //Previously Major -> Minor
                m.remove(UnrootedNote(4));
                m.insert(UnrootedNote(3));
            } else if m.contains(UnrootedNote(6)) { //Previously Diminished -> Major
                m.remove(UnrootedNote(6));
                m.remove(UnrootedNote(3));
                m.insert(UnrootedNote(4));
            } else { //Probably previously Minor -> Major
                m.remove(UnrootedNote(3));
                m.insert(UnrootedNote(4));
            }
        }),
        (Modifiers::No3, |m| {
            m.remove(UnrootedNote(3));
            m.remove(UnrootedNote(4));
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
    fn get_mask(&self) -> PitchClassSet{
        let mut mask = PitchClassSet::ROOT_ONLY;
        for (modifier, func) in Self::ORDERED_MOD_APPLICATIONS {
            if self.mods.contains(modifier) {
                func(&mut mask);
            }
        }
        mask
    }
}

pub trait ChordExt {
    fn contains(&self, note: UnkeyedNote) -> bool;
    fn has_root(&self, note: UnkeyedNote) -> bool;
}

impl ChordExt for Option<Chord> {
    fn contains(&self, note: UnkeyedNote) -> bool {
        match self {
            Some(chord) => chord.contains(note),
            None => true,
        }
    }

    fn has_root(&self, note: UnkeyedNote) -> bool {
        self.as_ref().is_some_and(|chord| chord.has_root(note))
    }
}
impl ChordExt for Chord {
    fn contains(&self, note: UnkeyedNote) -> bool {
        let rel = self.get_note_above_root(note);
        self.get_mask().contains(rel)
    }

    fn has_root(&self, note: UnkeyedNote) -> bool {
        note.wrap_to_octave() == self.root.wrap_to_octave()
    }
}
