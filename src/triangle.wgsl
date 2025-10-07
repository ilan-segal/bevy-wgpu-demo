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
    let init_color = in.color;
    var hsv = rgb_to_hsv(init_color);
    hsv.x += globals.time_seconds;
    let color = hsv_to_rgb(hsv);
    out.color = vec4(color, 1.0);
    out.uv = in.uv;
    return out;
}

fn rgb_to_hsv(rgb: vec3<f32>) -> vec3<f32> {
    let max_comp = max(rgb.r, max(rgb.g, rgb.b));
    let min_comp = min(rgb.r, min(rgb.g, rgb.b));
    let delta = max_comp - min_comp;

    var h: f32 = 0.0;
    var s: f32 = 0.0;
    let v: f32 = max_comp;

    if (delta == 0.0) { // Achromatic (gray)
        h = 0.0;
        s = 0.0;
    } else {
        if (max_comp != 0.0) {
            s = delta / max_comp;
        } else { // Black
            s = 0.0;
            h = 0.0; // or undefined, depending on desired behavior
            return vec3<f32>(h, s, v);
        }

        if (max_comp == rgb.r) {
            h = (rgb.g - rgb.b) / delta;
        } else if (max_comp == rgb.g) {
            h = (rgb.b - rgb.r) / delta + 2.0;
        } else { // max_comp == rgb.b
            h = (rgb.r - rgb.g) / delta + 4.0;
        }
        h = h / 6.0; // Normalize hue to [0, 1]
        if (h < 0.0) {
            h += 1.0;
        }
    }
    return vec3<f32>(h, s, v) * vec3(2.0 * PI, 1.0, 1.0);
}

/// https://en.wikipedia.org/wiki/HSL_and_HSV#HSV_to_RGB
fn hsv_to_rgb(hsv: vec3<f32>) -> vec3<f32> {
    let h = (hsv.x / PI * 180.) % 360.;
    let s = hsv.y;
    let v = hsv.z;

    let c = v * s;
    let x = c * (1.0 - abs((h / 60.0) % 2.0 - 1.0));
    let m = v - c;

    var rgb_prime: vec3<f32>;

    if (0.0 <= h && h < 60.0) {
        rgb_prime = vec3<f32>(c, x, 0.0);
    } else if (60.0 <= h && h < 120.0) {
        rgb_prime = vec3<f32>(x, c, 0.0);
    } else if (120.0 <= h && h < 180.0) {
        rgb_prime = vec3<f32>(0.0, c, x);
    } else if (180.0 <= h && h < 240.0) {
        rgb_prime = vec3<f32>(0.0, x, c);
    } else if (240.0 <= h && h < 300.0) {
        rgb_prime = vec3<f32>(x, 0.0, c);
    } else { // 300.0 <= h && h < 360.0
        rgb_prime = vec3<f32>(c, 0.0, x);
    }

    return rgb_prime + vec3<f32>(m, m, m);
}

// Fragment shader

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(my_texture, my_sampler, vertex.uv);
    return vertex.color * color;
}
