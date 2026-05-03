use std::num::NonZero;

use bevy::{ecs::query::QueryData, prelude::*};
use lib_async_component::{AsyncComponentPlugin, ComputeInProgress, ComputeTasks};
use lib_chunk::{ChunkPosition, NeighborhoodPlugin};
use lib_noise::FractalNoise;
use lib_spatial::{CHUNK_SIZE, SpatiallyMapped};
use lib_spatial_macro::{SpatiallyMapped2d, SpatiallyMapped3d};
use lib_utils::iter_3d;
use ndarray::{Array2, Array3};
use noise::NoiseFn;

use crate::block::Block;

pub struct WorldGenerationPlugin;

impl Plugin for WorldGenerationPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(WorldSeed(0xDEADBEEF))
            .add_plugins((
                NeighborhoodPlugin::<HeightNoise>::new(),
                NeighborhoodPlugin::<Blocks>::new(),
                AsyncComponentPlugin::<HeightNoise>::new(),
                AsyncComponentPlugin::<Blocks>::new(),
            ))
            .add_systems(
                Startup,
                (init_height_noise_generator, spawn_chunk_at_center_of_world),
            )
            .add_systems(Update, (assign_height_noise, assign_blocks));
    }
}

fn spawn_chunk_at_center_of_world(mut commands: Commands) {
    const RADIUS_HORIZONTAL: i32 = 10;
    const RADIUS_VERTICAL: i32 = 1;
    for (x, y, z) in iter_3d(
        -RADIUS_HORIZONTAL..=RADIUS_HORIZONTAL,
        -RADIUS_VERTICAL..=RADIUS_VERTICAL,
        -RADIUS_HORIZONTAL..=RADIUS_HORIZONTAL,
    ) {
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
struct HeightNoise(Array2<f32>);

impl HeightNoise {
    fn from_noise(chunk_position: ChunkPosition, noise: FractalNoise) -> Self {
        let offset = chunk_position.0 * CHUNK_SIZE as i32;
        let values = Array2::from_shape_fn((CHUNK_SIZE, CHUNK_SIZE), |(x, z)| {
            noise.get([x as i32 + offset.x, z as i32 + offset.z]) as f32
        });
        Self(values)
    }
}

fn assign_height_noise(
    q_chunks: Query<
        (Entity, &ChunkPosition),
        (
            With<Chunk>,
            Without<HeightNoise>,
            Without<ComputeInProgress<HeightNoise>>,
        ),
    >,
    generator: Res<HeightNoiseGenerator>,
    mut height_noise_tasks: ResMut<ComputeTasks<HeightNoise>>,
) {
    for (entity, chunk_position) in q_chunks.iter() {
        let chunk_position = *chunk_position;
        let generator = generator.0.clone();
        height_noise_tasks.spawn_task(entity, async move {
            HeightNoise::from_noise(chunk_position, generator)
        });
    }
}

#[derive(QueryData)]
struct BlockGenerationData {
    entity: Entity,
    chunk_position: &'static ChunkPosition,
    height_noise: &'static HeightNoise,
}

#[derive(Component, Clone, SpatiallyMapped3d)]
pub struct Blocks(Array3<Block>);

const BEDROCK_DEPTH: i32 = -128;
const DIRT_LAYER_THICKNESS: u32 = 3;
const WORLD_AMPLITUDE: f32 = 10.;

fn assign_blocks(
    mut commands: Commands,
    q_chunks: Query<BlockGenerationData, (With<Chunk>, Without<Blocks>)>,
) {
    for item in q_chunks.iter() {
        let chunk_y = item.chunk_position.0.y * CHUNK_SIZE as i32;
        let blocks = Array3::from_shape_fn((CHUNK_SIZE, CHUNK_SIZE, CHUNK_SIZE), |(x, y, z)| {
            let height_sample = *item.height_noise.at_pos([x, z]);
            let true_y = (y as i32 + chunk_y) as f32;
            let ground_height = height_sample * WORLD_AMPLITUDE;
            if true_y + 1. < BEDROCK_DEPTH as _ {
                Block::Air
            } else if true_y < BEDROCK_DEPTH as _ {
                Block::Bedrock
            } else if (true_y + (DIRT_LAYER_THICKNESS + 1) as f32) < ground_height {
                Block::Stone
            } else if true_y + 1. < ground_height {
                Block::Dirt
            } else if true_y < ground_height {
                Block::Grass
            } else {
                Block::Air
            }
        });
        commands.entity(item.entity).try_insert(Blocks(blocks));
    }
}
