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

struct ShimmerMaterial {
    base: vec4<f32>,
    highlight: vec4<f32>,
    speed: f32,
    width: f32,
    angle: f32,
    padding: f32,
};

@group(1) @binding(0)
var<uniform> material: ShimmerMaterial;

// Skeleton placeholder: a soft highlight band sweeping across the base color.
@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let dir = vec2<f32>(cos(material.angle), sin(material.angle));
    let t = fract(globals.time * material.speed);
    let pos = dot(in.uv - vec2<f32>(0.5), dir) + 0.5;
    let band = 1.0 - smoothstep(0.0, material.width, abs(pos - (t * 2.0 - 0.5)));
    let color = mix(material.base, material.highlight, band);
    return vec4<f32>(color.rgb, color.a * shape_coverage(in));
}
