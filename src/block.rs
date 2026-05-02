use lib_render::Normal;
use strum_macros::EnumIter;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, EnumIter)]
pub enum Block {
    #[default]
    Air,
    Stone,
    Dirt,
    Grass,
    Bedrock,
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
    Bedrock,
    GrassTop,
    GrassSide,
}

impl lib_render::texture::TextureIndex for Terrain {
    fn get_name(&self) -> &'static str {
        match self {
            Self::Stone => "stone",
            Self::Dirt => "dirt",
            Self::Bedrock => "bedrock",
            Self::GrassTop => "grass",
            Self::GrassSide => "grass_side",
        }
    }
}

impl TryFrom<(Block, Normal)> for Terrain {
    type Error = &'static str;
    fn try_from(value: (Block, Normal)) -> Result<Self, Self::Error> {
        match value {
            (Block::Air, _) => Err("Air is not terrain"),
            (Block::Dirt, _) => Ok(Self::Dirt),
            (Block::Stone, _) => Ok(Self::Stone),
            (Block::Bedrock, _) => Ok(Self::Bedrock),
            (Block::Grass, Normal::PosY) => Ok(Self::GrassTop),
            (Block::Grass, Normal::NegY) => Ok(Self::Dirt),
            (Block::Grass, _) => Ok(Self::GrassSide),
        }
    }
}
