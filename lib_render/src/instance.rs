use bevy::render::{mesh::VertexFormat, render_resource::VertexAttribute};

// pub struct Instance {
//     pub position: UVec3,
//     pub dimensions: UVec2,
//     pub normal: Normal,
// }

// /**
// All block face data in four bytes:

// - 0-4: Width (0-31, needs 5 bits)
// - 5-9: Height (0-31, needs 5 bits)
// - 10-14: X (in chunk) (0-31, needs 5 bits)
// - 15-19: Y (in chunk) (0-31, needs 5 bits)
// - 20-24: Z (in chunk) (0-31, needs 5 bits)
// - 25-27: Normal ID (0-5, needs 3 bits)
// - 28-29: Ambient occlusion factor (0-3, needs 2 bits)
// - 30-31: ???

// Note that the height and width actually have range 1-32, which naively needs
// 6 bits each, but since the value 0 is not possible, then we can map this onto
// the range 0-31 and get away with using 5 bits.
//  */
// #[repr(C)]
// #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
// pub struct InstanceRaw {
//     data: u32,
// }

// impl From<Instance> for InstanceRaw {
//     fn from(value: Instance) -> Self {
//         let mut data = 0;
//         data |= (value.dimensions.x - 1) << 0;
//         data |= (value.dimensions.y - 1) << 5;
//         data |= value.position.x << 10;
//         data |= value.position.y << 16;
//         data |= value.position.z << 22;
//         data |= (value.normal as u32) << 28;
//         return Self { data };
//     }
// }

pub struct Instance {
    pub texture_index: u32,
    pub normal: crate::Normal,
    pub local_pos: [u8; 3],
    pub chunk_pos: [i32; 3],
    // pub transform: bevy::prelude::Transform,
    /// Column-wise, starting with top right
    pub ambient_occlusion: [u8; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RawInstance {
    /// Bits:
    /// - 0-4: Local x (5 bits, 0-31)
    /// - 5-9: Local y (5 bits, 0-31)
    /// - 10-14: Local z (5 bits, 0-31)
    /// - 15-26: Ambient occlusion factors (3 bits each, 4 values, 0-4)
    /// - 27-29: Normal
    /// - 30-31: Texture index
    data: u32,
    chunk_pos: [f32; 3],
    // transform: [[f32; 4]; 4],
}

impl From<Instance> for RawInstance {
    fn from(value: Instance) -> Self {
        let [a0, a1, a2, a3] = value.ambient_occlusion.map(|x| x as u32);
        let ambient_occlusions = (a0 << 0) | (a1 << 3) | (a2 << 6) | (a3 << 9);
        Self {
            data: ((value.local_pos[0] as u32) << 0)
                | ((value.local_pos[1] as u32) << 5)
                | ((value.local_pos[2] as u32) << 10)
                | (ambient_occlusions << 15)
                | ((value.normal as u32) << 27)
                | (value.texture_index << 30),
            chunk_pos: value.chunk_pos.map(|value| value as _),
        }
    }
}

impl RawInstance {
    pub fn desc() -> [VertexAttribute; 2] {
        [
            VertexAttribute {
                format: VertexFormat::Uint32,
                offset: 0,
                shader_location: 4,
            },
            VertexAttribute {
                format: VertexFormat::Float32x3,
                offset: std::mem::size_of::<[f32; 1]>() as _,
                shader_location: 5,
            },
        ]
    }
}
