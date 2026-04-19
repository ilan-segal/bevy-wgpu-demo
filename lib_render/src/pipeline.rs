use std::marker::PhantomData;

use bevy::{
    prelude::*,
    render::{
        Extract,
        render_resource::{
            AddressMode, BindGroup, BindGroupEntry, BindGroupLayout, BindGroupLayoutEntry,
            BindingResource, BindingType, BlendState, Buffer, BufferBindingType, BufferDescriptor,
            BufferUsages, FilterMode, RenderPipeline, ShaderStages, TextureFormat, TextureUsages,
            TextureView,
        },
        renderer::RenderDevice,
    },
};

use crate::{
    globals::GlobalsData,
    instance::RawInstance,
    texture::TextureBindGroup,
    vertex::{INDICES, ModelVertex},
};

pub(crate) trait PipelineType {}

pub(crate) struct ShadowPass;

impl PipelineType for ShadowPass {}

pub(crate) struct OpaquePass;

impl PipelineType for OpaquePass {}

pub(crate) struct TranslucentPass;

impl PipelineType for TranslucentPass {}

#[derive(Resource)]
pub(crate) struct MyRenderPipeline<Type: PipelineType> {
    pub(crate) pipeline: RenderPipeline,
    _phantom: PhantomData<Type>,
}

impl<T: PipelineType> MyRenderPipeline<T> {
    fn new(pipeline: RenderPipeline) -> Self {
        Self {
            pipeline,
            _phantom: PhantomData::<T>,
        }
    }
}

#[derive(Resource)]
pub(crate) struct GlobalsUniformBuffer {
    pub buffer: Buffer,
}

#[derive(Resource)]
pub(crate) struct GlobalsUniformBindGroup {
    pub bind_group: BindGroup,
}

#[derive(Resource)]
pub(crate) struct ShadowPassGlobalsUniformBuffer {
    pub buffer: Buffer,
}

#[derive(Resource)]
pub(crate) struct ShadowPassGlobalsUniformBindGroup {
    pub bind_group: BindGroup,
}

#[derive(Resource)]
pub(crate) struct IndexBuffer {
    pub buffer: Buffer,
    pub num_indices: u32,
}

pub(crate) struct DepthTexture {
    pub view: TextureView,
    pub format: TextureFormat,
    #[allow(unused)]
    pub size: UVec2,
}

#[derive(Resource)]
pub(crate) struct MainPassDepth(pub DepthTexture);

#[derive(Resource)]
pub(crate) struct ShadowPassDepth(pub DepthTexture);

#[derive(Resource)]
pub(crate) struct ShadowMapTextureBindGroup {
    pub bind_group: BindGroup,
    #[allow(unused)]
    pub layout: BindGroupLayout,
}

pub(crate) fn init_pipeline(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    windows: Extract<Query<&Window>>,
    texture_bind_group: Option<Res<TextureBindGroup>>,
) {
    let Some(texture_bind_group) = texture_bind_group else {
        return;
    };

    let window = windows.single().expect("Main window");
    let depth_texture = create_depth_texture(
        "depth texture",
        &render_device,
        window.physical_width(),
        window.physical_height(),
    );
    const SHADOW_MAP_SIZE: u32 = 4096;
    let shadow_map = create_depth_texture(
        "shadow map",
        &render_device,
        SHADOW_MAP_SIZE,
        SHADOW_MAP_SIZE,
    );

    let globals_bind_group_layout = render_device.create_bind_group_layout(
        Some("Globals bind group layout"),
        &[BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::VERTEX_FRAGMENT,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    );

    let globals_buffer = render_device.create_buffer(&BufferDescriptor {
        label: Some("globals buffer"),
        size: std::mem::size_of::<GlobalsData>() as u64,
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let globals_bind_group = render_device.create_bind_group(
        Some("Globals bind group"),
        &globals_bind_group_layout,
        &[BindGroupEntry {
            binding: 0,
            resource: globals_buffer.as_entire_binding(),
        }],
    );

    let shadow_pass_globals_buffer = render_device.create_buffer(&BufferDescriptor {
        label: Some("globals buffer"),
        size: std::mem::size_of::<GlobalsData>() as u64,
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let shadow_pass_globals_bind_group = render_device.create_bind_group(
        Some("Shadow pass globals bind group"),
        &globals_bind_group_layout,
        &[BindGroupEntry {
            binding: 0,
            resource: shadow_pass_globals_buffer.as_entire_binding(),
        }],
    );

    commands.insert_resource(GlobalsUniformBuffer {
        buffer: globals_buffer,
    });
    commands.insert_resource(GlobalsUniformBindGroup {
        bind_group: globals_bind_group,
    });

    commands.insert_resource(ShadowPassGlobalsUniformBuffer {
        buffer: shadow_pass_globals_buffer,
    });
    commands.insert_resource(ShadowPassGlobalsUniformBindGroup {
        bind_group: shadow_pass_globals_bind_group,
    });

    let shader = render_device.create_and_validate_shader_module(
        bevy::render::render_resource::ShaderModuleDescriptor {
            label: Some("triangle shader"),
            source: bevy::render::render_resource::ShaderSource::Wgsl(
                include_str!("shaders/triangle.wgsl").into(),
            ),
        },
    );

    let vertex_layout = bevy::render::render_resource::RawVertexBufferLayout {
        array_stride: std::mem::size_of::<ModelVertex>() as _,
        step_mode: bevy::render::render_resource::VertexStepMode::Vertex,
        attributes: &ModelVertex::desc(),
    };

    let instance_layout = bevy::render::render_resource::RawVertexBufferLayout {
        array_stride: std::mem::size_of::<RawInstance>() as _,
        step_mode: bevy::render::render_resource::VertexStepMode::Instance,
        attributes: &RawInstance::desc(),
    };

    let index_buffer = render_device.create_buffer_with_data(
        &bevy::render::render_resource::BufferInitDescriptor {
            label: Some("Index buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: BufferUsages::INDEX,
        },
    );
    let num_indices = INDICES.len() as u32;
    commands.insert_resource(IndexBuffer {
        buffer: index_buffer,
        num_indices,
    });

    let shadow_pipeline_layout = render_device.create_pipeline_layout(
        &bevy::render::render_resource::PipelineLayoutDescriptor {
            label: Some("shadow pipeline layout"),
            bind_group_layouts: &[&globals_bind_group_layout],
            push_constant_ranges: &[bevy::render::render_resource::PushConstantRange {
                stages: ShaderStages::VERTEX,
                range: 0..12,
            }],
        },
    );

    let shadow_pass_pipeline = render_device.create_render_pipeline(
        &bevy::render::render_resource::RawRenderPipelineDescriptor {
            label: Some("shadow pipeline"),
            layout: Some(&shadow_pipeline_layout),
            vertex: bevy::render::render_resource::RawVertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[vertex_layout.clone(), instance_layout.clone()],
                compilation_options: default(),
            },
            fragment: None,
            primitive: bevy::render::render_resource::PrimitiveState {
                topology: bevy::render::mesh::PrimitiveTopology::TriangleStrip,
                cull_mode: Some(bevy::render::render_resource::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(bevy::render::render_resource::DepthStencilState {
                format: shadow_map.format,
                depth_write_enabled: true,
                depth_compare: bevy::render::render_resource::CompareFunction::Greater,
                stencil: bevy::render::render_resource::StencilState::default(),
                bias: bevy::render::render_resource::DepthBiasState::default(),
            }),
            multisample: default(),
            multiview: None,
            cache: None,
        },
    );

    let shadow_map_sampler =
        render_device.create_sampler(&bevy::render::render_resource::SamplerDescriptor {
            label: Some("shadow map sampler"),
            compare: Some(bevy::render::render_resource::CompareFunction::Greater),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Nearest,
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            ..Default::default()
        });
    let shadow_map_bind_group_layout = render_device.create_bind_group_layout(
        Some("shadow map bind group layout"),
        &[
            // Texture binding
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: bevy::render::render_resource::TextureSampleType::Depth,
                    view_dimension: bevy::render::render_resource::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            // Sampler binding
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Sampler(
                    bevy::render::render_resource::SamplerBindingType::Comparison,
                ),
                count: None,
            },
        ],
    );
    let shadow_map_bind_group = render_device.create_bind_group(
        Some("My texture bind group"),
        &shadow_map_bind_group_layout,
        &[
            BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(&shadow_map.view),
            },
            BindGroupEntry {
                binding: 1,
                resource: BindingResource::Sampler(&shadow_map_sampler),
            },
        ],
    );

    let layout = render_device.create_pipeline_layout(
        &bevy::render::render_resource::PipelineLayoutDescriptor {
            label: Some("main pipeline layout"),
            bind_group_layouts: &[
                &globals_bind_group_layout,
                &texture_bind_group.layout,
                &shadow_map_bind_group_layout,
            ],
            push_constant_ranges: &[bevy::render::render_resource::PushConstantRange {
                stages: ShaderStages::VERTEX,
                range: 0..12,
            }],
        },
    );

    let opaque_pipeline = render_device.create_render_pipeline(
        &bevy::render::render_resource::RawRenderPipelineDescriptor {
            label: Some("opaque pipeline"),
            layout: Some(&layout),
            vertex: bevy::render::render_resource::RawVertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[vertex_layout.clone(), instance_layout.clone()],
                compilation_options: default(),
            },
            fragment: Some(bevy::render::render_resource::RawFragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(bevy::render::render_resource::ColorTargetState {
                    format: TextureFormat::bevy_default(),
                    blend: None,
                    write_mask: bevy::render::render_resource::ColorWrites::ALL,
                })],
                compilation_options: default(),
            }),
            primitive: bevy::render::render_resource::PrimitiveState {
                topology: bevy::render::mesh::PrimitiveTopology::TriangleStrip,
                cull_mode: Some(bevy::render::render_resource::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(bevy::render::render_resource::DepthStencilState {
                format: depth_texture.format,
                depth_write_enabled: true,
                depth_compare: bevy::render::render_resource::CompareFunction::Greater,
                stencil: bevy::render::render_resource::StencilState::default(),
                bias: bevy::render::render_resource::DepthBiasState::default(),
            }),
            multisample: default(),
            multiview: None,
            cache: None,
        },
    );

    let translucent_pipeline = render_device.create_render_pipeline(
        &bevy::render::render_resource::RawRenderPipelineDescriptor {
            label: Some("translucent pipeline"),
            layout: Some(&layout),
            vertex: bevy::render::render_resource::RawVertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[vertex_layout.clone(), instance_layout.clone()],
                compilation_options: default(),
            },
            fragment: Some(bevy::render::render_resource::RawFragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(bevy::render::render_resource::ColorTargetState {
                    format: TextureFormat::bevy_default(),
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: bevy::render::render_resource::ColorWrites::ALL,
                })],
                compilation_options: default(),
            }),
            primitive: bevy::render::render_resource::PrimitiveState {
                topology: bevy::render::mesh::PrimitiveTopology::TriangleStrip,
                cull_mode: Some(bevy::render::render_resource::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(bevy::render::render_resource::DepthStencilState {
                format: depth_texture.format,
                depth_write_enabled: false,
                depth_compare: bevy::render::render_resource::CompareFunction::Greater,
                stencil: bevy::render::render_resource::StencilState::default(),
                bias: bevy::render::render_resource::DepthBiasState::default(),
            }),
            multisample: default(),
            multiview: None,
            cache: None,
        },
    );

    commands.insert_resource(MainPassDepth(depth_texture));
    commands.insert_resource(ShadowPassDepth(shadow_map));
    commands.insert_resource(ShadowMapTextureBindGroup {
        bind_group: shadow_map_bind_group,
        layout: shadow_map_bind_group_layout,
    });
    commands.insert_resource(MyRenderPipeline::<ShadowPass>::new(shadow_pass_pipeline));
    commands.insert_resource(MyRenderPipeline::<OpaquePass>::new(opaque_pipeline));
    commands.insert_resource(MyRenderPipeline::<TranslucentPass>::new(
        translucent_pipeline,
    ));
}

pub(crate) fn resize_depth_texture(
    mut resize_events: Extract<EventReader<bevy::window::WindowResized>>,
    mut depth: Option<ResMut<MainPassDepth>>,
    render_device: Res<RenderDevice>,
) {
    let Some(ref mut depth) = depth else {
        return;
    };
    for event in resize_events.read() {
        let width = event.width as u32;
        let height = event.height as u32;
        depth.0 = create_depth_texture("depth texture", &render_device, width, height);
    }
}

pub(crate) fn create_depth_texture(
    name: &'static str,
    device: &RenderDevice,
    width: u32,
    height: u32,
) -> DepthTexture {
    let format = TextureFormat::Depth32Float;
    let size = bevy::render::render_resource::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };

    let desc = bevy::render::render_resource::TextureDescriptor {
        label: Some(name),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: bevy::render::render_resource::TextureDimension::D2,
        format,
        usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    };

    let texture = device.create_texture(&desc);
    let view =
        texture.create_view(&bevy::render::render_resource::TextureViewDescriptor::default());

    DepthTexture {
        view,
        format,
        size: UVec2::new(width, height),
    }
}
