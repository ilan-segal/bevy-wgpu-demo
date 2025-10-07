use bevy::math::Mat4;

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Globals {
    pub elapsed_seconds: f32,
    _pad: [f32; 3], // pad out to 16 bytes
    pub projection_matrix: [[f32; 4]; 4],
}

impl Default for Globals {
    fn default() -> Self {
        Self {
            elapsed_seconds: 0.0,
            _pad: [0.; 3],
            projection_matrix: Mat4::IDENTITY.to_cols_array_2d(),
        }
    }
}

impl Globals {
    pub fn new<Matrix: Into<[[f32; 4]; 4]>>(seconds: f32, projection_matrix: Matrix) -> Self {
        Self {
            elapsed_seconds: seconds,
            projection_matrix: projection_matrix.into(),
            ..Default::default()
        }
    }
}
