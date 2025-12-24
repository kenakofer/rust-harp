use crate::app_state::ChordButton;
use crate::chord::Modifiers;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WheelDir8 {
    N,
    NE,
    E,
    SE,
    S,
    SW,
    W,
    NW,
}

impl WheelDir8 {
    pub fn from_i32(dir: i32) -> Option<Self> {
        match dir {
            0 => Some(Self::N),
            1 => Some(Self::NE),
            2 => Some(Self::E),
            3 => Some(Self::SE),
            4 => Some(Self::S),
            5 => Some(Self::SW),
            6 => Some(Self::W),
            7 => Some(Self::NW),
            _ => None,
        }
    }

    pub fn index(self) -> usize {
        match self {
            Self::N => 0,
            Self::NE => 1,
            Self::E => 2,
            Self::SE => 3,
            Self::S => 4,
            Self::SW => 5,
            Self::W => 6,
            Self::NW => 7,
        }
    }
}

fn is_major_degree(button: ChordButton) -> bool {
    use ChordButton::*;
    matches!(button, VIIB | IV | I | V)
}

/// Map a chord button + wheel direction to a modifier preset.
///
/// This is the core mapping used by the Android swipe-wheel UI.
pub fn modifiers_for(button: ChordButton, dir: WheelDir8) -> Modifiers {
    // Ordering: from top, clockwise.
    // See TODO.md for the exact spec.
    if is_major_degree(button) {
        match dir {
            WheelDir8::N => Modifiers::AddMinor7, // ^7
            WheelDir8::NE => Modifiers::AddMinor7 | Modifiers::AddMajor2, // ^9
            WheelDir8::E => Modifiers::AddMajor2, // +2
            WheelDir8::SE => {
                Modifiers::SwitchMinorMajor | Modifiers::AddMinor7 | Modifiers::AddMajor2
            } // iv^9
            WheelDir8::S => Modifiers::SwitchMinorMajor | Modifiers::AddMinor7, // iv^7
            WheelDir8::SW => Modifiers::SwitchMinorMajor | Modifiers::AddMajor7, // iv^M7
            WheelDir8::W => Modifiers::Sus4, // sus
            WheelDir8::NW => Modifiers::AddMajor7, // ^M7
        }
    } else {
        match dir {
            WheelDir8::N => Modifiers::SwitchMinorMajor | Modifiers::AddMinor7, // III^7
            WheelDir8::NE => {
                Modifiers::SwitchMinorMajor | Modifiers::AddMinor7 | Modifiers::AddMajor2
            } // III^9
            WheelDir8::E => Modifiers::AddMajor2, // iii+2
            WheelDir8::SE => Modifiers::AddMinor7 | Modifiers::AddMajor2, // iii^9
            WheelDir8::S => Modifiers::AddMinor7, // iii^7
            WheelDir8::SW => Modifiers::AddMajor7, // iii^M7
            WheelDir8::W => Modifiers::Sus4, // iiisus
            WheelDir8::NW => Modifiers::SwitchMinorMajor | Modifiers::AddMajor7, // III^M7
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn major_iv_mapping_examples() {
        assert_eq!(
            modifiers_for(ChordButton::IV, WheelDir8::N),
            Modifiers::AddMinor7
        );
        assert_eq!(
            modifiers_for(ChordButton::IV, WheelDir8::NW),
            Modifiers::AddMajor7
        );
        assert_eq!(
            modifiers_for(ChordButton::IV, WheelDir8::W),
            Modifiers::Sus4
        );
    }

    #[test]
    fn minor_iii_mapping_examples() {
        assert_eq!(
            modifiers_for(ChordButton::III, WheelDir8::S),
            Modifiers::AddMinor7
        );
        assert_eq!(
            modifiers_for(ChordButton::III, WheelDir8::N),
            Modifiers::SwitchMinorMajor | Modifiers::AddMinor7
        );
    }
}
