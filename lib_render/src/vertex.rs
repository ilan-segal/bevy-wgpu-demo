use bevy::{
    prelude::*,
    render::{
        mesh::VertexFormat,
        render_resource::{BufferDescriptor, BufferUsages, VertexAttribute},
        renderer::{RenderDevice, RenderQueue},
    },
};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct ModelVertex {
    position: [f32; 3],
    normal: [f32; 3],
    color: [f32; 3],
    uv: [f32; 2],
}

impl ModelVertex {
    pub const fn desc() -> [VertexAttribute; 4] {
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
                format: VertexFormat::Float32x3,
                offset: std::mem::size_of::<[f32; 6]>() as _,
                shader_location: 2,
            },
            VertexAttribute {
                format: VertexFormat::Float32x2,
                offset: std::mem::size_of::<[f32; 9]>() as _,
                shader_location: 3,
            },
        ]
    }
}

pub(crate) const VERTICES: &[ModelVertex] = &[
    ModelVertex {
        position: [-0.5, 0.5, 0.5],
        normal: [0.0, 0.0, -1.0],
        color: [1.0, 1.0, 1.0],
        uv: [0., 0.],
    },
    ModelVertex {
        position: [-0.5, -0.5, 0.5],
        normal: [0.0, 0.0, -1.0],
        color: [1.0, 1.0, 1.0],
        uv: [0., 1.],
    },
    ModelVertex {
        position: [0.5, 0.5, 0.5],
        normal: [0.0, 0.0, -1.0],
        color: [1.0, 1.0, 1.0],
        uv: [1., 0.],
    },
    ModelVertex {
        position: [0.5, -0.5, 0.5],
        normal: [0.0, 0.0, -1.0],
        color: [1.0, 1.0, 1.0],
        uv: [1., 1.],
    },
];

/// Triangle strip of a single rectangle
pub(crate) const INDICES: &[u16] = &[0, 1, 2, 3];

#[derive(Resource)]
pub(crate) struct VertexBuffer {
    pub vertex_buffer: bevy::render::render_resource::Buffer,
    pub num_vertices: u32,
}

pub(crate) struct VertexPlugin;

impl Plugin for VertexPlugin {
    fn build(&self, app: &mut App) {
        app.sub_app_mut(bevy::render::RenderApp).add_systems(
            ExtractSchedule,
            init_vertex_buffer.run_if(not(resource_exists::<VertexBuffer>)),
        );
    }
}

fn init_vertex_buffer(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    let vertex_buffer = render_device.create_buffer(&BufferDescriptor {
        label: Some("triangle vertex buffer"),
        size: (VERTICES.len() * std::mem::size_of::<ModelVertex>()) as u64,
        usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    render_queue.write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(VERTICES));
    commands.insert_resource(VertexBuffer {
        vertex_buffer,
        num_vertices: VERTICES.len() as _,
    });
}
