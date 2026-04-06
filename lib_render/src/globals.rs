use std::time::Instant;

use bevy::prelude::*;

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct GlobalsData {
    pub elapsed_seconds: f32,
    _pad_0: [f32; 3], // pad out to 16 bytes
    pub projection_matrix: [[f32; 4]; 4],
    pub camera_position: [f32; 3],
    _pad_1: [f32; 1],
    pub ambient_light: [f32; 3],
    _pad_2: [f32; 1],
    pub directional_light: [f32; 3],
    _pad_3: [f32; 1],
    pub directional_light_direction: [f32; 3],
    _pad_4: [f32; 1],
    pub fog_color: [f32; 3],
    // _pad_5: [f32; 1],
    pub fog_b: f32,
    // _pad_6: [f32; 3],
    pub shadow_map_projection: [[f32; 4]; 4],
}

#[derive(Resource)]
pub struct StartupTime(pub Instant);

impl Default for StartupTime {
    fn default() -> Self {
        Self(Instant::now())
    }
}

#[derive(Resource, Default)]
pub struct CameraData {
    pub position: Vec3,
    pub projection_matrix: Mat4,
}

#[derive(Resource, Clone, Copy)]
pub struct AmbientLight(pub Color);

impl Default for AmbientLight {
    fn default() -> Self {
        Self(Color::BLACK)
    }
}

#[derive(Resource, Clone, Copy)]
pub struct DirectionalLight {
    pub color: Color,
    pub direction: Dir3,
}

#[derive(Resource, Clone, Copy)]
pub struct FogSettings {
    pub color: Color,
    pub b: f32,
}
