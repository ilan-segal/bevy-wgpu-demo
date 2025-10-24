struct Globals {
    time_seconds: f32,
    world_to_clip: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> globals: Globals;
@group(1) @binding(0)
var my_texture: texture_2d<f32>;
@group(1) @binding(1)
var my_sampler: sampler;
// Vertex shader

const PI: f32 = 3.14159265;

struct VertexInput {
    @builtin(vertex_index) index: u32,
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

struct InstanceInput {
    @location(3) model_matrix_0: vec4<f32>,
    @location(4) model_matrix_1: vec4<f32>,
    @location(5) model_matrix_2: vec4<f32>,
    @location(6) model_matrix_3: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
}

@vertex
fn vs_main(in: VertexInput, instance: InstanceInput) -> VertexOutput {
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );
    var out: VertexOutput;
    out.clip_pos = globals.world_to_clip * model_matrix * vec4(in.position, 1.0);
    out.color = vec4(in.color, 1.0);
    out.uv = in.uv;
    return out;
}

// Fragment shader

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(my_texture, my_sampler, vertex.uv);
    return vertex.color * color;
}
