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


struct GradientBorderMaterial {
    start: vec4<f32>,
    end: vec4<f32>,
    fill: vec4<f32>,
    width: f32,
    angle: f32,
    speed: f32,
    padding: f32,
};

@group(1) @binding(0)
var<uniform> material: GradientBorderMaterial;

const TAU: f32 = 6.28318530718;

// A border that runs around the rounded node shape with an angular gradient. The gradient
// mirrors around the ring so it has no seam, and turns when speed is not zero.
@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let p = (in.uv - vec2<f32>(0.5)) * in.size;
    let d = sd_rounded_box(p, in.size, in.border_radius_x);
    let outer = clamp(0.5 - d, 0.0, 1.0);
    // The node's own border width (logical pixels, already scaled) unless one is given.
    let node_border = max(max(in.border_widths.x, in.border_widths.y), max(in.border_widths.z, in.border_widths.w));
    let width = select(node_border, material.width, material.width > 0.0);
    let inner = clamp(0.5 - (d + width), 0.0, 1.0);
    let ring = max(outer - inner, 0.0);
    let turn = material.angle + globals.time * material.speed * TAU;
    let t = fract((atan2(p.y, p.x) + turn) / TAU);
    let border = mix(material.start, material.end, 1.0 - abs(2.0 * t - 1.0));
    let fill_a = material.fill.a * inner;
    let border_a = border.a * ring;
    let alpha = fill_a + border_a;
    let rgb = (material.fill.rgb * fill_a + border.rgb * border_a) / max(alpha, 0.0001);
    return vec4<f32>(rgb, alpha);
}
