// Wings UI material. Self contained: the UI vertex output and the globals uniform are declared
// here with the exact layout used by the Bevy 0.20 UI material pipeline.

struct UiVertexOutput {
    @location(0) uv: vec2<f32>,
    @location(1) border_widths: vec4<f32>,
    @location(2) border_radius_x: vec4<f32>,
    @location(3) border_radius_y: vec4<f32>,
    @location(4) @interpolate(flat) size: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};

struct Globals {
    time: f32,
    delta_time: f32,
    frame_count: u32,
};

@group(0) @binding(1)
var<uniform> globals: Globals;

// Signed distance to a rounded box centered at the origin. Radii order: top left, top right,
// bottom right, bottom left.
fn sd_rounded_box(point: vec2<f32>, size: vec2<f32>, radii: vec4<f32>) -> f32 {
    let rs = select(radii.xy, radii.wz, 0.0 < point.y);
    let radius = select(rs.x, rs.y, 0.0 < point.x);
    let q = abs(point) - 0.5 * size + radius;
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - radius;
}

// Anti aliased coverage of the node shape (1 inside, 0 outside).
fn shape_coverage(in: UiVertexOutput) -> f32 {
    let p = (in.uv - vec2<f32>(0.5)) * in.size;
    let d = sd_rounded_box(p, in.size, in.border_radius_x);
    return clamp(0.5 - d, 0.0, 1.0);
}


struct NoiseMaterial {
    base: vec4<f32>,
    tint: vec4<f32>,
    scale: f32,
    speed: f32,
    intensity: f32,
    octaves: f32,
};

@group(1) @binding(0)
var<uniform> material: NoiseMaterial;

fn hash(p: vec2<f32>) -> f32 {
    let q = fract(p * vec2<f32>(123.34, 456.21));
    let r = q + dot(q, q + 45.32);
    return fract(r.x * r.y);
}

// Smooth value noise in 0..1.
fn value_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash(i);
    let b = hash(i + vec2<f32>(1.0, 0.0));
    let c = hash(i + vec2<f32>(0.0, 1.0));
    let d = hash(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// Fractal noise: octaves of value noise, each twice as fine and half as strong.
fn fbm(p: vec2<f32>, octaves: i32) -> f32 {
    var sum = 0.0;
    var amplitude = 0.5;
    var total = 0.0;
    var q = p;
    for (var i = 0; i < 8; i++) {
        if i >= octaves {
            break;
        }
        sum += amplitude * value_noise(q);
        total += amplitude;
        q = q * 2.03 + vec2<f32>(17.0, 31.0);
        amplitude *= 0.5;
    }
    return sum / max(total, 0.0001);
}

// Animated fractal noise between two colors: paper, clouds, smoke or (with a small scale and
// one octave) film grain. Noise is in pixels, so it does not stretch with the node.
@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let p = in.uv * in.size / max(material.scale, 0.5);
    let drift = vec2<f32>(globals.time * material.speed, globals.time * material.speed * 0.7);
    let n = fbm(p + drift, i32(clamp(material.octaves, 1.0, 8.0)));
    let color = mix(material.base, material.tint, clamp(n * material.intensity, 0.0, 1.0));
    return vec4<f32>(color.rgb, color.a * shape_coverage(in));
}
