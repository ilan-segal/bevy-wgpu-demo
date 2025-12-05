#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct Globals {
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
    pub ndc_mode: u32,
    _pad_6: [f32; 3],
}
