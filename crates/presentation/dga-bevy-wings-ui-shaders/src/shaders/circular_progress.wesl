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

struct CircularProgressMaterial {
    track: vec4<f32>,
    indicator: vec4<f32>,
    progress: f32,
    thickness: f32,
    padding_a: f32,
    padding_b: f32,
};

@group(1) @binding(0)
var<uniform> material: CircularProgressMaterial;

const TAU: f32 = 6.28318530718;

// A ring with a progress arc. Negative progress means indeterminate: a rotating arc whose
// length breathes over time.
@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let p = (in.uv - vec2<f32>(0.5)) * in.size;
    let outer = 0.5 * min(in.size.x, in.size.y);
    let inner = outer - material.thickness;
    let r = length(p);
    let ring = clamp(outer - r + 0.5, 0.0, 1.0) * clamp(r - inner + 0.5, 0.0, 1.0);
    // Angle measured clockwise from 12 o'clock, in 0..1.
    let angle = fract(atan2(p.x, -p.y) / TAU + 1.0);
    var start = 0.0;
    var sweep = clamp(material.progress, 0.0, 1.0);
    if material.progress < 0.0 {
        let t = globals.time;
        start = fract(t * 0.8);
        sweep = 0.1 + 0.6 * (0.5 + 0.5 * sin(t * 3.0));
    }
    let rel = fract(angle - start + 1.0);
    let on_arc = select(0.0, 1.0, rel <= sweep);
    let color = mix(material.track, material.indicator, on_arc);
    return vec4<f32>(color.rgb, color.a * ring);
}
