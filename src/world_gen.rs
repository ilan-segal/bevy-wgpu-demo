use std::num::NonZero;

use bevy::prelude::*;
use lib_chunk::ChunkPosition;
use lib_noise::FractalNoise;
use lib_spatial::{CHUNK_SIZE, SpatiallyMapped};
use lib_spatial_macro::SpatiallyMapped3d;
use lib_utils::cube_iter;
use noise::NoiseFn;

pub struct WorldGenerationPlugin;

impl Plugin for WorldGenerationPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(WorldSeed(0xDEADBEEF))
            .add_systems(Startup, init_height_noise_generator)
            .add_systems(Update, assign_height_noise);
    }
}

#[derive(Resource)]
struct WorldSeed(u32);

#[derive(Resource)]
struct HeightNoiseGenerator(FractalNoise);

fn init_height_noise_generator(mut commands: Commands, world_seed: Res<WorldSeed>) {
    let seed = world_seed.0;
    let num_layers = 3;
    let scale = 32.0;
    let noise = FractalNoise::new(seed, NonZero::new(num_layers).unwrap(), scale);
    let generator = HeightNoiseGenerator(noise);
    commands.insert_resource(generator);
}

#[derive(Component)]
struct Chunk;

#[derive(Component, SpatiallyMapped3d)]
struct HeightNoise(Vec<f64>);

impl HeightNoise {
    fn from_noise(chunk_position: &ChunkPosition, noise: &FractalNoise) -> Self {
        let values = cube_iter(0..CHUNK_SIZE as i32)
            .map(IVec3::from)
            .map(|pos| pos + chunk_position.0)
            .map(|pos| [pos.x, pos.z])
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
