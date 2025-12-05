struct Globals {
    time_seconds: f32,
    world_to_clip: mat4x4<f32>,
    camera_position: vec3<f32>,
    ambient_light: vec3<f32>,
    directional_light: vec3<f32>,
    directional_light_direction: vec3<f32>,
    fog_color: vec3<f32>,
    fog_b: f32,
    shadow_map_projection: mat4x4<f32>,
    shadow_ndc_display_mode: u32,
}

@group(0) @binding(0)
var<uniform> globals: Globals;
@group(1) @binding(0)
var my_texture: texture_2d_array<f32>;
@group(1) @binding(1)
var my_sampler: sampler;
@group(2) @binding(0)
var shadow_map: texture_depth_2d;
@group(2) @binding(1)
var shadow_map_sampler: sampler_comparison;

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
    @location(4) world_pos: vec3<f32>,
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
    out.world_pos = world_pos.xyz;
    return out;
}

// Fragment shader

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    var texture_color: vec4<f32>;
    var sunlight_factor: f32 = 1.0;
    let red = vec3(1., 0., 0.);
    let green = vec3(0., 1., 0.);
    let val = get_ndc_coordinates(vertex.world_pos);
    switch globals.shadow_ndc_display_mode {
        case 1u: {
            texture_color = vec4(lerp_colours(red, green, -1., 1., val.x), 1.0);
            if (val.y < -1. || val.y > 1. || val.z < 0. || val.z > 1.) {
                texture_color = vec4(0., 0., 0., 1.);
            }
        }
        case 2u: {
            texture_color = vec4(lerp_colours(red, green, -1., 1., val.y), 1.0);
            if (val.x < -1. || val.x > 1. || val.z < 0. || val.z > 1.) {
                texture_color = vec4(0., 0., 0., 1.);
            }
        }
        case 3u: {
            texture_color = vec4(lerp_colours(red, green, 0., 1., val.z), 1.0);
            if (val.x < -1. || val.x > 1. || val.y < -1. || val.y > 1.) {
                texture_color = vec4(0., 0., 0., 1.);
            }
        }
        default: {
            sunlight_factor = get_sunlight_factor(vertex.world_pos);
            if sunlight_factor < 0.0001 {
                return vec4(1., 0., 0., 1.);
            }
            texture_color = textureSample(
                my_texture,
                my_sampler,
                vertex.uv,
                vertex.material_index
            );
            // texture_color = vec4(textureSample(
            //     shadow_map,
            //     my_sampler,
            //     vertex.uv,
            // ) * vec3(1., 1., 1.), 1.);
        }
    }
    // let texture_color = textureSample(
    //     my_texture,
    //     my_sampler,
    //     vertex.uv,
    //     vertex.material_index
    // );
    // let sunlight_factor = get_light_factor_from_sun(vertex.world_pos);
    // return vec4(sunlight_factor, sunlight_factor, sunlight_factor, 1.0);
    let directional_illumination = (
        sunlight_factor
        * max(0.0, dot(vertex.normal, globals.directional_light_direction))
        * globals.directional_light
    );
    let light = globals.ambient_light + directional_illumination;
    let illuminated_color = vertex.color * texture_color * vec4(light, 1.0);
    let camera_distance = distance(globals.camera_position, vertex.world_pos);
    let color = fog_color(illuminated_color, camera_distance);
    return color;
}

fn fog_color(color: vec4<f32>, distance: f32) -> vec4<f32> {
    let fog_amount = 1.0 - exp(-distance * globals.fog_b);
    let fogged_color = mix(color.xyz, globals.fog_color, fog_amount);
    return vec4(fogged_color, color.w);
}

fn lerp_colours(a: vec3<f32>, b: vec3<f32>, x0: f32, x1: f32, x: f32) -> vec3<f32> {
    let t = (x - x0) / (x1 - x0);
    if t < 0. || t > 1. {
        return vec3(0., 0., 0.);
    }
    return ((1.0 - t) * a) + (t * b);
}

// 0.0 -> Shadow
// 1.0 -> Lit
fn get_ndc_coordinates(world_pos: vec3<f32>) -> vec3<f32> {
    let shadow_clip = globals.shadow_map_projection * vec4(world_pos, 1.0);
    let ndc = shadow_clip.xyz / shadow_clip.w;
    return ndc;
}

// 0.0 -> Shadow
// 1.0 -> Lit
fn get_sunlight_factor(world_pos: vec3<f32>) -> f32 {
    let shadow_clip = globals.shadow_map_projection * vec4(world_pos, 1.0);
    let ndc = shadow_clip.xyz / shadow_clip.w;
    // [-1, 1] -> [0, 1]
    let uv = (ndc.xy * 0.5 + vec2(0.5));
    let receiver_depth = ndc.z;
    if (
        uv.x < 0.
        || uv.x > 1.
        || uv.y < 0.
        || uv.y > 1.
        || receiver_depth < 0.
        || receiver_depth > 1.
    ) {
        return 1.0;
    }
    let lit = textureSampleCompare(
        shadow_map,
        shadow_map_sampler,
        uv,
        receiver_depth + 0.007
    );
    return 1.0 - lit;
}