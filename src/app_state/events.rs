use crate::chord::Modifiers;
use crate::notes::{NoteVolume, UnkeyedNote};

use bitflags::bitflags;

pub const DEFAULT_STRUM_VOLUME: NoteVolume = NoteVolume(70);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyState {
    Pressed,
    Released,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyEvent {
    Chord {
        state: KeyState,
        button: ChordButton,
    },
    Modifier {
        state: KeyState,
        button: ModButton,
        modifiers: Modifiers,
    },
    Action {
        state: KeyState,
        button: ActionButton,
        action: Actions,
    },
    StrumCrossing {
        row: crate::rows::RowId,
        note: UnkeyedNote,
        /// Touch/strum intensity snapshot at note-on.
        volume: NoteVolume,
    },
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ChordButton {
    VIIB,
    IV,
    I,
    V,
    II,
    VI,
    III,
    VII,
    HeptatonicMajor,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ModButton {
    Major2,
    Minor7,
    Major7,
    Sus4,
    MinorMajor,
    No3,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ActionButton {
    ChangeKey,
    Pulse,
}

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct Actions: u16 {
        const Pulse = 1 << 0;
        const ChangeKey = 1 << 1;
    }
}
