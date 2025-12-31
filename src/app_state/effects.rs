use crate::notes::{NoteVolume, Transpose, UnmidiNote};
use crate::touch::PointerId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoteOn {
    pub note: UnmidiNote,
    pub volume: NoteVolume,
    pub pointer: Option<PointerId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BendNote {
    pub pointer: PointerId,
    pub target: UnmidiNote,
}

#[derive(Debug)]
pub struct AppEffects {
    pub play_notes: Vec<NoteOn>,
    pub stop_notes: Vec<UnmidiNote>,
    pub bend_notes: Vec<BendNote>,
    pub stop_bend_pointers: Vec<PointerId>,
    pub redraw: bool,
    pub change_key: Option<Transpose>,
}

impl AppEffects {
    /// Apply note-offs before note-ons so re-triggering the same note doesn't immediately stop it.
    pub fn apply_all<Ctx>(
        self,
        ctx: &mut Ctx,
        mut stop: impl FnMut(&mut Ctx, UnmidiNote),
        mut stop_pointer: impl FnMut(&mut Ctx, PointerId),
        mut play: impl FnMut(&mut Ctx, NoteOn),
        mut bend: impl FnMut(&mut Ctx, BendNote),
    ) -> bool {
        let played = !self.play_notes.is_empty() || !self.bend_notes.is_empty();
        for un in self.stop_notes {
            stop(ctx, un);
        }
        for pid in self.stop_bend_pointers {
            stop_pointer(ctx, pid);
        }
        for pn in self.play_notes {
            play(ctx, pn);
        }
        for bn in self.bend_notes {
            bend(ctx, bn);
        }
        played
    }
}
