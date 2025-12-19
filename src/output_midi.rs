use crate::notes::{MidiNote, NoteVolume};

pub const BASS_VELOCITY_MULTIPLIER: f32 = 1.0;
pub const MAIN_BASS_BOTTOM: MidiNote = MidiNote(35);
pub const MAIN_BASS_TOP: MidiNote = MidiNote(80);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MidiVelocityPair {
    pub main: u8,
    pub bass: u8,
}

impl MidiVelocityPair {
    pub fn from_note_and_volume(midi_note: MidiNote, volume: NoteVolume) -> Self {
        let base = volume.0;
        if base == 0 {
            return Self { main: 0, bass: 0 };
        }

        let main_factor = (midi_note - MAIN_BASS_BOTTOM)
            .ratio(MAIN_BASS_TOP - MAIN_BASS_BOTTOM)
            .clamp(0.0, 1.0);
        let bass_factor = 1.0 - main_factor;

        // Give main_factor twice as long of a fade
        let main_factor = 1.0 - 0.5 * (1.0 - main_factor);

        let mut main = ((base as f32) * main_factor).round() as u8;
        let mut bass = ((base as f32) * bass_factor * BASS_VELOCITY_MULTIPLIER).round() as u8;

        if main > 0 {
            main = main.clamp(1, 127);
        }
        if bass > 0 {
            bass = bass.clamp(1, 127);
        }

        Self { main, bass }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn midi_velocity_pair_zero_volume_mutes_both_channels() {
        let p = MidiVelocityPair::from_note_and_volume(MidiNote(60), NoteVolume(0));
        assert_eq!(p.main, 0);
        assert_eq!(p.bass, 0);
    }

    #[test]
    fn midi_velocity_pair_clamps_low_and_high_note_ranges() {
        let base = NoteVolume(100);

        // Below MAIN_BASS_BOTTOM: bass dominates, main is at half (per fade curve)
        let low = MidiVelocityPair::from_note_and_volume(MidiNote(0), base);
        assert_eq!(low.main, 50);
        assert_eq!(low.bass, 100);

        // Above MAIN_BASS_TOP: main dominates, bass off
        let high = MidiVelocityPair::from_note_and_volume(MidiNote(127), base);
        assert_eq!(high.main, 100);
        assert_eq!(high.bass, 0);
    }
}
