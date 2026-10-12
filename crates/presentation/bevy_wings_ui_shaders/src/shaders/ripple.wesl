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

struct RippleMaterial {
    color: vec4<f32>,
    center: vec2<f32>,
    progress: f32,
    alpha: f32,
};

@group(1) @binding(0)
var<uniform> material: RippleMaterial;

// An expanding, fading circle clipped to the node shape: touch feedback for buttons.
@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let p = in.uv * in.size;
    let c = material.center * in.size;
    // The ripple reaches the farthest corner at progress 1.
    let far = max(max(length(c), length(c - vec2<f32>(in.size.x, 0.0))),
                  max(length(c - in.size), length(c - vec2<f32>(0.0, in.size.y))));
    let eased = 1.0 - pow(1.0 - clamp(material.progress, 0.0, 1.0), 3.0);
    let radius = far * eased;
    let edge = clamp(radius - distance(p, c) + 1.0, 0.0, 1.0);
    let a = material.color.a * material.alpha * edge * shape_coverage(in);
    return vec4<f32>(material.color.rgb, a);
}
