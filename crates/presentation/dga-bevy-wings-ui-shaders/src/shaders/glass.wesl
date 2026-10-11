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

struct GlassMaterial {
    tint: vec4<f32>,
    edge: vec4<f32>,
    grain: f32,
    edge_width: f32,
    padding_a: f32,
    padding_b: f32,
};

@group(1) @binding(0)
var<uniform> material: GlassMaterial;

fn hash(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}

// Frosted glass look: translucent tint, film grain and a bright inner edge highlight.
@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let p = (in.uv - vec2<f32>(0.5)) * in.size;
    let d = sd_rounded_box(p, in.size, in.border_radius_x);
    let coverage = clamp(0.5 - d, 0.0, 1.0);
    let noise = (hash(floor(in.uv * in.size)) - 0.5) * material.grain;
    let rim = 1.0 - clamp(-d / max(material.edge_width, 0.001), 0.0, 1.0);
    // Highlight the top edge more than the bottom one, like light coming from above.
    let light = mix(1.0, 0.4, in.uv.y);
    var color = material.tint.rgb + vec3<f32>(noise);
    color = mix(color, material.edge.rgb, rim * material.edge.a * light);
    let alpha = clamp(material.tint.a + rim * material.edge.a * light * 0.5, 0.0, 1.0);
    return vec4<f32>(color, alpha * coverage);
}
