struct FogSettings {
    color: vec3<f32>,
    b: f32,
}

struct Globals {
    time_seconds: f32,
    world_to_clip: mat4x4<f32>,
    camera_position: vec3<f32>,
    ambient_light: vec3<f32>,
    directional_light: vec3<f32>,
    directional_light_direction: vec3<f32>,
    fog: FogSettings,
}

@group(0) @binding(0)
var<uniform> globals: Globals;
@group(1) @binding(0)
var my_texture: texture_2d_array<f32>;
@group(1) @binding(1)
var my_sampler: sampler;

// Vertex shader

struct VertexInput {
    @builtin(vertex_index) index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
    @location(3) uv: vec2<f32>,
}

struct InstanceInput {
    @location(4) model_matrix_0: vec4<f32>,
    @location(5) model_matrix_1: vec4<f32>,
    @location(6) model_matrix_2: vec4<f32>,
    @location(7) model_matrix_3: vec4<f32>,
    @location(8) texture_index: u32,
};

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) material_index: u32,
    @location(1) color: vec4<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) uv: vec2<f32>,
}

@vertex
fn vs_main(in: VertexInput, instance: InstanceInput) -> VertexOutput {
    let local_to_world = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );
    let local_normal_to_world = mat3x3<f32>(
        instance.model_matrix_0.xyz,
        instance.model_matrix_1.xyz,
        instance.model_matrix_2.xyz,
    );
    let world_pos = local_to_world * vec4(in.position, 1.0);
    var out: VertexOutput;
    out.clip_pos = globals.world_to_clip * world_pos;
    out.color = vec4(in.color, 1.0);
    out.uv = in.uv;
    out.normal = local_normal_to_world * in.normal;
    out.material_index = instance.texture_index;
    return out;
}

// Fragment shader

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    let texture_color = textureSample(
        my_texture,
        my_sampler,
        vertex.uv,
        vertex.material_index
    );
    let light = (
        globals.ambient_light 
        + (
            max(0.0, dot(vertex.normal, globals.directional_light_direction))
            * globals.directional_light
        )
    );
    return vertex.color * texture_color * vec4(light, 1.0);
}
