use crate::notes::{UnkeyedNote, UnrootedNote};

use bitflags::bitflags;

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct Modifiers: u16 {
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


#[repr(transparent)]
#[derive(Clone, Copy, Eq, PartialEq)]
struct PitchClassSet(u16);

impl PitchClassSet {
    fn contains(&self, pc: UnrootedNote) -> bool {
        self.0 & (1 << pc.0) != 0
    }

    fn insert(&mut self, pc: UnrootedNote) {
        self.0 |= 1 << pc.0;
    }

    fn remove(&mut self, pc: UnrootedNote) {
        self.0 &= !(1 << pc.0);
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Chord {
    // Disable name for now, since this will be better as a debugging tool rather than crucial logic
    //name: &'static str,
    root: UnkeyedNote,
    mask: PitchClassSet, // bits 0..11
    mods: Modifiers,
}

type ModifierFn = fn(&mut Chord);
impl Chord {

    const MOD_APPLICATIONS: [(Modifiers, ModifierFn); 11] = [
        (Modifiers::MajorTri, |c| c.mask = PitchClassSet(0b000010010001)),
        (Modifiers::MinorTri, |c| c.mask = PitchClassSet(0b000010001001)),
        (Modifiers::DiminTri, |c| c.mask = PitchClassSet(0b000001001001)),
        (Modifiers::AddMajor2, |c| c.mask.insert(UnrootedNote(2))),
        (Modifiers::AddMinor7, |c| c.mask.insert(UnrootedNote(2))),
        (Modifiers::AddMajor7, |c| c.mask.insert(UnrootedNote(2))),
        (Modifiers::Minor3ToMajor, |c| c.mask.insert(UnrootedNote(2))),
        (Modifiers::RestorePerfect5, |c| c.mask.insert(UnrootedNote(2))),
        (Modifiers::Add4, |c| c.mask.insert(UnrootedNote(2))),
        (Modifiers::SwitchMinorMajor, |c| c.mask.insert(UnrootedNote(2))),
        (Modifiers::No3, |c| c.mask.insert(UnrootedNote(2))),
    ];

    pub fn new(rt: UnkeyedNote, mods: Modifiers) -> Self {
        let mut c = Self { root: rt, mask: PitchClassSet(1), mods: mods }; // 1 to include the root (0th bit on)
        c.regen_mask();
        c
    }

    pub fn add_mod_now(&mut self, mods: Modifiers) {
        self.mods |= mods;
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

trait ChordExt {
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

//Modifiers need:
// - Iteration of types in detemined order
// - Map from keypress to modifier
// - Map from keypress to button
// - Map from modifier to function
// - Are buttons necessary? Can we just check if the modifier is active?
//      - Issue there is we want to press and release a button, and have it enqueued for the next
//      chord, not active on the current chord.
//      - Ok, then it sounds like we need two independent booleans: mod_active and mod_enqueued
//      - Is mod_active really a thing? When we pop it off the queue it modifies the chord. Then
//      the chord sticks like that.
//      - Maybe having the chord necessarily stick isn't very elegant, like we can't undo a mod if we wanted
//      to. the information about mods that have been applied is no longer available.
//      - Ok, so lets pivot to a (mod_enqueued, mod_active setup) dynamic state for each mod. Is
//      that all the dynamic mod state we need? Obv. there's static info we want to track.
//      - The wider picture of dynamic state is:
//          - Midi
//              - Connection
//          - Window
//              - Window
//              - Surface
//          - Mouse
//              - down
//              - last pos
//          - Transpose
//          - Notes Playing
//          - Chord State
//              root: UnkeyedNote,
//              mask: PitchClassSet, // bits 0..11
//              (Mod state)
//                  - For each mod
//                      - mod_active
//          - mod_enqueued
//
//          Trouble is we can represent impossible states with both mod state and chord mask. mod
//          state is a superset of chord mask, so if we want both, maybe we should only do that
//          one, and compute the mask at the top of each loop? So it's not technically part of the
//          state.
//
//          Recalculating at the top of each loop could still be confusing. Can we make it more
//          impossible to represent undesirable states?
//
//          What if modifying active_mods immediately and necessarily recalculates chord mask?
//
//          mod_active could maybe go inside the Chord struct, but mod_enqueued is not information
//          about a chord.
//
//          With mod_active inside the chord, we have interesting equality decisions, but that's
//          not a dealbreaker.
//
//          By putting it inside, we can control write access to the data and have simple
//          assurances that it's always in a good state.
//
//          Cool, I think that's our first implementation step: add a Modifier: active_mods to the
//          Chord struct, and prohibit outside write access to the mask
//
//
//          ...Hmmm
//
//          Now that I've started, I realize we haven't planned for ChangeKey or Pulse. These
//          really aren't chord modifiers in a pure sense. They can be enqueued and applied
//          though. Maybe enqueued state has both a Modifiers component and a Actions component
//
//          Ooooo, next puzzle. Do we throw on base modifiers (like to form a major triad) at application time? What if those last minute mods overrule the enqueued mods the user pressed? Two options:
//           - Don't use chord-button-caused mods in the chord construction
//              - In that case, the active mods will not, on their own, fully determine the mask. Sad
//              - But perhaps the root (which determines major/minor/dim) along with mods can.
//           - When using chord-button-caused mods, use the lowest priority mods that can always be overruled.
//              - Example: enqueue
//
//           Example:
//
//
//
