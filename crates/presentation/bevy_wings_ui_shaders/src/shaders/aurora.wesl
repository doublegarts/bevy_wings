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

struct AuroraMaterial {
    color_a: vec4<f32>,
    color_b: vec4<f32>,
    color_c: vec4<f32>,
    speed: f32,
    intensity: f32,
    padding_a: f32,
    padding_b: f32,
};

@group(1) @binding(0)
var<uniform> material: AuroraMaterial;

// Slowly moving three color gradient blobs: an animated hero or app background.
@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let t = globals.time * material.speed;
    let uv = in.uv;
    let p1 = vec2<f32>(0.3 + 0.2 * sin(t * 0.7), 0.3 + 0.2 * cos(t * 0.9));
    let p2 = vec2<f32>(0.7 + 0.2 * cos(t * 0.5), 0.6 + 0.2 * sin(t * 0.8));
    let w1 = exp(-6.0 * distance(uv, p1) * distance(uv, p1) / max(material.intensity, 0.05));
    let w2 = exp(-6.0 * distance(uv, p2) * distance(uv, p2) / max(material.intensity, 0.05));
    var color = material.color_c;
    color = mix(color, material.color_a, clamp(w1, 0.0, 1.0));
    color = mix(color, material.color_b, clamp(w2, 0.0, 1.0));
    return vec4<f32>(color.rgb, color.a * shape_coverage(in));
}
