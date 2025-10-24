use bevy::render::{mesh::VertexFormat, render_resource::VertexAttribute};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelVertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
    pub uv: [f32; 2],
}

impl ModelVertex {
    pub const fn desc() -> [VertexAttribute; 3] {
        [
            VertexAttribute {
                format: VertexFormat::Float32x3,
                offset: 0,
                shader_location: 0,
            },
            VertexAttribute {
                format: VertexFormat::Float32x3,
                offset: std::mem::size_of::<[f32; 3]>() as _,
                shader_location: 1,
            },
            VertexAttribute {
                format: VertexFormat::Float32x2,
                offset: std::mem::size_of::<[f32; 6]>() as _,
                shader_location: 2,
            },
        ]
    }
}

pub const VERTICES: &[ModelVertex] = &[
    ModelVertex {
        position: [-0.5, 0.5, 0.5],
        color: [1.0, 1.0, 1.0],
        uv: [0., 0.],
    },
    ModelVertex {
        position: [-0.5, -0.5, 0.5],
        color: [1.0, 1.0, 1.0],
        uv: [0., 1.],
    },
    ModelVertex {
        position: [0.5, 0.5, 0.5],
        color: [1.0, 1.0, 1.0],
        uv: [1., 0.],
    },
    ModelVertex {
        position: [0.5, -0.5, 0.5],
        color: [1.0, 1.0, 1.0],
        uv: [1., 1.],
    },
];

/// Triangle strip of a single rectangle
pub const INDICES: &[u16] = &[0, 1, 2, 3];
