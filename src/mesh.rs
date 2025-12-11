use std::num::NonZero;

use bevy::prelude::*;
use lib_async_component::{AsyncComponentPlugin, ComputeTasks};
use lib_chunk::Neighborhood;
use lib_utils::cube_iter;

use crate::{
    block::Block,
    normal::Normal,
    world_gen::{Blocks, Chunk},
};

pub struct WorldMeshPlugin;

impl Plugin for WorldMeshPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<QuadCount>()
            .add_systems(Update, assign_quads)
            .add_observer(update_quad_count_for_despawn)
            .add_observer(update_quad_count_for_replace)
            .add_observer(update_quad_count_for_insert)
            .add_plugins(AsyncComponentPlugin::<Quads>::new());
    }
}

#[derive(Component)]
pub struct Quads(pub Vec<Quad>);

pub struct Quad {
    pub block: Block,
    pub normal: Normal,
    pub width: NonZero<u32>,
    pub height: NonZero<u32>,
    pub pos: IVec3,
    /// Column-wise, starting with top right
    pub ambient_occlusion: [u8; 4],
}

#[derive(Resource, Default)]
pub struct QuadCount(pub u32);

fn update_quad_count_for_despawn(
    trigger: Trigger<OnRemove, Quads>,
    mut count: ResMut<QuadCount>,
    q_quads: Query<&Quads>,
) {
    let entity = trigger.target();
    let Ok(quads) = q_quads.get(entity) else {
        return;
    };
    count.0 -= quads.0.len() as u32;
}

fn update_quad_count_for_replace(
    trigger: Trigger<OnReplace, Quads>,
    mut count: ResMut<QuadCount>,
    q_quads: Query<&Quads>,
) {
    let entity = trigger.target();
    let Ok(quads) = q_quads.get(entity) else {
        return;
    };
    count.0 -= quads.0.len() as u32;
}

fn update_quad_count_for_insert(
    trigger: Trigger<OnInsert, Quads>,
    mut count: ResMut<QuadCount>,
    q_quads: Query<&Quads>,
) {
    let entity = trigger.target();
    let Ok(quads) = q_quads.get(entity) else {
        return;
    };
    count.0 += quads.0.len() as u32;
}

#[derive(Resource, Clone)]
pub enum MeshingType {
    Naive,
}

fn assign_quads(
    meshing_type: Res<MeshingType>,
    q_unmeshed_chunks: Query<
        (Entity, &Neighborhood<Blocks>),
        (With<Chunk>, Changed<Neighborhood<Blocks>>),
    >,
    mut compute_tasks: ResMut<ComputeTasks<Quads>>,
) {
    for (entity, blocks) in q_unmeshed_chunks.iter() {
        let blocks = blocks.clone();
        let meshing_type = meshing_type.clone();
        compute_tasks.spawn_task(entity, async move { get_quads(blocks, meshing_type) });
    }
}

fn get_quads(blocks: Neighborhood<Blocks>, meshing_type: MeshingType) -> Quads {
    let quads = match meshing_type {
        MeshingType::Naive => get_quads_naive(&blocks),
    };
    Quads(quads)
}

fn get_quads_naive(blocks: &Neighborhood<Blocks>) -> Vec<Quad> {
    cube_iter(0..32)
        .map(|(x, y, z)| [x, y, z])
        .flat_map(|pos| get_quads_around_block(blocks, pos))
        .collect()
}

fn get_quads_around_block(
    blocks: &Neighborhood<Blocks>,
    pos: [i32; 3],
) -> impl Iterator<Item = Quad> {
    [
        Normal::PosX,
        Normal::NegX,
        Normal::PosY,
        Normal::NegY,
        Normal::PosZ,
        Normal::NegZ,
    ]
    .iter()
    .filter_map(move |normal| get_quad_on_face(blocks, pos, normal))
}

fn get_quad_on_face(blocks: &Neighborhood<Blocks>, pos: [i32; 3], normal: &Normal) -> Option<Quad> {
    let block = blocks
        .at_pos(&pos)
        .filter(|block| block != &&Block::Air)
        .cloned()?;
    let pos = IVec3::from(pos);
    let other_pos = pos + normal.as_unit_direction();
    let other_block = blocks
        .at_pos(&other_pos.into())
        .cloned()
        .unwrap_or_default();
    if !other_block.is_transparent() {
        return None;
    }
    let quad = Quad {
        block,
        normal: *normal,
        width: NonZero::new(1).unwrap(),
        height: NonZero::new(1).unwrap(),
        pos,
        ambient_occlusion: [0, 1, 2, 3]
            .map(|idx| get_ambient_occlusion_factor(blocks, pos, normal, idx)),
    };
    return Some(quad);
}

fn get_ambient_occlusion_factor(
    blocks: &Neighborhood<Blocks>,
    pos: IVec3,
    normal: &Normal,
    corner_index: u8,
) -> u8 {
    let (a0, a1) = get_perpendicular_axes(normal);
    let one_layer_up = normal.as_unit_direction() + pos;
    let offset_0 = a0.as_unit_direction()
        * match corner_index {
            0 | 1 => -1,
            _ => 1,
        };
    let offset_1 = a1.as_unit_direction()
        * match corner_index {
            0 | 2 => -1,
            _ => 1,
        };
    let is_solid = |p: IVec3| {
        blocks
            .at_pos(&p.to_array())
            .map(|block| !block.is_transparent())
            .unwrap_or(false)
    };
    let left = is_solid(one_layer_up + offset_0);
    let right = is_solid(one_layer_up + offset_1);
    let corner = is_solid(one_layer_up + offset_0 + offset_1);
    if left && right {
        return 4;
    }
    if left || right {
        if corner {
            return 3;
        } else {
            return 2;
        }
    }
    if corner {
        return 1;
    }
    return 0;
}

fn get_perpendicular_axes(normal: &Normal) -> (Normal, Normal) {
    match normal {
        Normal::PosX => (Normal::NegZ, Normal::NegY),
        Normal::PosY => (Normal::NegZ, Normal::PosX),
        Normal::PosZ => (Normal::PosX, Normal::NegY),
        Normal::NegX => (Normal::PosZ, Normal::NegY),
        Normal::NegY => (Normal::NegZ, Normal::NegX),
        Normal::NegZ => (Normal::NegX, Normal::NegY),
    }
}
