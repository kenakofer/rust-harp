#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum RowId {
    Top,
    Bottom,
}

impl RowId {
    pub fn from_y_norm(y: f32) -> Self {
        if y < 0.5 {
            RowId::Top
        } else {
            RowId::Bottom
        }
    }

    pub fn index(self) -> usize {
        match self {
            RowId::Top => 0,
            RowId::Bottom => 1,
        }
    }
}
