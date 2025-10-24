#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Block {
    #[default]
    Air,
    Stone,
}

impl Block {
    pub fn is_transparent(&self) -> bool {
        match self {
            Block::Air => true,
            Block::Stone => false,
        }
    }
}
