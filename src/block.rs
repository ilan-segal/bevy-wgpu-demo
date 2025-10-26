use strum_macros::EnumIter;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, EnumIter)]
pub enum Block {
    #[default]
    Air,
    Stone,
    Dirt,
}

impl Block {
    pub fn is_transparent(&self) -> bool {
        match self {
            Block::Air => true,
            _ => false,
        }
    }

    pub fn get_texture_index(&self) -> Option<TextureIndex> {
        let result = match self {
            Self::Stone => TextureIndex {
                index: 0,
                asset_path: "stone.png",
            },
            Self::Dirt => TextureIndex {
                index: 1,
                asset_path: "dirt.png",
            },
            _ => return None,
        };
        return Some(result);
    }
}

pub struct TextureIndex {
    pub index: usize,
    pub asset_path: &'static str,
}
