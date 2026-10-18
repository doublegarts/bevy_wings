//! The backdrop behind Liquid Glass.
//!
//! Glass needs to see what is behind it. Wings renders UI in two layers on the same window:
//! the app's camera draws the content, then an overlay camera (marked
//! [`WingsBackdropCamera`]) draws floating chrome on top. Before the overlay camera's UI pass,
//! [`capture_backdrop`] copies the finished content into two small textures:
//!
//! * `light`: half resolution, slightly blurred (clear glass),
//! * `heavy`: quarter resolution, strongly blurred (regular, frosted glass).
//!
//! [`WingsLiquidGlassMaterial`](crate::WingsLiquidGlassMaterial) samples them at the
//! fragment's screen position. The passes are a dual Kawase filter at reduced resolution, so
//! the whole effect costs three tiny fullscreen draws per frame.

use bevy::asset::{RenderAssetUsages, embedded_asset, load_embedded_asset};
use bevy::core_pipeline::FullscreenShader;
use bevy::core_pipeline::schedule::{Core2d, Core2dSystems};
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::binding_types::{sampler, texture_2d};
use bevy::render::render_resource::*;
use bevy::render::renderer::{RenderContext, RenderDevice, ViewQuery};
use bevy::render::sync_world::RenderEntity;
use bevy::render::texture::GpuImage;
use bevy::render::view::ViewTarget;
use bevy::render::{Extract, ExtractSchedule, RenderApp, RenderStartup};
use bevy::shader::Shader;
use bevy::window::{PrimaryWindow, Window};

/// Format of the backdrop textures.
pub const BACKDROP_FORMAT: TextureFormat = TextureFormat::Rgba8UnormSrgb;

/// The backdrop textures glass samples. Kept at half and quarter of the primary window's
/// physical size.
#[derive(Resource, Debug, Clone)]
pub struct WingsBackdrop {
    /// Half resolution, lightly blurred.
    pub light: Handle<Image>,
    /// Quarter resolution, strongly blurred.
    pub heavy: Handle<Image>,
    /// Quarter resolution scratch texture.
    pub scratch: Handle<Image>,
    /// Physical size of the light texture.
    pub size: UVec2,
}

/// Marks the camera whose UI pass may sample the backdrop (the Wings overlay camera). The
/// backdrop is captured from its window texture just before its UI is drawn.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct WingsBackdropCamera;

/// Render world marker of a backdrop capturing view.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct WingsBackdropView;

/// Sent (as a change of [`WingsBackdrop`]) when the textures were recreated; glass materials
/// rebind then.
fn backdrop_image(size: UVec2) -> Image {
    let mut image = Image::new_uninit(
        Extent3d {
            width: size.x.max(1),
            height: size.y.max(1),
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        BACKDROP_FORMAT,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;
    image.sampler = ImageSampler::linear();
    image
}

/// Backdrop texture sizes for a window size: `(light, heavy)`.
pub fn backdrop_sizes(window: UVec2) -> (UVec2, UVec2) {
    let light = (window / 2).max(UVec2::ONE);
    let heavy = (window / 4).max(UVec2::ONE);
    (light, heavy)
}

fn create_backdrop(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    commands.insert_resource(WingsBackdrop {
        light: images.add(backdrop_image(UVec2::ONE)),
        heavy: images.add(backdrop_image(UVec2::ONE)),
        scratch: images.add(backdrop_image(UVec2::ONE)),
        size: UVec2::ONE,
    });
}

/// Resizes the backdrop textures with the primary window.
pub fn resize_backdrop(
    windows: Query<&Window, With<PrimaryWindow>>,
    backdrop: Option<ResMut<WingsBackdrop>>,
    mut images: ResMut<Assets<Image>>,
) {
    let (Ok(window), Some(mut backdrop)) = (windows.single(), backdrop) else {
        return;
    };
    let size = UVec2::new(window.physical_width(), window.physical_height());
    let (light, heavy) = backdrop_sizes(size);
    if backdrop.size == light {
        return;
    }
    let _ = images.insert(&backdrop.light, backdrop_image(light));
    let _ = images.insert(&backdrop.heavy, backdrop_image(heavy));
    let _ = images.insert(&backdrop.scratch, backdrop_image(heavy));
    backdrop.size = light;
}

/// Pipelines of the backdrop passes.
#[derive(Resource)]
pub struct WingsBackdropPipelines {
    layout: BindGroupLayoutDescriptor,
    sampler: Sampler,
    downsample: CachedRenderPipelineId,
    blur: CachedRenderPipelineId,
}

fn init_backdrop_pipelines(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    fullscreen: Res<FullscreenShader>,
    asset_server: Res<AssetServer>,
    pipeline_cache: Res<PipelineCache>,
) {
    let layout = BindGroupLayoutDescriptor::new(
        "wings_backdrop_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
            ),
        ),
    );
    let sampler = render_device.create_sampler(&SamplerDescriptor {
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        address_mode_u: AddressMode::ClampToEdge,
        address_mode_v: AddressMode::ClampToEdge,
        ..default()
    });
    let shader: Handle<Shader> =
        load_embedded_asset!(asset_server.as_ref(), "shaders/backdrop.wesl");
    let pipeline = |entry: &'static str| RenderPipelineDescriptor {
        label: Some(format!("wings_backdrop_{entry}").into()),
        layout: vec![layout.clone()],
        vertex: fullscreen.to_vertex_state(),
        fragment: Some(FragmentState {
            shader: shader.clone(),
            entry_point: Some(entry.into()),
            targets: vec![Some(ColorTargetState {
                format: BACKDROP_FORMAT,
                blend: None,
                write_mask: ColorWrites::ALL,
            })],
            ..default()
        }),
        ..default()
    };
    let downsample = pipeline_cache.queue_render_pipeline(pipeline("downsample"));
    let blur = pipeline_cache.queue_render_pipeline(pipeline("blur"));
    commands.insert_resource(WingsBackdropPipelines {
        layout,
        sampler,
        downsample,
        blur,
    });
}

fn extract_backdrop(
    mut commands: Commands,
    backdrop: Extract<Option<Res<WingsBackdrop>>>,
    cameras: Extract<Query<RenderEntity, With<WingsBackdropCamera>>>,
) {
    if let Some(backdrop) = backdrop.as_ref() {
        commands.insert_resource((**backdrop).clone());
    }
    for camera in &cameras {
        commands.entity(camera).insert(WingsBackdropView);
    }
}

/// Copies and blurs the window texture into the backdrop before the overlay camera's UI.
pub fn capture_backdrop(
    view: ViewQuery<(&ViewTarget, &WingsBackdropView)>,
    backdrop: Option<Res<WingsBackdrop>>,
    pipelines: Option<Res<WingsBackdropPipelines>>,
    pipeline_cache: Res<PipelineCache>,
    images: Res<RenderAssets<GpuImage>>,
    mut ctx: RenderContext,
) {
    let (target, _) = view.into_inner();
    let (Some(backdrop), Some(pipelines)) = (backdrop, pipelines) else {
        return;
    };
    let (Some(downsample), Some(blur)) = (
        pipeline_cache.get_render_pipeline(pipelines.downsample),
        pipeline_cache.get_render_pipeline(pipelines.blur),
    ) else {
        return;
    };
    let (Some(light), Some(heavy), Some(scratch)) = (
        images.get(&backdrop.light),
        images.get(&backdrop.heavy),
        images.get(&backdrop.scratch),
    ) else {
        return;
    };
    let layout = pipeline_cache.get_bind_group_layout(&pipelines.layout);
    let passes = [
        (
            target.main_texture_view(),
            &light.texture_view,
            downsample,
            "wings_backdrop_light",
        ),
        (
            &light.texture_view,
            &scratch.texture_view,
            downsample,
            "wings_backdrop_down",
        ),
        (
            &scratch.texture_view,
            &heavy.texture_view,
            blur,
            "wings_backdrop_heavy",
        ),
    ];
    for (source, destination, pipeline, label) in passes {
        let bind_group = ctx.render_device().create_bind_group(
            Some(label),
            &layout,
            &BindGroupEntries::sequential((source, &pipelines.sampler)),
        );
        let mut pass = ctx
            .command_encoder()
            .begin_render_pass(&RenderPassDescriptor {
                label: Some(label),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: destination,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations::default(),
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

/// Adds the backdrop textures and the capture pass.
#[derive(Default)]
pub struct WingsBackdropPlugin;

impl Plugin for WingsBackdropPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "shaders/backdrop.wesl");
        app.add_systems(PreStartup, create_backdrop)
            .add_systems(PostUpdate, resize_backdrop);
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .add_systems(RenderStartup, init_backdrop_pipelines)
            .add_systems(ExtractSchedule, extract_backdrop)
            .add_systems(
                Core2d,
                capture_backdrop
                    .after(Core2dSystems::PostProcess)
                    .before(bevy::ui_render::ui_pass),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Backdrop textures are half and quarter of the window and never empty.
    #[test]
    fn sizes() {
        assert_eq!(
            backdrop_sizes(UVec2::new(2200, 1520)),
            (UVec2::new(1100, 760), UVec2::new(550, 380))
        );
        assert_eq!(backdrop_sizes(UVec2::new(1, 1)), (UVec2::ONE, UVec2::ONE));
    }

    /// Backdrop images can be rendered into and sampled.
    #[test]
    fn images_are_render_targets() {
        let image = backdrop_image(UVec2::new(10, 10));
        assert!(
            image
                .texture_descriptor
                .usage
                .contains(TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING)
        );
    }
}
