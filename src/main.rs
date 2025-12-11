use std::{ops::Deref, time::Instant};

use bevy::{
    core_pipeline::core_3d::graph::{Core3d, Node3d},
    ecs::query::{QueryData, QuerySingleError},
    platform::collections::HashMap,
    prelude::*,
    render::{
        Extract, RenderApp,
        camera::{CameraProjection, ExtractedCamera},
        mesh::PrimitiveTopology,
        render_asset::RenderAssets,
        render_graph::{RenderGraphApp, RenderGraphContext, RenderLabel, ViewNode, ViewNodeRunner},
        render_resource::{
            AddressMode, BindGroup, BindGroupEntry, BindGroupLayout, BindGroupLayoutEntry,
            BindingResource, BindingType, Buffer, BufferBindingType, BufferDescriptor,
            BufferInitDescriptor, BufferUsages, ColorTargetState, ColorWrites, CompareFunction,
            DepthBiasState, DepthStencilState, Extent3d, Face, FilterMode, IndexFormat, LoadOp,
            Operations, Origin3d, PipelineLayoutDescriptor, PrimitiveState, RawFragmentState,
            RawRenderPipelineDescriptor, RawVertexBufferLayout, RawVertexState,
            RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPassDescriptor,
            RenderPipeline, SamplerBindingType, SamplerDescriptor, ShaderModuleDescriptor,
            ShaderSource, ShaderStages, StencilState, StoreOp, TexelCopyBufferLayout,
            TexelCopyTextureInfo, TextureAspect, TextureDescriptor, TextureDimension,
            TextureFormat, TextureSampleType, TextureUsages, TextureView, TextureViewDescriptor,
            TextureViewDimension, VertexStepMode,
        },
        renderer::{RenderContext, RenderDevice, RenderQueue},
        texture::GpuImage,
        view::ViewTarget,
    },
    ui::graph::NodeUi,
    window::{CursorGrabMode, PresentMode, PrimaryWindow, WindowResized},
};
use lib_chunk::{ChunkIndexPlugin, ChunkPosition};
use lib_first_person_camera::FirstPersonCameraPlugin;
use strum::IntoEnumIterator;

use crate::{
    block::Block,
    debug_hud::DebugHudPlugin,
    globals::Globals,
    instance::{DetailedInstance, DetailedInstanceRaw},
    mesh::{MeshingType, Quad, Quads, WorldMeshPlugin},
    vertex::{INDICES, ModelVertex, VERTICES},
    world_gen::WorldGenerationPlugin,
};

mod block;
mod debug_hud;
mod globals;
mod instance;
mod mesh;
mod normal;
mod vertex;
mod world_gen;

const SKY_COLOR: Color = Color::linear_rgba(0.1, 0.2, 0.4, 1.0);
const FOG_COLOR: Color = Color::linear_rgba(0.4, 0.4, 0.4, 1.0);
const AMBIENT_LIGHT: Color = Color::srgb(0.1, 0.1, 0.1);

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    present_mode: PresentMode::AutoNoVsync,
                    ..Default::default()
                }),
                ..Default::default()
            }),
            DebugHudPlugin,
            MyRenderPlugin,
            FirstPersonCameraPlugin::<RenderCamera>::new(),
            ChunkIndexPlugin,
            WorldGenerationPlugin,
            WorldMeshPlugin,
        ))
        .insert_resource(MeshingType::Naive)
        .insert_resource(AmbientLight(AMBIENT_LIGHT))
        .insert_resource(DirectionalLight {
            color: Color::srgb(0.75, 0.75, 0.75),
            direction: Dir3::new(Vec3::new(0.5, -0.75, 2.0))
                .expect("Non-zero light direction vector"),
        })
        .insert_resource(FogSettings {
            color: FOG_COLOR,
            b: 0.001,
        })
        .add_systems(
            Startup,
            (spawn_camera, load_terrain_textures, capture_mouse),
        )
        .run();
}

fn capture_mouse(mut q_windows: Query<&mut Window, With<PrimaryWindow>>) {
    let mut primary_window = q_windows.single_mut().unwrap();

    // for a game that doesn't use the cursor (like a shooter):
    // use `Locked` mode to keep the cursor in one place
    primary_window.cursor_options.grab_mode = CursorGrabMode::Locked;

    // also hide the cursor
    primary_window.cursor_options.visible = false;
}

#[derive(Component)]
struct RenderCamera;

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(5.1, 0.1, 2.).looking_at(Vec3::ZERO, Vec3::Y),
        RenderCamera,
    ));
}

#[derive(Resource)]
struct TerrainColorTextureHandles {
    handles: Vec<Handle<Image>>,
}

fn load_terrain_textures(mut commands: Commands, asset_server: Res<AssetServer>) {
    let mut texture_index_values = Block::iter()
        .filter_map(|block| block.get_texture_index())
        .collect::<Vec<_>>();
    texture_index_values.sort_by_key(|t| t.index);
    let handles = texture_index_values
        .iter()
        .map(|t| t.asset_path)
        .map(|path| asset_server.load(path))
        .collect();
    let resource = TerrainColorTextureHandles { handles };
    commands.insert_resource(resource);
}

#[derive(Resource, Clone, Copy)]
struct AmbientLight(Color);

#[derive(Resource, Clone, Copy)]
struct DirectionalLight {
    color: Color,
    direction: Dir3,
}

#[derive(Resource, Clone, Copy)]
struct FogSettings {
    color: Color,
    b: f32,
}

fn extract_resource_to_render_world<T: Resource + Clone>(
    mut commands: Commands,
    resource: Extract<Option<Res<T>>>,
) {
    match resource.deref() {
        Some(value) => {
            commands.insert_resource(value.deref().clone());
        }
        None => {
            commands.remove_resource::<T>();
        }
    }
}

struct MyRenderPlugin;

impl Plugin for MyRenderPlugin {
    fn build(&self, app: &mut App) {
        let render_app = app
            .add_observer(emit_chunk_despawn_event)
            .add_event::<ChunkDespawn>()
            .sub_app_mut(RenderApp)
            .init_resource::<StartupTime>()
            .init_resource::<CameraData>()
            .init_resource::<PipelineIsNotInitialized>()
            .init_resource::<InstanceBuffers>()
            .add_systems(
                ExtractSchedule,
                (
                    (
                        // extract_stone_texture_handle,
                        prepare_texture_bind_group,
                        init_vertex_buffer,
                        init_pipeline,
                    )
                        .chain()
                        .run_if(resource_exists::<PipelineIsNotInitialized>),
                    update_camera_data,
                    (remove_buffer_for_despawned_chunk, update_instance_buffer).chain(),
                    resize_depth_texture,
                    extract_resource_to_render_world::<AmbientLight>,
                    extract_resource_to_render_world::<DirectionalLight>,
                    extract_resource_to_render_world::<FogSettings>,
                ),
            );

        // add our node (use ViewNodeRunner to run a ViewNode) to the Core2d graph,
        // and insert an ordering edge so our node runs before the UI subgraph.
        render_app
            .add_render_graph_node::<ViewNodeRunner<MyRenderNode>>(Core3d, MyRenderNodeLabel)
            // (Node2d::EndMainPassPostProcessing, MyCustomPassLabel, SubGraphUi)
            // means: EndMainPassPostProcessing -> MyCustomPass -> SubGraphUi
            .add_render_graph_edges(
                Core3d,
                (
                    Node3d::EndMainPassPostProcessing,
                    MyRenderNodeLabel,
                    NodeUi::UiPass,
                ),
            );
    }
}

#[derive(Resource, Default)]
struct PipelineIsNotInitialized;

#[derive(RenderLabel, Hash, Clone, Debug, PartialEq, Eq)]
struct MyRenderNodeLabel;

#[derive(Resource)]
struct TextureBindGroup {
    bind_group: BindGroup,
    layout: BindGroupLayout,
}

fn prepare_texture_bind_group(
    mut commands: Commands,
    gpu_images: Res<RenderAssets<GpuImage>>,
    texture_handles: Extract<Res<TerrainColorTextureHandles>>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    image_assets: Extract<Res<Assets<Image>>>,
) {
    let image_layers = texture_handles
        .handles
        .iter()
        .flat_map(|handle| gpu_images.get(handle))
        .collect::<Vec<_>>();
    if image_layers.len() != texture_handles.handles.len() {
        return;
    }
    info!("Loaded GPU images. Creating texture array.");

    let layer_count = image_layers.len() as u32;
    let extent = Extent3d {
        depth_or_array_layers: layer_count,
        ..image_layers[0].size
    };
    let array_texture = render_device.create_texture(&TextureDescriptor {
        label: Some("terrain_color_texture_array"),
        size: extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: image_layers[0].texture_format,
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        view_formats: &[],
    });

    for (i, img) in image_layers.iter().enumerate() {
        let data = image_assets
            .get(texture_handles.handles[i].id())
            .cloned()
            .expect("Texture should exist in CPU land")
            .data;
        let data = data.unwrap().clone();
        let data = data.as_slice();
        render_queue.write_texture(
            TexelCopyTextureInfo {
                texture: &array_texture,
                mip_level: 0,
                origin: Origin3d {
                    x: 0,
                    y: 0,
                    z: i as _,
                },
                aspect: TextureAspect::All,
            },
            data,
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(img.size.width * 4),
                rows_per_image: None,
            },
            Extent3d {
                depth_or_array_layers: 1,
                ..img.size
            },
        );
    }

    let layout = render_device.create_bind_group_layout(
        Some("my texture bind group layout"),
        &[
            // Texture binding
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: true },
                    view_dimension: TextureViewDimension::D2Array,
                    multisampled: false,
                },
                count: None,
            },
            // Sampler binding
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Sampler(SamplerBindingType::Filtering),
                count: None,
            },
        ],
    );
    let nearest_sampler = render_device.create_sampler(&SamplerDescriptor {
        label: Some("nearest_sampler"),
        mag_filter: FilterMode::Nearest,
        min_filter: FilterMode::Linear,
        mipmap_filter: FilterMode::Nearest,
        address_mode_u: AddressMode::ClampToEdge,
        address_mode_v: AddressMode::ClampToEdge,
        address_mode_w: AddressMode::ClampToEdge,
        ..Default::default()
    });

    // Create view, sampler, and bind group
    let texture_view = array_texture.create_view(&TextureViewDescriptor {
        dimension: Some(TextureViewDimension::D2Array),
        ..Default::default()
    });

    let bind_group = render_device.create_bind_group(
        Some("My texture bind group"),
        &layout,
        &[
            BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(&texture_view),
            },
            BindGroupEntry {
                binding: 1,
                resource: BindingResource::Sampler(&nearest_sampler),
            },
        ],
    );

    commands.insert_resource(TextureBindGroup { bind_group, layout });
}

#[derive(Resource)]
struct MyRenderPipeline {
    pipeline: RenderPipeline,
}

#[derive(Resource)]
struct MyShadowMapPipeline {
    pipeline: RenderPipeline,
}

#[derive(Resource)]
struct GlobalsUniformBuffer {
    buffer: Buffer,
}

#[derive(Resource)]
struct GlobalsUniformBindGroup {
    bind_group: BindGroup,
}

#[derive(Resource)]
struct ShadowPassGlobalsUniformBuffer {
    buffer: Buffer,
}

#[derive(Resource)]
struct ShadowPassGlobalsUniformBindGroup {
    bind_group: BindGroup,
}

#[derive(Resource)]
pub struct IndexBuffer {
    buffer: Buffer,
    num_indices: u32,
}

pub struct DepthTexture {
    view: TextureView,
    format: TextureFormat,
    #[allow(unused)]
    size: UVec2,
}

#[derive(Resource)]
pub struct MainPassDepth(DepthTexture);

#[derive(Resource)]
pub struct ShadowPassDepth(DepthTexture);

#[derive(Resource)]
struct ShadowMapTextureBindGroup {
    bind_group: BindGroup,
    #[allow(unused)]
    layout: BindGroupLayout,
}

fn init_pipeline(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    windows: Extract<Query<&Window>>,
    texture_bind_group: Option<Res<TextureBindGroup>>,
) {
    let Some(texture_bind_group) = texture_bind_group else {
        return;
    };

    commands.remove_resource::<PipelineIsNotInitialized>();

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
        size: std::mem::size_of::<Globals>() as u64,
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
        size: std::mem::size_of::<Globals>() as u64,
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

    let shader = render_device.create_and_validate_shader_module(ShaderModuleDescriptor {
        label: Some("triangle shader"),
        source: ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
    });

    let vertex_layout = RawVertexBufferLayout {
        array_stride: std::mem::size_of::<ModelVertex>() as _,
        step_mode: VertexStepMode::Vertex,
        attributes: &ModelVertex::desc(),
    };

    let instance_layout = RawVertexBufferLayout {
        array_stride: std::mem::size_of::<DetailedInstanceRaw>() as _,
        step_mode: VertexStepMode::Instance,
        attributes: &DetailedInstanceRaw::desc(),
    };

    let index_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("Index buffer"),
        contents: bytemuck::cast_slice(INDICES),
        usage: BufferUsages::INDEX,
    });
    let num_indices = INDICES.len() as u32;
    commands.insert_resource(IndexBuffer {
        buffer: index_buffer,
        num_indices,
    });

    let shadow_pipeline_layout = render_device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("shadow pipeline layout"),
        bind_group_layouts: &[&globals_bind_group_layout],
        push_constant_ranges: &[],
    });

    let shadow_pass_pipeline = render_device.create_render_pipeline(&RawRenderPipelineDescriptor {
        label: Some("shadow pipeline"),
        layout: Some(&shadow_pipeline_layout),
        vertex: RawVertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[vertex_layout.clone(), instance_layout.clone()],
            compilation_options: default(),
        },
        fragment: None,
        primitive: PrimitiveState {
            topology: PrimitiveTopology::TriangleStrip,
            cull_mode: Some(Face::Back),
            ..Default::default()
        },
        depth_stencil: Some(DepthStencilState {
            format: shadow_map.format,
            depth_write_enabled: true,
            depth_compare: CompareFunction::Greater,
            stencil: StencilState::default(),
            bias: DepthBiasState::default(),
        }),
        multisample: default(),
        multiview: None,
        cache: None,
    });

    let shadow_map_sampler = render_device.create_sampler(&SamplerDescriptor {
        label: Some("shadow map sampler"),
        compare: Some(CompareFunction::Greater),
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
                    sample_type: TextureSampleType::Depth,
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            // Sampler binding
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Sampler(SamplerBindingType::Comparison),
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

    let layout = render_device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("main pipeline layout"),
        bind_group_layouts: &[
            &globals_bind_group_layout,
            &texture_bind_group.layout,
            &shadow_map_bind_group_layout,
        ],
        push_constant_ranges: &[],
    });

    let pipeline = render_device.create_render_pipeline(&RawRenderPipelineDescriptor {
        label: Some("main pipeline"),
        layout: Some(&layout),
        vertex: RawVertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[vertex_layout.clone(), instance_layout.clone()],
            compilation_options: default(),
        },
        fragment: Some(RawFragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(ColorTargetState {
                format: TextureFormat::bevy_default(),
                blend: None,
                write_mask: ColorWrites::ALL,
            })],
            compilation_options: default(),
        }),
        primitive: PrimitiveState {
            topology: PrimitiveTopology::TriangleStrip,
            cull_mode: Some(Face::Back),
            ..Default::default()
        },
        depth_stencil: Some(DepthStencilState {
            format: depth_texture.format,
            depth_write_enabled: true,
            depth_compare: CompareFunction::Greater,
            stencil: StencilState::default(),
            bias: DepthBiasState::default(),
        }),
        multisample: default(),
        multiview: None,
        cache: None,
    });

    commands.insert_resource(MainPassDepth(depth_texture));
    commands.insert_resource(MyRenderPipeline { pipeline });
    commands.insert_resource(ShadowPassDepth(shadow_map));
    commands.insert_resource(ShadowMapTextureBindGroup {
        bind_group: shadow_map_bind_group,
        layout: shadow_map_bind_group_layout,
    });
    commands.insert_resource(MyShadowMapPipeline {
        pipeline: shadow_pass_pipeline,
    });
}

fn resize_depth_texture(
    mut resize_events: Extract<EventReader<WindowResized>>,
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

fn create_depth_texture(
    name: &'static str,
    device: &RenderDevice,
    width: u32,
    height: u32,
) -> DepthTexture {
    let format = TextureFormat::Depth32Float;
    let size = Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };

    let desc = TextureDescriptor {
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
    let view = texture.create_view(&TextureViewDescriptor::default());

    DepthTexture {
        view,
        format,
        size: UVec2::new(width, height),
    }
}

#[derive(Resource)]
#[allow(unused)]
struct VertexBuffer {
    vertex_buffer: Buffer,
    num_vertices: u32,
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

#[derive(Resource)]
struct StartupTime(Instant);

impl Default for StartupTime {
    fn default() -> Self {
        Self(Instant::now())
    }
}

#[derive(Resource, Default)]
struct CameraData {
    position: Vec3,
    projection_matrix: Mat4,
}

fn update_camera_data(
    mut camera_data: ResMut<CameraData>,
    camera_query: Extract<Query<(&GlobalTransform, &Projection), With<RenderCamera>>>,
) {
    let (camera_transform, projection) = match camera_query.single() {
        Ok(items) => items,
        Err(QuerySingleError::NoEntities(_)) => {
            warn!("Couldn't find a rendering camera :(");
            return;
        }
        Err(QuerySingleError::MultipleEntities(_)) => {
            warn!("Multiple render cameras????");
            return;
        }
    };
    let projection_matrix =
        projection.get_clip_from_view() * camera_transform.compute_matrix().inverse();
    camera_data.projection_matrix = projection_matrix;
    camera_data.position = camera_transform.translation();
}

struct InstanceBuffer {
    buffer: Buffer,
    num_instances: u32,
}

#[derive(Resource, Default)]
struct InstanceBuffers {
    chunk_pos_to_buffer: HashMap<IVec3, InstanceBuffer>,
}

#[derive(Event)]
struct ChunkDespawn(ChunkPosition);

fn emit_chunk_despawn_event(
    trigger: Trigger<OnRemove, ChunkPosition>,
    q_chunk_position: Query<&ChunkPosition>,
    mut ew: EventWriter<ChunkDespawn>,
) {
    let entity = trigger.target();
    let Ok(pos) = q_chunk_position.get(entity) else {
        return;
    };
    ew.write(ChunkDespawn(*pos));
}

fn remove_buffer_for_despawned_chunk(
    mut er: Extract<EventReader<ChunkDespawn>>,
    mut instance_buffers: ResMut<InstanceBuffers>,
) {
    for ChunkDespawn(ChunkPosition(pos)) in er.read() {
        instance_buffers.chunk_pos_to_buffer.remove(pos);
    }
}

fn update_instance_buffer(
    render_device: Res<RenderDevice>,
    mut instance_buffers: ResMut<InstanceBuffers>,
    q_quads: Extract<Query<(&Quads, &ChunkPosition), Changed<Quads>>>,
) {
    for (quads, chunk_position) in q_quads.iter() {
        let instances_raw = quads
            .0
            .iter()
            .map(|quad| create_instance(quad, chunk_position))
            .map(DetailedInstanceRaw::from)
            .collect::<Vec<_>>();
        let num_instances = instances_raw.len() as u32;
        let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("Instance buffer"),
            contents: bytemuck::cast_slice(instances_raw.as_slice()),
            usage: BufferUsages::VERTEX,
        });
        let item = InstanceBuffer {
            buffer,
            num_instances,
        };
        instance_buffers
            .chunk_pos_to_buffer
            .insert(chunk_position.0, item);
    }
}

fn create_instance(quad: &Quad, chunk_position: &ChunkPosition) -> DetailedInstance {
    let transform =
        Transform::from_translation(quad.pos.as_vec3() + 32.0 * chunk_position.0.as_vec3())
            .with_scale(Vec3::new(quad.width.get() as _, quad.height.get() as _, 1.))
            .looking_to(quad.normal.as_unit_direction().as_vec3() * -0.5, Vec3::Y);
    DetailedInstance {
        transform,
        texture_index: quad
            .block
            .get_texture_index()
            .expect("quad should have texture-able block")
            .index,
        ambient_occlusion: quad.ambient_occlusion,
    }
}

#[derive(Default)]
struct MyRenderNode;

impl ViewNode for MyRenderNode {
    // choose the appropriate ViewQuery type for your node; `()` is a no-op
    type ViewQuery = ();

    fn update(&mut self, world: &mut World) {
        if world.contains_resource::<PipelineIsNotInitialized>() {
            return;
        }
        // Update globals buffer
        let CameraData {
            projection_matrix,
            position: camera_position,
        } = world.resource::<CameraData>();
        let StartupTime(startup_time) = world.resource::<StartupTime>();
        let elapsed_seconds = startup_time.elapsed().as_secs_f32();

        let mut globals = Globals::default();
        globals.elapsed_seconds = elapsed_seconds;
        globals.projection_matrix = projection_matrix.to_cols_array_2d();
        globals.camera_position = camera_position.to_array();
        if let Some(AmbientLight(colour)) = world.get_resource::<AmbientLight>() {
            globals.ambient_light = colour.to_srgba().to_f32_array_no_alpha();
        }
        if let Some(directional_light) = world.get_resource::<DirectionalLight>() {
            globals.directional_light = directional_light.color.to_srgba().to_f32_array_no_alpha();
            globals.directional_light_direction = directional_light.direction.to_array();
            const SHADOW_SIZE: f32 = 128.0;
            const NEGATIVE_Z: Mat4 = Mat4::from_cols_array_2d(&[
                [1., 0., 0., 0.],
                [0., 1., 0., 0.],
                [0., 0., -1., 0.],
                [0., 0., 1., 1.],
            ]);
            let shadow_projection = NEGATIVE_Z
                * Mat4::orthographic_rh(
                    -SHADOW_SIZE,
                    SHADOW_SIZE,
                    -SHADOW_SIZE,
                    SHADOW_SIZE,
                    -SHADOW_SIZE * 2.,
                    SHADOW_SIZE * 2.,
                )
                * Transform::from_translation(Vec3::ZERO)
                    .looking_to(directional_light.direction, Vec3::Y)
                    .compute_matrix()
                    .inverse();
            globals.shadow_map_projection = shadow_projection.to_cols_array_2d();
        }
        if let Some(fog_settings) = world.get_resource::<FogSettings>() {
            globals.fog_color = fog_settings.color.to_linear().to_f32_array_no_alpha();
            globals.fog_b = fog_settings.b;
        }

        let render_queue = world.resource::<RenderQueue>();
        let buffer = world.resource::<GlobalsUniformBuffer>();
        render_queue.write_buffer(&buffer.buffer, 0, bytemuck::bytes_of(&globals));

        let mut shadow_pass_globals = globals.clone();
        shadow_pass_globals.projection_matrix = globals.shadow_map_projection;
        let shadow_pass_buffer = world.resource::<ShadowPassGlobalsUniformBuffer>();
        render_queue.write_buffer(
            &shadow_pass_buffer.buffer,
            0,
            bytemuck::bytes_of(&shadow_pass_globals),
        );
    }

    fn run<'w>(
        &self,
        _graph: &mut RenderGraphContext<'_>,
        render_context: &mut RenderContext<'_>,
        _view_query: <Self::ViewQuery as QueryData>::Item<'_>,
        world: &'_ World,
    ) -> std::result::Result<(), bevy::render::render_graph::NodeRunError> {
        if world.contains_resource::<PipelineIsNotInitialized>() {
            return Ok(());
        }
        let shadow_pipeline = world.resource::<MyShadowMapPipeline>();
        let shadow_depth = world.resource::<ShadowPassDepth>();
        let main_pipeline = world.resource::<MyRenderPipeline>();
        let VertexBuffer { vertex_buffer, .. } = world.resource::<VertexBuffer>();
        let IndexBuffer {
            buffer: index_buffer,
            num_indices,
        } = world.resource::<IndexBuffer>();
        let depth = world.resource::<MainPassDepth>();

        let Some(mut query) =
            world.try_query_filtered::<(&ViewTarget, &ExtractedCamera), With<Camera>>()
        else {
            panic!("Failed query for view target and extracted camera");
        };

        let GlobalsUniformBindGroup {
            bind_group: globals_uniform_bind_group,
        } = world.resource::<GlobalsUniformBindGroup>();
        let ShadowPassGlobalsUniformBindGroup {
            bind_group: shadow_pass_globals_uniform_bind_group,
        } = world.resource::<ShadowPassGlobalsUniformBindGroup>();
        let TextureBindGroup {
            bind_group: texture_bind_group,
            ..
        } = world.resource::<TextureBindGroup>();
        let ShadowMapTextureBindGroup {
            bind_group: shadow_map_bind_group,
            ..
        } = world.resource::<ShadowMapTextureBindGroup>();

        for (view_target, _cam) in query.iter(&world) {
            let shadow_pass_desc = RenderPassDescriptor {
                label: Some("shadow_pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &shadow_depth.0.view,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(0.0),
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            };
            {
                let mut shadow_pass = render_context
                    .command_encoder()
                    .begin_render_pass(&shadow_pass_desc);
                shadow_pass.set_pipeline(&shadow_pipeline.pipeline);
                shadow_pass.set_bind_group(0, shadow_pass_globals_uniform_bind_group, &[]);
                shadow_pass.set_index_buffer(*index_buffer.slice(..).deref(), IndexFormat::Uint16);
                shadow_pass.set_vertex_buffer(0, *vertex_buffer.slice(..).deref());

                for InstanceBuffer {
                    buffer: instance_buffer,
                    num_instances,
                } in world
                    .resource::<InstanceBuffers>()
                    .chunk_pos_to_buffer
                    .values()
                {
                    if num_instances == &0 {
                        continue;
                    }
                    shadow_pass.set_vertex_buffer(1, *instance_buffer.slice(..).deref());
                    shadow_pass.draw_indexed(0..*num_indices, 0, 0..*num_instances);
                }
            }

            let view = view_target.main_texture_view();
            let color_attachment = RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(SKY_COLOR.to_linear().into()),
                    store: StoreOp::Store,
                },
            };

            let desc = RenderPassDescriptor {
                label: Some("triangle_pass"),
                color_attachments: &[Some(color_attachment)],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &depth.0.view,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(0.0),
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            };

            {
                let mut pass = render_context.command_encoder().begin_render_pass(&desc);
                pass.set_pipeline(&main_pipeline.pipeline);
                pass.set_bind_group(0, globals_uniform_bind_group, &[]);
                pass.set_bind_group(1, texture_bind_group, &[]);
                pass.set_bind_group(2, shadow_map_bind_group, &[]);
                pass.set_index_buffer(*index_buffer.slice(..).deref(), IndexFormat::Uint16);
                pass.set_vertex_buffer(0, *vertex_buffer.slice(..).deref());

                for InstanceBuffer {
                    buffer: instance_buffer,
                    num_instances,
                } in world
                    .resource::<InstanceBuffers>()
                    .chunk_pos_to_buffer
                    .values()
                {
                    if num_instances == &0 {
                        continue;
                    }
                    pass.set_vertex_buffer(1, *instance_buffer.slice(..).deref());
                    pass.draw_indexed(0..*num_indices, 0, 0..*num_instances);
                }
            }
        }

        Ok(())
    }
}
