// Wings scroll edge (iOS 26 "soft" style). Content scrolling under a bar blurs progressively
// toward the top of the screen, takes on the background color and fades out toward the bar's
// bottom, so the bar has no visible edge.

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

struct ScrollEdgeMaterial {
    tint: vec4<f32>,
    fade_start: f32,
    blur: f32,
    strength: f32,
    padding: f32,
};

@group(1) @binding(0)
var<uniform> material: ScrollEdgeMaterial;
@group(1) @binding(1)
var light_texture: texture_2d<f32>;
@group(1) @binding(2)
var light_sampler: sampler;
@group(1) @binding(3)
var heavy_texture: texture_2d<f32>;
@group(1) @binding(4)
var heavy_sampler: sampler;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    // Screen position in the backdrop (the light texture is half the window resolution).
    let screen = vec2<f32>(textureDimensions(light_texture)) * 2.0;
    let uv = in.position.xy / screen;
    let t = clamp(in.uv.y, 0.0, 1.0);
    let clear = textureSample(light_texture, light_sampler, uv).rgb;
    let frosted = textureSample(heavy_texture, heavy_sampler, uv).rgb;
    // Fully blurred through the toolbar row, easing off near the bottom.
    var color = mix(clear, frosted, (1.0 - smoothstep(0.55, 1.0, t)) * material.blur);
    // The background color settles over the content, so titles and buttons stay readable.
    color = mix(color, material.tint.rgb, material.tint.a * (1.0 - 0.35 * t));
    // Fully covering above `fade_start`, then fading out with no visible bottom edge.
    // `strength` grows with the content scrolled under the bar, so the effect eases in.
    let alpha = (1.0 - smoothstep(material.fade_start, 1.0, t)) * material.strength;
    _ = globals.time;
    return vec4<f32>(clamp(color, vec3<f32>(0.0), vec3<f32>(1.0)), alpha);
}
