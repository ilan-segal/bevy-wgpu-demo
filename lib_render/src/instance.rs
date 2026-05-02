use bevy::render::{mesh::VertexFormat, render_resource::VertexAttribute};

pub struct Instance {
    pub texture_index: u32,
    pub normal: crate::Normal,
    pub local_pos: [u8; 3],
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
    data: u32,
    material_index: u32,
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
                | ((value.normal as u32) << 27),
            material_index: value.texture_index,
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
                format: VertexFormat::Uint32,
                offset: std::mem::size_of::<u32>() as _,
                shader_location: 5,
            },
        ]
    }
}
