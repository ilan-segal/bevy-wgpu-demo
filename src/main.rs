use std::{
    ops::Deref,
    time::{Duration, Instant},
};

use bevy::{
    color::palettes::css::WHITE,
    core_pipeline::core_3d::graph::{Core3d, Node3d},
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    ecs::query::{QueryData, QuerySingleError},
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
            Operations, PipelineLayoutDescriptor, PrimitiveState, RawFragmentState,
            RawRenderPipelineDescriptor, RawVertexBufferLayout, RawVertexState,
            RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPassDescriptor,
            RenderPipeline, SamplerBindingType, SamplerDescriptor, ShaderModuleDescriptor,
            ShaderSource, ShaderStages, StencilState, StoreOp, TextureDescriptor, TextureFormat,
            TextureSampleType, TextureUsages, TextureView, TextureViewDescriptor,
            TextureViewDimension, VertexStepMode,
        },
        renderer::{RenderContext, RenderDevice, RenderQueue},
        texture::GpuImage,
        view::ViewTarget,
    },
    time::common_conditions::on_timer,
    ui::graph::NodeUi,
    window::{CursorGrabMode, PresentMode, PrimaryWindow, WindowResized},
};
use lib_chunk::ChunkIndexPlugin;
use lib_first_person_camera::FirstPersonCameraPlugin;

use crate::{
    globals::Globals,
    instance::{DetailedInstance, DetailedInstanceRaw},
    mesh::{MeshingType, Quad, Quads, WorldMeshPlugin},
    vertex::{INDICES, ModelVertex, VERTICES},
    world_gen::WorldGenerationPlugin,
};

mod block;
mod globals;
mod instance;
mod mesh;
mod normal;
mod vertex;
mod world_gen;

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
            MyRenderPlugin,
            // LogDiagnosticsPlugin::default(),
            FrameTimeDiagnosticsPlugin::default(),
            FirstPersonCameraPlugin::<RenderCamera>::new(),
            ChunkIndexPlugin,
            WorldGenerationPlugin,
            WorldMeshPlugin,
        ))
        .insert_resource(MeshingType::Naive)
        .add_systems(
            Startup,
            (setup_ui, load_stone_texture_handle, capture_mouse),
        )
        .add_systems(
            Update,
            update_fps_counter.run_if(on_timer(Duration::from_secs_f32(0.25))),
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

#[derive(Component)]
struct UiFpsText;

fn setup_ui(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(5.1, 0.1, 2.).looking_at(Vec3::ZERO, Vec3::Y),
        RenderCamera,
    ));
    commands.spawn((
        UiFpsText,
        Text::new(""),
        TextFont {
            font_size: 20.0,
            ..default()
        },
        TextColor(WHITE.into()),
        Node {
            align_self: AlignSelf::Start,
            justify_self: JustifySelf::Start,
            ..default()
        },
    ));
}

fn load_stone_texture_handle(mut commands: Commands, asset_server: Res<AssetServer>) {
    let handle = asset_server.load("smooth_stone.png");
    commands.insert_resource(StoneTextureHandle(handle));
}

#[derive(Resource)]
struct StoneTextureHandle(Handle<Image>);

fn update_fps_counter(
    diagnostics: Res<DiagnosticsStore>,
    mut q_text: Query<&mut Text, With<UiFpsText>>,
) {
    if let Some(fps) = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|fps| fps.smoothed())
    {
        for mut text in &mut q_text {
            text.0 = format!("FPS: {fps:>3.0}");
        }
    } else {
        for mut text in &mut q_text {
            text.0 = format!("FPS: N/A");
        }
    }
}

struct MyRenderPlugin;

impl Plugin for MyRenderPlugin {
    fn build(&self, app: &mut App) {
        let render_app = app
            .sub_app_mut(RenderApp)
            .init_resource::<StartupTime>()
            .init_resource::<CameraProjectionMatrix>()
            .init_resource::<PipelineIsNotInitialized>()
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
                    update_camera_projection_matrix,
                    update_instance_buffer,
                    resize_depth_texture,
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

// fn extract_stone_texture_handle(mut commands: Commands, handle: Extract<Res<StoneTextureHandle>>) {
//     commands.insert_resource(StoneTextureHandle(handle.0.clone_weak()));
// }

#[derive(Resource)]
struct TextureBindGroup {
    bind_group: BindGroup,
    layout: BindGroupLayout,
}

fn prepare_texture_bind_group(
    mut commands: Commands,
    gpu_images: Res<RenderAssets<GpuImage>>,
    texture_handle: Extract<Res<StoneTextureHandle>>,
    render_device: Res<RenderDevice>,
) {
    let Some(image) = gpu_images.get(&texture_handle.0) else {
        info!("Waiting to load GPU image");
        return;
    };
    info!("Loaded GPU image");
    let layout = render_device.create_bind_group_layout(
        Some("my texture bind group layout"),
        &[
            // Texture binding
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: true },
                    view_dimension: TextureViewDimension::D2,
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
    let bind_group = render_device.create_bind_group(
        Some("My texture bind group"),
        &layout,
        &[
            BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(&image.texture_view),
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
struct GlobalsUniformBuffer {
    buffer: Buffer,
}

#[derive(Resource)]
struct GlobalsUniformBindGroup {
    bind_group: BindGroup,
}

#[derive(Resource)]
pub struct IndexBuffer {
    buffer: Buffer,
    num_indices: u32,
}

#[derive(Resource)]
pub struct InstanceBuffer {
    buffer: Buffer,
    num_instances: u32,
}

#[derive(Resource)]
pub struct DepthTexture {
    view: TextureView,
    format: TextureFormat,
    #[allow(unused)]
    size: UVec2,
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
        &render_device,
        window.physical_width(),
        window.physical_height(),
    );

    let globals_bind_group_layout = render_device.create_bind_group_layout(
        Some("Globals bind group layout"),
        &[BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::VERTEX,
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
    commands.insert_resource(GlobalsUniformBuffer {
        buffer: globals_buffer,
    });
    commands.insert_resource(GlobalsUniformBindGroup {
        bind_group: globals_bind_group,
    });

    let shader = render_device.create_and_validate_shader_module(ShaderModuleDescriptor {
        label: Some("triangle shader"),
        source: ShaderSource::Wgsl(include_str!("triangle.wgsl").into()),
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
    // const INSTANCES_PER_ROW: u32 = 1000;
    // const INSTANCES_ROWS: u32 = 1000;
    // const SPACING: f32 = 1.1;
    // let instances_raw = (0..INSTANCES_PER_ROW * INSTANCES_ROWS)
    //     .map(|i| (i % INSTANCES_PER_ROW, i / INSTANCES_PER_ROW))
    //     .map(|(x, z)| {
    //         let x = x as f32;
    //         let z = z as f32;
    //         let translation = Vec3::new(-x * SPACING, 0.0, -z * SPACING);
    //         let rotation = Quat::from_rotation_y(TAU * 0.04 * ((x * 0.5).sin() + 1.0))
    //             * Quat::from_rotation_z(TAU * 0.1 * (((x + z) * 0.5).sin() + 1.0))
    //             * Quat::from_rotation_x(TAU * 0.05 * (((x + z) * 0.5).sin() + 1.0));
    //         DetailedInstance {
    //             translation,
    //             rotation,
    //         }
    //     })
    //     .map(DetailedInstanceRaw::from)
    //     .collect::<Vec<_>>();
    let instances_raw = Vec::<DetailedInstanceRaw>::new();
    info!("{} instances to load.", instances_raw.len());
    let instance_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("Instance buffer"),
        contents: bytemuck::cast_slice(instances_raw.as_slice()),
        usage: BufferUsages::VERTEX,
    });
    commands.insert_resource(InstanceBuffer {
        buffer: instance_buffer,
        num_instances: instances_raw.len() as u32,
    });

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

    let layout = render_device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("triangle pipeline layout"),
        bind_group_layouts: &[&globals_bind_group_layout, &texture_bind_group.layout],
        push_constant_ranges: &[],
    });

    let pipeline = render_device.create_render_pipeline(&RawRenderPipelineDescriptor {
        label: Some("triangle pipeline"),
        layout: Some(&layout),
        vertex: RawVertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[vertex_layout, instance_layout],
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

    commands.insert_resource(depth_texture);
    commands.insert_resource(MyRenderPipeline { pipeline });
}

fn resize_depth_texture(
    mut resize_events: Extract<EventReader<WindowResized>>,
    mut depth: Option<ResMut<DepthTexture>>,
    render_device: Res<RenderDevice>,
) {
    let Some(ref mut depth) = depth else {
        return;
    };
    for event in resize_events.read() {
        let width = event.width as u32;
        let height = event.height as u32;
        **depth = create_depth_texture(&render_device, width, height);
    }
}

fn create_depth_texture(device: &RenderDevice, width: u32, height: u32) -> DepthTexture {
    let format = TextureFormat::Depth32Float;
    let size = Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };

    let desc = TextureDescriptor {
        label: Some("depth_texture"),
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

#[derive(Resource)]
struct CameraProjectionMatrix(Mat4);

impl Default for CameraProjectionMatrix {
    fn default() -> Self {
        Self(Mat4::IDENTITY)
    }
}

fn update_camera_projection_matrix(
    mut matrix: ResMut<CameraProjectionMatrix>,
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
    matrix.0 = projection_matrix;
}

fn update_instance_buffer(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    q_quads: Extract<Query<&Quads, Changed<Quads>>>,
) {
    let Ok(quads) = q_quads.single() else {
        return;
    };
    let instances_raw = quads
        .0
        .iter()
        .map(instance_from_quad)
        .map(DetailedInstanceRaw::from)
        .collect::<Vec<_>>();
    let num_instances = instances_raw.len() as u32;
    info!("{} instances", num_instances);
    let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("Instance buffer"),
        contents: bytemuck::cast_slice(instances_raw.as_slice()),
        usage: BufferUsages::VERTEX,
    });
    commands.insert_resource(InstanceBuffer {
        buffer,
        num_instances,
    });
}

fn instance_from_quad(quad: &Quad) -> DetailedInstance {
    let translation = quad.pos.as_vec3();
    let rotation = Transform::IDENTITY
        .looking_to(quad.normal.as_unit_direction().as_vec3() * -0.5, Vec3::Y)
        .rotation;
    DetailedInstance {
        translation,
        rotation,
        ambient_occlusions: quad.ambient_occlusions,
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
        let projection_matrix = world.resource::<CameraProjectionMatrix>().0;
        let StartupTime(startup_time) = world.resource::<StartupTime>();
        let elapsed_seconds = startup_time.elapsed().as_secs_f32();
        let globals = Globals::new(elapsed_seconds, projection_matrix.to_cols_array_2d());
        // info!("{:?}", camera_transform.compute_matrix());
        let render_queue = world.resource::<RenderQueue>();
        let buffer = world.resource::<GlobalsUniformBuffer>();
        render_queue.write_buffer(&buffer.buffer, 0, bytemuck::bytes_of(&globals));
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
        let pipeline = world.resource::<MyRenderPipeline>();
        let VertexBuffer { vertex_buffer, .. } = world.resource::<VertexBuffer>();
        let IndexBuffer {
            buffer: index_buffer,
            num_indices,
        } = world.resource::<IndexBuffer>();
        let InstanceBuffer {
            buffer: instance_buffer,
            num_instances,
        } = world.resource::<InstanceBuffer>();
        let depth = world.resource::<DepthTexture>();

        let Some(mut query) =
            world.try_query_filtered::<(&ViewTarget, &ExtractedCamera), With<Camera>>()
        else {
            panic!();
        };

        let GlobalsUniformBindGroup {
            bind_group: globals_uniform_bind_group,
        } = world.resource::<GlobalsUniformBindGroup>();
        let TextureBindGroup {
            bind_group: texture_bind_group,
            ..
        } = world.resource::<TextureBindGroup>();

        for (view_target, _cam) in query.iter(&world) {
            let view = view_target.main_texture_view();
            let color_attachment = RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::linear_rgb(0.1, 0.2, 0.4).to_linear().into()),
                    store: StoreOp::Store,
                },
            };

            let desc = RenderPassDescriptor {
                label: Some("triangle_pass"),
                color_attachments: &[Some(color_attachment)],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &depth.view,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(0.0),
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            };

            let mut pass = render_context.command_encoder().begin_render_pass(&desc);
            pass.set_pipeline(&pipeline.pipeline);
            pass.set_bind_group(0, globals_uniform_bind_group, &[]);
            pass.set_bind_group(1, texture_bind_group, &[]);
            pass.set_index_buffer(*index_buffer.slice(..).deref(), IndexFormat::Uint16);
            pass.set_vertex_buffer(0, *vertex_buffer.slice(..).deref());
            if num_instances > &0 {
                pass.set_vertex_buffer(1, *instance_buffer.slice(..).deref());
                pass.draw_indexed(0..*num_indices, 0, 0..*num_instances);
            }
        }

        Ok(())
    }
}
