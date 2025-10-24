use bevy::math::IVec3;

#[derive(Clone, Copy)]
#[repr(C)]
pub enum Normal {
    PosX,
    NegX,
    PosY,
    NegY,
    PosZ,
    NegZ,
}

impl Normal {
    pub fn as_unit_direction(&self) -> IVec3 {
        match self {
            Self::PosX => IVec3::X,
            Self::NegX => IVec3::NEG_X,
            Self::PosY => IVec3::Y,
            Self::NegY => IVec3::NEG_Y,
            Self::PosZ => IVec3::Z,
            Self::NegZ => IVec3::NEG_Z,
        }
    }
}
