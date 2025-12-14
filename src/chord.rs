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
        const Add4 = 1 << 9;
        const SwitchMinorMajor = 1 << 10;
        const No3 = 1 << 11;
        const ChangeKey = 1 << 12;
        const Pulse = 1 << 13;
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Chord {
    // Disable name for now, since this will be better as a debugging tool rather than crucial logic
    //name: &'static str,
    root: UnkeyedNote,
    mods: Modifiers,
    mask: PitchClassSet, // bits 0..11
}

type ModifierFn = fn(&mut Chord);
impl Chord {
    // Set of the major chord roots
    const MAJOR_ROOTS: [i16; 3] = [0, 5, 7];
    const MINOR_ROOTS: [i16; 3] = [2, 4, 9];
    const DIMIN_ROOTS: [i16; 1] = [11];

    const MOD_APPLICATIONS: [(Modifiers, ModifierFn); 11] = [
        (Modifiers::MajorTri, |c| c.mask = PitchClassSet::MAJOR_TRI),
        (Modifiers::MinorTri, |c| c.mask = PitchClassSet::MINOR_TRI),
        (Modifiers::DiminTri, |c| c.mask = PitchClassSet::DIMIN_TRI),
        (Modifiers::AddMajor2, |c| c.mask.insert(UnrootedNote(2))),
        (Modifiers::AddMinor7, |c| c.mask.insert(UnrootedNote(10))),
        (Modifiers::AddMajor7, |c| c.mask.insert(UnrootedNote(11))),
        (Modifiers::Minor3ToMajor, |c| {
            c.mask.remove(UnrootedNote(3));
            c.mask.insert(UnrootedNote(4))
        }),
        (Modifiers::RestorePerfect5, |c| {
            c.mask.remove(UnrootedNote(6));
            c.mask.remove(UnrootedNote(8));
            c.mask.insert(UnrootedNote(7))
        }),
        (Modifiers::Add4, |c| c.mask.insert(UnrootedNote(5))),
        (Modifiers::SwitchMinorMajor, |c| {
            if Chord::MINOR_ROOTS.contains(&c.root.wrap_to_octave()) {
                c.mask.remove(UnrootedNote(3));
                c.mask.insert(UnrootedNote(4));
            } else if Chord::DIMIN_ROOTS.contains(&c.root.wrap_to_octave()) {
                c.mask.remove(UnrootedNote(3));
                c.mask.insert(UnrootedNote(4));
            } else {
                c.mask.remove(UnrootedNote(4));
                c.mask.insert(UnrootedNote(3));
            }
        }),
        (Modifiers::No3, |c| {
            c.mask.remove(UnrootedNote(3));
            c.mask.remove(UnrootedNote(4));
        }),
    ];

    pub fn new(rt: UnkeyedNote, mods: Modifiers) -> Self {
        let mut c = Self {
            root: rt,
            mask: PitchClassSet::ROOT_ONLY,
            mods: mods,
        };
        c.regen_mask();
        c
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
        self.regen_mask()
    }

    pub fn set_mods_now(&mut self, mods: Modifiers) {
        self.mods = mods;
        self.regen_mask()
    }

    pub fn get_note_above_root(&self, note: UnkeyedNote) -> UnrootedNote {
        UnrootedNote::new(note - self.root)
    }

    // Crucial to call this immediately after every change to self.mods
    fn regen_mask(&mut self) {
        for (modifier, func) in Self::MOD_APPLICATIONS {
            if self.mods.contains(modifier) {
                func(self);
            }
        }
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
        self.mask.contains(rel)
    }

    fn has_root(&self, note: UnkeyedNote) -> bool {
        note.wrap_to_octave() == self.root.wrap_to_octave()
    }
}
