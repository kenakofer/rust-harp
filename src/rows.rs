#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum RowId {
    Top,
    Middle,
    Bottom,
}

impl RowId {
    pub fn from_y_norm(y: f32) -> Self {
        // 40% top, 40% middle, 20% bottom
        if y < 0.4 {
            RowId::Top
        } else if y < 0.8 {
            RowId::Middle
        } else {
            RowId::Bottom
        }
    }

    pub fn index(self) -> usize {
        match self {
            RowId::Top => 0,
            RowId::Middle => 1,
            RowId::Bottom => 2,
        }
    }
}
