use bevy::math::Mat4;

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Globals {
    pub elapsed_seconds: f32,
    _pad_0: [f32; 3], // pad out to 16 bytes
    pub projection_matrix: [[f32; 4]; 4],
    pub ambient_light: [f32; 3],
    _pad_1: [f32; 1],
}

impl Default for Globals {
    fn default() -> Self {
        Self {
            elapsed_seconds: 0.0,
            _pad_0: [0.; 3],
            projection_matrix: Mat4::IDENTITY.to_cols_array_2d(),
            ambient_light: [0.0; 3],
            _pad_1: [0.; 1],
        }
    }
}
