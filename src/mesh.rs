use std::num::NonZero;

use bevy::prelude::*;
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
        app.add_systems(Update, assign_quads);
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
}

#[derive(Resource)]
pub enum MeshingType {
    Naive,
}

fn assign_quads(
    mut commands: Commands,
    meshing_type: Res<MeshingType>,
    q_unmeshed_chunks: Query<
        (Entity, &Neighborhood<Blocks>),
        (With<Chunk>, Changed<Neighborhood<Blocks>>),
    >,
) {
    for (entity, blocks) in q_unmeshed_chunks.iter() {
        let quads = get_quads(blocks, &meshing_type);
        let quads = Quads(quads);
        commands.entity(entity).try_insert(quads);
    }
}

fn get_quads(blocks: &Neighborhood<Blocks>, meshing_type: &MeshingType) -> Vec<Quad> {
    match meshing_type {
        MeshingType::Naive => get_quads_naive(blocks),
    }
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
    let posv = IVec3::from(pos);
    let other_pos = posv + normal.as_unit_direction();
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
        pos: pos.into(),
    };
    return Some(quad);
}
