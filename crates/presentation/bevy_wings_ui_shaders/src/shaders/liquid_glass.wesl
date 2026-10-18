// Wings Liquid Glass. Samples the blurred backdrop the content layer left behind, bends it
// near the edges like a lens, tints it and adds a rim light. Self contained: the UI vertex
// output and the globals uniform are declared with the exact Bevy 0.20 UI material layout.

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

struct LiquidGlassMaterial {
    tint: vec4<f32>,
    highlight: vec4<f32>,
    frost: f32,
    refraction: f32,
    edge: f32,
    press: f32,
};

@group(1) @binding(0)
var<uniform> material: LiquidGlassMaterial;
@group(1) @binding(1)
var light_texture: texture_2d<f32>;
@group(1) @binding(2)
var light_sampler: sampler;
@group(1) @binding(3)
var heavy_texture: texture_2d<f32>;
@group(1) @binding(4)
var heavy_sampler: sampler;

// Signed distance to a rounded box centered at the origin. Radii order: top left, top right,
// bottom right, bottom left.
fn sd_rounded_box(point: vec2<f32>, size: vec2<f32>, radii: vec4<f32>) -> f32 {
    let rs = select(radii.xy, radii.wz, 0.0 < point.y);
    let radius = select(rs.x, rs.y, 0.0 < point.x);
    let q = abs(point) - 0.5 * size + radius;
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - radius;
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let p = (in.uv - vec2<f32>(0.5)) * in.size;
    let radii = in.border_radius_x;
    let d = sd_rounded_box(p, in.size, radii);
    let coverage = clamp(0.5 - d, 0.0, 1.0);

    // Outward normal of the shape.
    let e = 1.0;
    let gx = sd_rounded_box(p + vec2<f32>(e, 0.0), in.size, radii)
        - sd_rounded_box(p - vec2<f32>(e, 0.0), in.size, radii);
    let gy = sd_rounded_box(p + vec2<f32>(0.0, e), in.size, radii)
        - sd_rounded_box(p - vec2<f32>(0.0, e), in.size, radii);
    let n = normalize(vec2<f32>(gx, gy) + vec2<f32>(1e-5, 1e-5));

    // Screen position in the backdrop (the light texture is half the window resolution).
    let screen = vec2<f32>(textureDimensions(light_texture)) * 2.0;
    // Lens: near the rim the backdrop is pulled inwards, magnifying it like a glass edge.
    let band = clamp(1.0 + d / max(material.edge, 1.0), 0.0, 1.0);
    let bend = band * band * material.refraction;
    let uv = (in.position.xy - n * bend) / screen;

    let clear = textureSample(light_texture, light_sampler, uv).rgb;
    let frosted = textureSample(heavy_texture, heavy_sampler, uv).rgb;
    var color = mix(clear, frosted, material.frost);

    // Glass keeps colors lively.
    let luma = dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
    color = mix(vec3<f32>(luma), color, 1.15);
    color = mix(color, material.tint.rgb, material.tint.a);

    // Rim light, brightest where the edge faces the light (top left).
    let rim = clamp(1.0 + d / 1.5, 0.0, 1.0);
    let facing = 0.55 + 0.45 * dot(n, normalize(vec2<f32>(-0.6, -0.8)));
    color += material.highlight.rgb * rim * facing * material.highlight.a;
    // A soft inner shade along the bottom edge gives the glass some depth.
    let shade = clamp(1.0 + d / 6.0, 0.0, 1.0) * max(n.y, 0.0) * 0.06;
    color -= vec3<f32>(shade);
    // Interactive glass glows while pressed.
    color += vec3<f32>(material.press * 0.08);
    _ = globals.time;
    return vec4<f32>(clamp(color, vec3<f32>(0.0), vec3<f32>(1.0)), coverage);
}
