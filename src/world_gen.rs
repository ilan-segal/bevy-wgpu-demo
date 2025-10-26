use std::num::NonZero;

use bevy::{ecs::query::QueryData, prelude::*};
use lib_chunk::{ChunkPosition, NeighborhoodPlugin};
use lib_noise::FractalNoise;
use lib_spatial::{CHUNK_SIZE, SpatiallyMapped};
use lib_spatial_macro::{SpatiallyMapped2d, SpatiallyMapped3d};
use lib_utils::{cube_iter, square_iter};
use noise::NoiseFn;

use crate::block::Block;

pub struct WorldGenerationPlugin;

impl Plugin for WorldGenerationPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(WorldSeed(0xDEADBEEF))
            .add_plugins((
                NeighborhoodPlugin::<HeightNoise>::new(),
                NeighborhoodPlugin::<Blocks>::new(),
            ))
            .add_systems(
                Startup,
                (init_height_noise_generator, spawn_chunk_at_center_of_world),
            )
            .add_systems(Update, (assign_height_noise, assign_blocks));
    }
}

fn spawn_chunk_at_center_of_world(mut commands: Commands) {
    for (x, y, z) in cube_iter(-2..=2) {
        let pos = IVec3::new(x, y, z);
        commands.spawn((Chunk, ChunkPosition(pos)));
    }
}

#[derive(Resource)]
struct WorldSeed(u32);

#[derive(Resource)]
struct HeightNoiseGenerator(FractalNoise);

fn init_height_noise_generator(mut commands: Commands, world_seed: Res<WorldSeed>) {
    let seed = world_seed.0;
    let num_layers = 6;
    let scale = 0.02;
    let noise = FractalNoise::new(seed, NonZero::new(num_layers).unwrap(), scale);
    let generator = HeightNoiseGenerator(noise);
    commands.insert_resource(generator);
}

#[derive(Component)]
pub struct Chunk;

#[derive(Component, Clone, SpatiallyMapped2d)]
struct HeightNoise(Vec<f64>);

impl HeightNoise {
    fn from_noise(chunk_position: &ChunkPosition, noise: &FractalNoise) -> Self {
        let offset = chunk_position.0 * CHUNK_SIZE as i32;
        let values = square_iter(0..CHUNK_SIZE as i32)
            .map(|(x, z)| [x + offset.x, z + offset.z])
            .map(|point| noise.get(point))
            .collect();
        Self(values)
    }
}

fn assign_height_noise(
    mut commands: Commands,
    q_chunks: Query<(Entity, &ChunkPosition), (With<Chunk>, Without<HeightNoise>)>,
    generator: Res<HeightNoiseGenerator>,
) {
    for (entity, chunk_position) in q_chunks.iter() {
        let height_noise = HeightNoise::from_noise(chunk_position, &generator.0);
        commands.entity(entity).try_insert(height_noise);
    }
}

#[derive(QueryData)]
struct BlockGenerationData {
    entity: Entity,
    chunk_position: &'static ChunkPosition,
    height_noise: &'static HeightNoise,
}

#[derive(Component, Clone, SpatiallyMapped3d)]
pub struct Blocks(Vec<Block>);

fn assign_blocks(
    mut commands: Commands,
    q_chunks: Query<BlockGenerationData, (With<Chunk>, Without<Blocks>)>,
) {
    const WORLD_AMPLITUDE: f64 = 10.;
    for item in q_chunks.iter() {
        let chunk_y = item.chunk_position.0.y * CHUNK_SIZE as i32;
        let blocks = cube_iter(0..CHUNK_SIZE)
            .map(|(x, y, z)| {
                let height_sample = *item.height_noise.at_pos([x, z]);
                let true_y = (y as i32 + chunk_y) as f64;
                if true_y + 1. < height_sample * WORLD_AMPLITUDE {
                    Block::Stone
                } else if true_y < height_sample * WORLD_AMPLITUDE {
                    Block::Dirt
                } else {
                    Block::Air
                }
            })
            .collect();
        commands.entity(item.entity).try_insert(Blocks(blocks));
    }
}
