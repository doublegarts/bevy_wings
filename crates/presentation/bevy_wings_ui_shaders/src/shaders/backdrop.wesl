// Backdrop blur passes for Wings Liquid Glass. Fullscreen fragment shaders (driven by Bevy's
// fullscreen vertex shader) implementing the dual Kawase filter: `downsample` halves the
// source while blurring, `blur` spreads it further at the same size.

struct FullscreenVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0)
var source: texture_2d<f32>;
@group(0) @binding(1)
var source_sampler: sampler;

fn texel() -> vec2<f32> {
    return 1.0 / vec2<f32>(textureDimensions(source));
}

@fragment
fn downsample(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let h = texel();
    var sum = textureSample(source, source_sampler, in.uv) * 4.0;
    sum += textureSample(source, source_sampler, in.uv - h);
    sum += textureSample(source, source_sampler, in.uv + h);
    sum += textureSample(source, source_sampler, in.uv + vec2<f32>(h.x, -h.y));
    sum += textureSample(source, source_sampler, in.uv - vec2<f32>(h.x, -h.y));
    return vec4<f32>((sum / 8.0).rgb, 1.0);
}

@fragment
fn blur(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let h = texel() * 1.5;
    var sum = textureSample(source, source_sampler, in.uv + vec2<f32>(-h.x * 2.0, 0.0));
    sum += textureSample(source, source_sampler, in.uv + vec2<f32>(-h.x, h.y)) * 2.0;
    sum += textureSample(source, source_sampler, in.uv + vec2<f32>(0.0, h.y * 2.0));
    sum += textureSample(source, source_sampler, in.uv + vec2<f32>(h.x, h.y)) * 2.0;
    sum += textureSample(source, source_sampler, in.uv + vec2<f32>(h.x * 2.0, 0.0));
    sum += textureSample(source, source_sampler, in.uv + vec2<f32>(h.x, -h.y)) * 2.0;
    sum += textureSample(source, source_sampler, in.uv + vec2<f32>(0.0, -h.y * 2.0));
    sum += textureSample(source, source_sampler, in.uv + vec2<f32>(-h.x, -h.y)) * 2.0;
    return vec4<f32>((sum / 12.0).rgb, 1.0);
}
