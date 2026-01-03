use crate::notes::{NoteVolume, Transpose, UnmidiNote};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoteOn {
    pub note: UnmidiNote,
    pub volume: NoteVolume,
}

#[derive(Debug)]
pub struct AppEffects {
    pub play_notes: Vec<NoteOn>,
    pub stop_notes: Vec<UnmidiNote>,
    pub redraw: bool,
    pub change_key: Option<Transpose>,
}

impl AppEffects {
    /// Apply note-offs before note-ons so re-triggering the same note doesn't immediately stop it.
    pub fn apply_stop_then_play<Ctx>(
        self,
        ctx: &mut Ctx,
        mut stop: impl FnMut(&mut Ctx, UnmidiNote),
        mut play: impl FnMut(&mut Ctx, NoteOn),
    ) -> bool {
        let played = !self.play_notes.is_empty();
        for un in self.stop_notes {
            stop(ctx, un);
        }
        for pn in self.play_notes {
            play(ctx, pn);
        }
        played
    }
}
