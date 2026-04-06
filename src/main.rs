use bevy::{
    prelude::*,
    window::{CursorGrabMode, PresentMode, PrimaryWindow},
};
use lib_chunk::{ChunkIndexPlugin, ChunkPosition};
use lib_first_person_camera::FirstPersonCameraPlugin;

use crate::{
    debug_hud::DebugHudPlugin,
    world_gen::{Chunk, WorldGenerationPlugin},
};

mod block;
mod debug_hud;
mod mesh;
mod world_gen;

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
            lib_render::TerrainRenderPlugin::<crate::block::Terrain>::new(),
            FirstPersonCameraPlugin::<lib_render::camera::RenderCamera>::new(),
            ChunkIndexPlugin,
            WorldGenerationPlugin,
            mesh::WorldMeshPlugin,
        ))
        .insert_resource(mesh::MeshingType::Naive)
        .insert_resource(lib_render::globals::AmbientLight(AMBIENT_LIGHT))
        .insert_resource(lib_render::globals::DirectionalLight {
            color: Color::srgb(0.75, 0.75, 0.75),
            direction: Dir3::new(Vec3::new(0.5, -0.75, 2.0))
                .expect("Non-zero light direction vector"),
        })
        .insert_resource(lib_render::globals::FogSettings {
            color: FOG_COLOR,
            b: 0.001,
        })
        .add_systems(Startup, (spawn_camera, capture_mouse))
        .add_systems(Update, assign_terrain_position)
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

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(5.1, 0.1, 2.).looking_at(Vec3::ZERO, Vec3::Y),
        lib_render::camera::RenderCamera,
    ));
}

fn assign_terrain_position(
    mut commands: Commands,
    q_chunk: Query<(Entity, &ChunkPosition), (With<Chunk>, Without<lib_render::TerrainPosition>)>,
) {
    for (entity, chunk_pos) in q_chunk.iter() {
        let terrain_position = lib_render::TerrainPosition(chunk_pos.0);
        commands.entity(entity).try_insert(terrain_position);
    }
}
