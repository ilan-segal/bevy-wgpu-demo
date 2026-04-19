use std::{marker::PhantomData, num::NonZero, ops::Deref};

use bevy::{
    platform::collections::HashMap,
    prelude::*,
    render::{
        Extract, camera::CameraProjection, render_graph::RenderGraphApp,
        render_resource::BufferUsages,
    },
};
use strum::IntoEnumIterator;

use crate::{
    camera::RenderCamera,
    pipeline::OpaquePass,
    render_node::{MyRenderNode, MyRenderNodeLabel},
};

pub mod camera;
pub mod globals;
mod instance;
pub mod pipeline;
mod render_node;
pub mod texture;
mod vertex;

const SKY_COLOR: Color = Color::linear_rgba(0.1, 0.2, 0.4, 1.0);

pub struct TerrainRenderPlugin<TerrainType> {
    _phantom: PhantomData<TerrainType>,
}

impl<T> TerrainRenderPlugin<T> {
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<TerrainType: 'static + Send + Sync + texture::TextureIndex + IntoEnumIterator> Plugin
    for TerrainRenderPlugin<TerrainType>
{
    fn build(&self, app: &mut App) {
        let render_app = app
            .add_observer(emit_quads_despawn_event)
            .add_event::<TerrainDespawnEvent>()
            .add_plugins((
                vertex::VertexPlugin,
                texture::TexturePlugin::<TerrainType>::new(),
            ))
            .sub_app_mut(bevy::render::RenderApp)
            .init_resource::<globals::StartupTime>()
            .init_resource::<globals::CameraData>()
            .init_resource::<InstanceBuffers>()
            .add_systems(
                ExtractSchedule,
                (
                    // prepare_texture_bind_group,
                    pipeline::init_pipeline.run_if(not(resource_exists::<
                        pipeline::MyRenderPipeline<OpaquePass>,
                    >)),
                    (
                        remove_buffer_for_despawned_terrain,
                        update_instance_buffer::<TerrainType>,
                    )
                        .chain(),
                    pipeline::resize_depth_texture,
                    update_camera_data,
                    extract_resource_to_render_world::<globals::AmbientLight>,
                    extract_resource_to_render_world::<globals::DirectionalLight>,
                    extract_resource_to_render_world::<globals::FogSettings>,
                ),
            );

        // add our node (use ViewNodeRunner to run a ViewNode) to the Core2d graph,
        // and insert an ordering edge so our node runs before the UI subgraph.
        render_app
            .add_render_graph_node::<bevy::render::render_graph::ViewNodeRunner<MyRenderNode>>(
                bevy::core_pipeline::core_3d::graph::Core3d,
                MyRenderNodeLabel,
            )
            // (Node2d::EndMainPassPostProcessing, MyCustomPassLabel, SubGraphUi)
            // means: EndMainPassPostProcessing -> MyCustomPass -> SubGraphUi
            .add_render_graph_edges(
                bevy::core_pipeline::core_3d::graph::Core3d,
                (
                    bevy::core_pipeline::core_3d::graph::Node3d::EndMainPassPostProcessing,
                    MyRenderNodeLabel,
                    bevy::ui::graph::NodeUi::UiPass,
                ),
            );
    }
}

fn update_camera_data(
    mut camera_data: ResMut<globals::CameraData>,
    camera_query: bevy::render::Extract<Query<(&GlobalTransform, &Projection), With<RenderCamera>>>,
) {
    let (camera_transform, projection) = match camera_query.single() {
        Ok(items) => items,
        Err(bevy::ecs::query::QuerySingleError::NoEntities(_)) => {
            warn!("Couldn't find a rendering camera :(");
            return;
        }
        Err(bevy::ecs::query::QuerySingleError::MultipleEntities(_)) => {
            warn!("Multiple render cameras????");
            return;
        }
    };
    let projection_matrix = projection.get_clip_from_view()
        * camera_transform
            .compute_matrix()
            .inverse();
    camera_data.projection_matrix = projection_matrix;
    camera_data.position = camera_transform.translation();
}

fn extract_resource_to_render_world<T: Resource + Clone>(
    mut commands: Commands,
    resource: bevy::render::Extract<Option<Res<T>>>,
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

#[derive(Component, Clone, Copy, Debug)]
pub struct TerrainPosition(pub IVec3);

#[derive(Event)]
pub(crate) struct TerrainDespawnEvent(TerrainPosition);

fn emit_quads_despawn_event(
    trigger: Trigger<OnRemove, TerrainPosition>,
    q_chunk_position: Query<&TerrainPosition>,
    mut ew: EventWriter<TerrainDespawnEvent>,
) {
    let entity = trigger.target();
    let Ok(pos) = q_chunk_position.get(entity) else {
        return;
    };
    ew.write(TerrainDespawnEvent(*pos));
}

pub(crate) struct InstanceBuffer {
    buffer: bevy::render::render_resource::Buffer,
    num_instances: u32,
}

#[derive(Resource, Default)]
pub(crate) struct InstanceBuffers {
    chunk_pos_to_buffer: HashMap<IVec3, InstanceBuffer>,
}

fn remove_buffer_for_despawned_terrain(
    mut er: bevy::render::Extract<EventReader<TerrainDespawnEvent>>,
    mut instance_buffers: ResMut<InstanceBuffers>,
) {
    for TerrainDespawnEvent(TerrainPosition(pos)) in er.read() {
        instance_buffers
            .chunk_pos_to_buffer
            .remove(pos);
    }
}

#[derive(Component)]
struct Buffered;

fn update_instance_buffer<TerrainType: Send + Sync + texture::TextureIndex>(
    render_device: Res<bevy::render::renderer::RenderDevice>,
    mut instance_buffers: ResMut<InstanceBuffers>,
    q_quads: Extract<Query<(&Quads<TerrainType>, &TerrainPosition), Changed<Quads<TerrainType>>>>,
    indices: Extract<Res<texture::TerrainColorTextureIndices>>,
) {
    // let Some(indices) = indices.deref() else {
    //     info!("gluh");
    //     return;
    // };
    for (quads, chunk_position) in q_quads.iter() {
        if quads.0.is_empty() {
            continue;
        }
        let instances_raw = quads
            .0
            .iter()
            .map(|quad| create_instance(quad, indices.as_ref()))
            .map(instance::RawInstance::from)
            .collect::<Vec<_>>();
        let num_instances = instances_raw.len() as u32;
        let buffer = render_device.create_buffer_with_data(
            &bevy::render::render_resource::BufferInitDescriptor {
                label: Some("Instance buffer"),
                contents: bytemuck::cast_slice(instances_raw.as_slice()),
                usage: BufferUsages::VERTEX,
            },
        );
        let item = InstanceBuffer {
            buffer,
            num_instances,
        };
        instance_buffers
            .chunk_pos_to_buffer
            .insert(chunk_position.0, item);
    }
}

fn create_instance<TerrainType: texture::TextureIndex>(
    quad: &Quad<TerrainType>,
    // chunk_position: &TerrainPosition,
    indices: &texture::TerrainColorTextureIndices,
) -> instance::Instance {
    instance::Instance {
        normal: quad.normal,
        local_pos: quad.pos.to_array().map(|x| x as _),
        // chunk_pos: chunk_position.0.to_array(),
        texture_index: *indices
            .get_index(&quad.ty)
            .expect("Terrain texture index") as _,
        ambient_occlusion: quad.ambient_occlusion,
    }
}

#[derive(Component)]
pub struct Quads<TerrainType>(pub Vec<Quad<TerrainType>>);

pub struct Quad<TerrainType> {
    pub ty: TerrainType,
    pub normal: Normal,
    pub width: NonZero<u32>,
    pub height: NonZero<u32>,
    pub pos: IVec3,
    /// Column-wise, starting with top right
    pub ambient_occlusion: [u8; 4],
}

#[derive(Clone, Copy, Debug)]
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
