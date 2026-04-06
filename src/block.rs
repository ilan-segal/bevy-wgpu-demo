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
}

#[derive(EnumIter, Clone)]
pub enum Terrain {
    Stone,
    Dirt,
}

impl lib_render::texture::TextureIndex for Terrain {
    fn get_name(&self) -> &'static str {
        match self {
            Self::Stone => "stone",
            Self::Dirt => "dirt",
        }
    }
}

impl TryFrom<Block> for Terrain {
    type Error = &'static str;
    fn try_from(value: Block) -> Result<Self, Self::Error> {
        match value {
            Block::Air => Err("Air is not terrain"),
            Block::Dirt => Ok(Self::Dirt),
            Block::Stone => Ok(Self::Stone),
        }
    }
}
