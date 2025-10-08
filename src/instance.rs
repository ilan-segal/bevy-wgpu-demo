use crate::normal::Normal;
use bevy::{
    math::{Mat4, Quat, UVec2, UVec3, Vec3},
    render::{mesh::VertexFormat, render_resource::VertexAttribute},
};

pub struct Instance {
    pub position: UVec3,
    pub dimensions: UVec2,
    pub normal: Normal,
}

// pub const CUBE_FACE_INSTANCES: &[Instance; 6] = &[
//     Instance {
//         position: UVec3::ZERO,
//         dimensions: UVec2::ONE,
//         normal: Normal::PosX,
//     },
//     Instance {
//         position: UVec3::ZERO,
//         dimensions: UVec2::ONE,
//         normal: Normal::NegX,
//     },
//     Instance {
//         position: UVec3::ZERO,
//         dimensions: UVec2::ONE,
//         normal: Normal::PosY,
//     },
//     Instance {
//         position: UVec3::ZERO,
//         dimensions: UVec2::ONE,
//         normal: Normal::NegY,
//     },
//     Instance {
//         position: UVec3::ZERO,
//         dimensions: UVec2::ONE,
//         normal: Normal::PosZ,
//     },
//     Instance {
//         position: UVec3::ZERO,
//         dimensions: UVec2::ONE,
//         normal: Normal::NegZ,
//     },
// ];

/**
All block face data in four bytes:

- 0-4: Width (0-31, needs 5 bits)
- 5-9: Height (0-31, needs 5 bits)
- 10-14: X (in chunk) (0-31, needs 5 bits)
- 15-19: Y (in chunk) (0-31, needs 5 bits)
- 20-24: Z (in chunk) (0-31, needs 5 bits)
- 25-27: Normal ID (0-5, needs 3 bits)
- 28-29: Ambient occlusion factor (0-3, needs 2 bits)
- 30-31: ???

Note that the height and width actually have range 1-32, which naively needs
6 bits each, but since the value 0 is not possible, then we can map this onto
the range 0-31 and get away with using 5 bits.
 */
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceRaw {
    data: u32,
}

impl From<Instance> for InstanceRaw {
    fn from(value: Instance) -> Self {
        let mut data = 0;
        data |= (value.dimensions.x - 1) << 0;
        data |= (value.dimensions.y - 1) << 5;
        data |= value.position.x << 10;
        data |= value.position.y << 16;
        data |= value.position.z << 22;
        data |= (value.normal as u32) << 28;
        return Self { data };
    }
}

pub struct DetailedInstance {
    pub translation: Vec3,
    pub rotation: Quat,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct DetailedInstanceRaw {
    matrix_cols: [[f32; 4]; 4],
}

impl From<DetailedInstance> for DetailedInstanceRaw {
    fn from(value: DetailedInstance) -> Self {
        let matrix = Mat4::from_translation(value.translation) * Mat4::from_quat(value.rotation);
        let matrix_cols = matrix.to_cols_array_2d();
        Self { matrix_cols }
    }
}

impl DetailedInstanceRaw {
    pub fn desc() -> [VertexAttribute; 4] {
        [
            VertexAttribute {
                format: VertexFormat::Float32x4,
                offset: 0,
                shader_location: 3,
            },
            VertexAttribute {
                format: VertexFormat::Float32x4,
                offset: std::mem::size_of::<[f32; 4]>() as _,
                shader_location: 4,
            },
            VertexAttribute {
                format: VertexFormat::Float32x4,
                offset: std::mem::size_of::<[f32; 8]>() as _,
                shader_location: 5,
            },
            VertexAttribute {
                format: VertexFormat::Float32x4,
                offset: std::mem::size_of::<[f32; 12]>() as _,
                shader_location: 6,
            },
        ]
    }
}
