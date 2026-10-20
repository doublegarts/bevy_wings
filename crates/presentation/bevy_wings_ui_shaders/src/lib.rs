//! # bevy_wings_ui_shaders
//!
//! Custom UI materials written in WESL for Bevy 0.20's `UiMaterial` pipeline:
//!
//! | Material | Effect |
//! |---|---|
//! | [`WingsRippleMaterial`] | Expanding touch ripple clipped to rounded corners |
//! | [`WingsShimmerMaterial`] | Skeleton loading placeholder with a sweeping highlight |
//! | [`WingsCircularProgressMaterial`] | Determinate or indeterminate progress ring |
//! | [`WingsAuroraMaterial`] | Animated three color gradient background |
//! | [`WingsGlassMaterial`] | Frosted glass tint with grain and an edge highlight |
//! | [`WingsLiquidGlassMaterial`] | Liquid Glass: the blurred backdrop with refraction and a rim light |
//! | [`WingsGradientBorderMaterial`] | Border with an angular gradient, optionally turning |
//! | [`WingsNoiseMaterial`] | Animated fractal noise: paper, clouds, smoke or film grain |
//! | [`WingsScrollEdgeMaterial`] | iOS 26 soft scroll edge: progressive blur that fades out under bars |
//!
//! Every shader is embedded in the binary (no asset files to ship), respects the node's
//! border radius with an anti aliased signed distance field, and animated ones read Bevy's
//! global time uniform, so animating them costs no CPU work per frame.
//!
//! ```ignore
//! fn spawn_loader(mut commands: Commands, mut materials: ResMut<Assets<WingsCircularProgressMaterial>>) {
//!     commands.spawn((
//!         Node { width: px(48), height: px(48), ..default() },
//!         MaterialNode(materials.add(WingsCircularProgressMaterial::indeterminate(Color::WHITE))),
//!     ));
//! }
//! ```

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/doublegarts/bevy_wings/main/assets/branding/bevy-wings-icon.png",
    html_favicon_url = "https://raw.githubusercontent.com/doublegarts/bevy_wings/main/assets/branding/bevy-wings-icon.png"
)]

use bevy::asset::embedded_asset;
use bevy::color::{Color, ColorToComponents, LinearRgba};
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;
use bevy::ui_render::UiMaterialPlugin;
use bevy::ui_render::ui_material::UiMaterial;

pub mod backdrop;

/// Embedded path of a Wings shader.
macro_rules! shader_path {
    ($file:literal) => {
        concat!("embedded://bevy_wings_ui_shaders/shaders/", $file)
    };
}

/// WESL sources, exposed for tooling and tests.
pub mod sources {
    /// Ripple shader.
    pub const RIPPLE: &str = include_str!("shaders/ripple.wesl");
    /// Shimmer shader.
    pub const SHIMMER: &str = include_str!("shaders/shimmer.wesl");
    /// Circular progress shader.
    pub const CIRCULAR_PROGRESS: &str = include_str!("shaders/circular_progress.wesl");
    /// Aurora shader.
    pub const AURORA: &str = include_str!("shaders/aurora.wesl");
    /// Glass shader.
    pub const GLASS: &str = include_str!("shaders/glass.wesl");
    /// Liquid Glass shader.
    pub const LIQUID_GLASS: &str = include_str!("shaders/liquid_glass.wesl");
    /// Gradient border shader.
    pub const GRADIENT_BORDER: &str = include_str!("shaders/gradient_border.wesl");
    /// Noise shader.
    pub const NOISE: &str = include_str!("shaders/noise.wesl");
    /// Scroll edge shader.
    pub const SCROLL_EDGE: &str = include_str!("shaders/scroll_edge.wesl");
    /// Backdrop blur passes (fullscreen, entry points `downsample` and `blur`).
    pub const BACKDROP: &str = include_str!("shaders/backdrop.wesl");
    /// All UI material sources with their file names.
    pub const ALL: [(&str, &str); 9] = [
        ("ripple.wesl", RIPPLE),
        ("shimmer.wesl", SHIMMER),
        ("circular_progress.wesl", CIRCULAR_PROGRESS),
        ("aurora.wesl", AURORA),
        ("glass.wesl", GLASS),
        ("liquid_glass.wesl", LIQUID_GLASS),
        ("gradient_border.wesl", GRADIENT_BORDER),
        ("noise.wesl", NOISE),
        ("scroll_edge.wesl", SCROLL_EDGE),
    ];
}

fn lin(c: Color) -> LinearRgba {
    c.to_linear()
}

/// Touch ripple.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone, PartialEq)]
pub struct WingsRippleMaterial {
    /// Ripple color (alpha included).
    #[uniform(0)]
    pub color: LinearRgba,
    /// Ripple origin in node UV space (`0..1`).
    #[uniform(0)]
    pub center: Vec2,
    /// Expansion progress `0..1`.
    #[uniform(0)]
    pub progress: f32,
    /// Fade multiplier `0..1`.
    #[uniform(0)]
    pub alpha: f32,
}

impl Default for WingsRippleMaterial {
    fn default() -> Self {
        Self::new(Color::WHITE.with_alpha(0.24), Vec2::splat(0.5))
    }
}

impl WingsRippleMaterial {
    /// A ripple of `color` starting at `center` (UV space).
    pub fn new(color: Color, center: Vec2) -> Self {
        Self {
            color: lin(color),
            center,
            progress: 0.0,
            alpha: 1.0,
        }
    }

    /// Converts a picking hit position (`-0.5..0.5`, origin at the node center) to UV space.
    pub fn center_from_hit(hit: Vec2) -> Vec2 {
        (hit + Vec2::splat(0.5)).clamp(Vec2::ZERO, Vec2::ONE)
    }

    /// Radius in pixels reached at `progress` for a node of `size` with the ripple at `center`.
    /// Mirrors the shader so CPU side logic (and tests) can reason about coverage.
    pub fn radius_at(size: Vec2, center: Vec2, progress: f32) -> f32 {
        let c = center * size;
        let far = [
            Vec2::ZERO,
            Vec2::new(size.x, 0.0),
            size,
            Vec2::new(0.0, size.y),
        ]
        .into_iter()
        .map(|corner| c.distance(corner))
        .fold(0.0f32, f32::max);
        let eased = 1.0 - (1.0 - progress.clamp(0.0, 1.0)).powi(3);
        far * eased
    }
}

impl UiMaterial for WingsRippleMaterial {
    fn fragment_shader() -> ShaderRef {
        shader_path!("ripple.wesl").into()
    }
}

/// Skeleton loading shimmer.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone, PartialEq)]
pub struct WingsShimmerMaterial {
    /// Base placeholder color.
    #[uniform(0)]
    pub base: LinearRgba,
    /// Highlight color.
    #[uniform(0)]
    pub highlight: LinearRgba,
    /// Sweeps per second.
    #[uniform(0)]
    pub speed: f32,
    /// Highlight band width in UV units.
    #[uniform(0)]
    pub width: f32,
    /// Sweep direction in radians.
    #[uniform(0)]
    pub angle: f32,
    /// Padding (keeps the uniform 16 byte aligned).
    #[uniform(0)]
    pub padding: f32,
}

impl Default for WingsShimmerMaterial {
    fn default() -> Self {
        Self::new(Color::srgb(0.88, 0.88, 0.9), Color::srgb(0.97, 0.97, 0.98))
    }
}

impl WingsShimmerMaterial {
    /// A shimmer between two colors.
    pub fn new(base: Color, highlight: Color) -> Self {
        Self {
            base: lin(base),
            highlight: lin(highlight),
            speed: 0.8,
            width: 0.25,
            angle: 0.35,
            padding: 0.0,
        }
    }
}

impl UiMaterial for WingsShimmerMaterial {
    fn fragment_shader() -> ShaderRef {
        shader_path!("shimmer.wesl").into()
    }
}

/// Progress ring.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone, PartialEq)]
pub struct WingsCircularProgressMaterial {
    /// Track (background ring) color.
    #[uniform(0)]
    pub track: LinearRgba,
    /// Indicator (arc) color.
    #[uniform(0)]
    pub indicator: LinearRgba,
    /// Progress `0..1`, or negative for indeterminate.
    #[uniform(0)]
    pub progress: f32,
    /// Ring thickness in pixels.
    #[uniform(0)]
    pub thickness: f32,
    /// Padding.
    #[uniform(0)]
    pub padding_a: f32,
    /// Padding.
    #[uniform(0)]
    pub padding_b: f32,
}

impl Default for WingsCircularProgressMaterial {
    fn default() -> Self {
        Self::indeterminate(Color::srgb(0.24, 0.35, 0.99))
    }
}

impl WingsCircularProgressMaterial {
    /// A determinate ring.
    pub fn determinate(indicator: Color, progress: f32) -> Self {
        Self {
            track: lin(indicator.with_alpha(0.2)),
            indicator: lin(indicator),
            progress: progress.clamp(0.0, 1.0),
            thickness: 4.0,
            padding_a: 0.0,
            padding_b: 0.0,
        }
    }

    /// An indeterminate, spinning ring.
    pub fn indeterminate(indicator: Color) -> Self {
        Self {
            progress: -1.0,
            ..Self::determinate(indicator, 0.0)
        }
    }

    /// True when the ring spins.
    pub fn is_indeterminate(&self) -> bool {
        self.progress < 0.0
    }

    /// Sets the progress (clamped) and switches to determinate mode.
    pub fn set_progress(&mut self, progress: f32) {
        self.progress = progress.clamp(0.0, 1.0);
    }
}

impl UiMaterial for WingsCircularProgressMaterial {
    fn fragment_shader() -> ShaderRef {
        shader_path!("circular_progress.wesl").into()
    }
}

/// Animated gradient background.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone, PartialEq)]
pub struct WingsAuroraMaterial {
    /// First blob color.
    #[uniform(0)]
    pub color_a: LinearRgba,
    /// Second blob color.
    #[uniform(0)]
    pub color_b: LinearRgba,
    /// Base color.
    #[uniform(0)]
    pub color_c: LinearRgba,
    /// Animation speed.
    #[uniform(0)]
    pub speed: f32,
    /// Blob size.
    #[uniform(0)]
    pub intensity: f32,
    /// Padding.
    #[uniform(0)]
    pub padding_a: f32,
    /// Padding.
    #[uniform(0)]
    pub padding_b: f32,
}

impl Default for WingsAuroraMaterial {
    fn default() -> Self {
        Self::new(
            Color::srgb(0.42, 0.33, 0.98),
            Color::srgb(0.1, 0.75, 0.85),
            Color::srgb(0.05, 0.06, 0.12),
        )
    }
}

impl WingsAuroraMaterial {
    /// An aurora with two blob colors over a base color.
    pub fn new(a: Color, b: Color, base: Color) -> Self {
        Self {
            color_a: lin(a),
            color_b: lin(b),
            color_c: lin(base),
            speed: 0.4,
            intensity: 0.5,
            padding_a: 0.0,
            padding_b: 0.0,
        }
    }
}

impl UiMaterial for WingsAuroraMaterial {
    fn fragment_shader() -> ShaderRef {
        shader_path!("aurora.wesl").into()
    }
}

/// Frosted glass.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone, PartialEq)]
pub struct WingsGlassMaterial {
    /// Tint color with alpha (translucency).
    #[uniform(0)]
    pub tint: LinearRgba,
    /// Edge highlight color with strength in alpha.
    #[uniform(0)]
    pub edge: LinearRgba,
    /// Grain amount.
    #[uniform(0)]
    pub grain: f32,
    /// Edge highlight width in pixels.
    #[uniform(0)]
    pub edge_width: f32,
    /// Padding.
    #[uniform(0)]
    pub padding_a: f32,
    /// Padding.
    #[uniform(0)]
    pub padding_b: f32,
}

impl Default for WingsGlassMaterial {
    fn default() -> Self {
        Self::new(Color::WHITE.with_alpha(0.14))
    }
}

impl WingsGlassMaterial {
    /// Glass with a tint.
    pub fn new(tint: Color) -> Self {
        Self {
            tint: lin(tint),
            edge: lin(Color::WHITE.with_alpha(0.5)),
            grain: 0.04,
            edge_width: 1.5,
            padding_a: 0.0,
            padding_b: 0.0,
        }
    }
}

impl UiMaterial for WingsGlassMaterial {
    fn fragment_shader() -> ShaderRef {
        shader_path!("glass.wesl").into()
    }
}

/// Liquid Glass: the blurred backdrop behind the node, bent at the rim like a lens, tinted and
/// lit. Only meaningful on the Wings overlay layer (see [`backdrop`]); elsewhere use a
/// translucent fill.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone, PartialEq)]
pub struct WingsLiquidGlassMaterial {
    /// Tint color, strength in alpha.
    #[uniform(0)]
    pub tint: LinearRgba,
    /// Rim light color, strength in alpha.
    #[uniform(0)]
    pub highlight: LinearRgba,
    /// `0` for clear glass, `1` for regular (frosted) glass.
    #[uniform(0)]
    pub frost: f32,
    /// How far the rim bends the backdrop, in physical pixels.
    #[uniform(0)]
    pub refraction: f32,
    /// Width of the lens band along the rim, in physical pixels.
    #[uniform(0)]
    pub edge: f32,
    /// `0..1` glow of interactive glass while pressed.
    #[uniform(0)]
    pub press: f32,
    /// Half resolution backdrop.
    #[texture(1)]
    #[sampler(2)]
    pub light: Handle<Image>,
    /// Quarter resolution, blurred backdrop.
    #[texture(3)]
    #[sampler(4)]
    pub heavy: Handle<Image>,
}

impl WingsLiquidGlassMaterial {
    /// Regular glass over the given backdrop, tinted for a light or dark appearance.
    pub fn regular(backdrop: &backdrop::WingsBackdrop, dark: bool) -> Self {
        let (tint, highlight) = if dark {
            (
                Color::srgba(0.08, 0.08, 0.09, 0.42),
                Color::srgba(1.0, 1.0, 1.0, 0.22),
            )
        } else {
            (
                Color::srgba(1.0, 1.0, 1.0, 0.55),
                Color::srgba(1.0, 1.0, 1.0, 0.9),
            )
        };
        Self {
            tint: lin(tint),
            highlight: lin(highlight),
            frost: 1.0,
            refraction: 10.0,
            edge: 18.0,
            press: 0.0,
            light: backdrop.light.clone(),
            heavy: backdrop.heavy.clone(),
        }
    }

    /// Clear glass: little blur and tint, strong lensing.
    pub fn clear(backdrop: &backdrop::WingsBackdrop, dark: bool) -> Self {
        let mut m = Self::regular(backdrop, dark);
        m.frost = 0.15;
        m.tint = lin(if dark {
            Color::srgba(0.0, 0.0, 0.0, 0.18)
        } else {
            Color::srgba(1.0, 1.0, 1.0, 0.12)
        });
        m.refraction = 16.0;
        m
    }

    /// Glass tinted with a color (prominent glass buttons).
    pub fn tinted(backdrop: &backdrop::WingsBackdrop, color: Color, dark: bool) -> Self {
        let mut m = Self::regular(backdrop, dark);
        m.tint = lin(color.with_alpha(0.82));
        m.highlight = lin(Color::srgba(1.0, 1.0, 1.0, 0.45));
        m
    }
}

impl UiMaterial for WingsLiquidGlassMaterial {
    fn fragment_shader() -> ShaderRef {
        shader_path!("liquid_glass.wesl").into()
    }
}

/// A border with an angular gradient around the node's rounded shape.
///
/// The gradient runs from `start` to `end` and back around the ring, so it has no seam; a
/// non zero `speed` turns it (turns per second), like the glowing outline of a selected or
/// "AI" control. The inside is filled with `fill`, transparent by default.
///
/// The ring is as wide as the node's `Node::border`, so it scales with the display:
///
/// ```ignore
/// commands.spawn((
///     Node { border: UiRect::all(px(2)), border_radius: BorderRadius::all(px(16)), ..default() },
///     MaterialNode(materials.add(WingsGradientBorderMaterial::new(start, end).turning(0.25))),
/// ));
/// ```
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone, PartialEq)]
pub struct WingsGradientBorderMaterial {
    /// Gradient color at the start angle.
    #[uniform(0)]
    pub start: LinearRgba,
    /// Gradient color half way around.
    #[uniform(0)]
    pub end: LinearRgba,
    /// Fill inside the border.
    #[uniform(0)]
    pub fill: LinearRgba,
    /// Border width in physical pixels; 0 uses the node's `Node::border` width.
    #[uniform(0)]
    pub width: f32,
    /// Start angle in radians.
    #[uniform(0)]
    pub angle: f32,
    /// Turns per second (0 for a still gradient).
    #[uniform(0)]
    pub speed: f32,
    /// Padding (keeps the uniform 16 byte aligned).
    #[uniform(0)]
    pub padding: f32,
}

impl Default for WingsGradientBorderMaterial {
    fn default() -> Self {
        Self::new(Color::srgb(0.25, 0.5, 1.0), Color::srgb(0.85, 0.3, 0.95))
    }
}

impl WingsGradientBorderMaterial {
    /// A still border from `start` to `end`, as wide as the node's border.
    pub fn new(start: Color, end: Color) -> Self {
        Self {
            start: lin(start),
            end: lin(end),
            fill: LinearRgba::NONE,
            width: 0.0,
            angle: 0.0,
            speed: 0.0,
            padding: 0.0,
        }
    }

    /// Fills the inside.
    pub fn with_fill(mut self, fill: Color) -> Self {
        self.fill = lin(fill);
        self
    }

    /// Turns the gradient at `turns_per_second`.
    pub fn turning(mut self, turns_per_second: f32) -> Self {
        self.speed = turns_per_second;
        self
    }
}

impl UiMaterial for WingsGradientBorderMaterial {
    fn fragment_shader() -> ShaderRef {
        shader_path!("gradient_border.wesl").into()
    }
}

/// Animated fractal noise between two colors.
///
/// Large `scale` and several octaves give clouds or smoke; a scale of 1 to 2 px with one octave
/// and a low `intensity` gives film grain for flat backgrounds.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone, PartialEq)]
pub struct WingsNoiseMaterial {
    /// Color where the noise is 0.
    #[uniform(0)]
    pub base: LinearRgba,
    /// Color where the noise is 1.
    #[uniform(0)]
    pub tint: LinearRgba,
    /// Size of the noise features in physical pixels.
    #[uniform(0)]
    pub scale: f32,
    /// Drift in noise cells per second (0 for still noise).
    #[uniform(0)]
    pub speed: f32,
    /// How far the noise moves from `base` toward `tint`.
    #[uniform(0)]
    pub intensity: f32,
    /// Octaves (1 to 8).
    #[uniform(0)]
    pub octaves: f32,
}

impl Default for WingsNoiseMaterial {
    fn default() -> Self {
        Self::clouds(Color::srgb(0.1, 0.12, 0.2), Color::srgb(0.45, 0.55, 0.9))
    }
}

impl WingsNoiseMaterial {
    /// Slowly drifting clouds from `base` to `tint`.
    pub fn clouds(base: Color, tint: Color) -> Self {
        Self {
            base: lin(base),
            tint: lin(tint),
            scale: 96.0,
            speed: 0.08,
            intensity: 1.0,
            octaves: 5.0,
        }
    }

    /// Still, fine grain over `base` (a `tint` with low alpha keeps it subtle).
    pub fn grain(base: Color, tint: Color) -> Self {
        Self {
            base: lin(base),
            tint: lin(tint),
            scale: 1.5,
            speed: 0.0,
            intensity: 1.0,
            octaves: 1.0,
        }
    }
}

impl UiMaterial for WingsNoiseMaterial {
    fn fragment_shader() -> ShaderRef {
        shader_path!("noise.wesl").into()
    }
}

/// The iOS 26 "soft" scroll edge effect under a bar: the content behind blurs progressively
/// toward the top of the screen, takes on the background color and fades out toward the bar's
/// bottom edge, so the bar has no visible edge. Samples the same backdrop as
/// [`WingsLiquidGlassMaterial`], so the node must be on the overlay layer.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone, PartialEq)]
pub struct WingsScrollEdgeMaterial {
    /// Background color, strength in alpha.
    #[uniform(0)]
    pub tint: LinearRgba,
    /// Share of the height (from the top) that stays fully covered before fading out.
    #[uniform(0)]
    pub fade_start: f32,
    /// `0..1` blur strength at the top.
    #[uniform(0)]
    pub blur: f32,
    /// `0..1` overall strength: how much content has scrolled under the bar.
    #[uniform(0)]
    pub strength: f32,
    /// Padding.
    #[uniform(0)]
    pub padding: f32,
    /// Half resolution backdrop.
    #[texture(1)]
    #[sampler(2)]
    pub light: Handle<Image>,
    /// Quarter resolution, blurred backdrop.
    #[texture(3)]
    #[sampler(4)]
    pub heavy: Handle<Image>,
}

impl WingsScrollEdgeMaterial {
    /// The soft edge over `backdrop`, settling into `background`.
    pub fn soft(backdrop: &backdrop::WingsBackdrop, background: Color, dark: bool) -> Self {
        Self {
            tint: lin(background.with_alpha(if dark { 0.72 } else { 0.8 })),
            fade_start: 0.72,
            blur: 1.0,
            strength: 1.0,
            padding: 0.0,
            light: backdrop.light.clone(),
            heavy: backdrop.heavy.clone(),
        }
    }
}

impl UiMaterial for WingsScrollEdgeMaterial {
    fn fragment_shader() -> ShaderRef {
        shader_path!("scroll_edge.wesl").into()
    }
}

/// Converts a color to the `[f32; 4]` linear representation used by the uniforms.
pub fn linear_array(c: Color) -> [f32; 4] {
    c.to_linear().to_f32_array()
}

/// Registers every Wings material and embeds their shaders.
#[derive(Default)]
pub struct WingsShadersPlugin;

impl Plugin for WingsShadersPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "shaders/ripple.wesl");
        embedded_asset!(app, "shaders/shimmer.wesl");
        embedded_asset!(app, "shaders/circular_progress.wesl");
        embedded_asset!(app, "shaders/aurora.wesl");
        embedded_asset!(app, "shaders/glass.wesl");
        embedded_asset!(app, "shaders/liquid_glass.wesl");
        embedded_asset!(app, "shaders/gradient_border.wesl");
        embedded_asset!(app, "shaders/noise.wesl");
        embedded_asset!(app, "shaders/scroll_edge.wesl");
        app.add_plugins((
            UiMaterialPlugin::<WingsRippleMaterial>::default(),
            UiMaterialPlugin::<WingsShimmerMaterial>::default(),
            UiMaterialPlugin::<WingsCircularProgressMaterial>::default(),
            UiMaterialPlugin::<WingsAuroraMaterial>::default(),
            UiMaterialPlugin::<WingsGlassMaterial>::default(),
            UiMaterialPlugin::<WingsLiquidGlassMaterial>::default(),
            UiMaterialPlugin::<WingsGradientBorderMaterial>::default(),
            UiMaterialPlugin::<WingsNoiseMaterial>::default(),
            UiMaterialPlugin::<WingsScrollEdgeMaterial>::default(),
            backdrop::WingsBackdropPlugin,
        ));
    }
}

/// Commonly used items.
pub mod prelude {
    pub use crate::backdrop::{WingsBackdrop, WingsBackdropCamera};
    pub use crate::{
        WingsAuroraMaterial, WingsCircularProgressMaterial, WingsGlassMaterial,
        WingsGradientBorderMaterial, WingsLiquidGlassMaterial, WingsNoiseMaterial,
        WingsRippleMaterial, WingsScrollEdgeMaterial, WingsShadersPlugin, WingsShimmerMaterial,
    };
    pub use bevy::ui_render::prelude::MaterialNode;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(name: &str, src: &str) -> naga::Module {
        naga::front::wgsl::parse_str(src)
            .unwrap_or_else(|e| panic!("{name}: {}", e.emit_to_string(src)))
    }

    /// Every embedded shader parses and passes full naga validation.
    #[test]
    fn shaders_parse_and_validate() {
        for (name, src) in sources::ALL {
            let module = parse(name, src);
            naga::valid::Validator::new(
                naga::valid::ValidationFlags::all(),
                naga::valid::Capabilities::all(),
            )
            .validate(&module)
            .unwrap_or_else(|e| panic!("{name}: {e:?}"));
            assert!(
                module.entry_points.iter().any(|e| e.name == "fragment"),
                "{name} has a fragment entry point"
            );
        }
    }

    /// Global time bindings must fit Bevy's actual buffer, not an assumed padded layout.
    /// Desktop Bevy binds 12 bytes; declaring an extra padding field invalidates the pipeline.
    #[test]
    fn globals_fit_bevy_pipeline_binding() {
        use bevy::render::globals::GlobalsUniform;
        use bevy::render::render_resource::ShaderType;

        for (name, src) in sources::ALL {
            let module = parse(name, src);
            let (_, globals) = module
                .global_variables
                .iter()
                .find(|(_, variable)| variable.name.as_deref() == Some("globals"))
                .unwrap();
            assert_eq!(
                globals.binding,
                Some(naga::ResourceBinding {
                    group: 0,
                    binding: 1
                }),
                "{name}"
            );
            let mut layouter = naga::proc::Layouter::default();
            layouter.update(module.to_ctx()).unwrap();
            assert!(
                u64::from(layouter[globals.ty].size) <= GlobalsUniform::min_size().get(),
                "{name}: globals exceeds Bevy's binding size"
            );
        }
    }

    /// The WGSL material structs have the size implied by the Rust uniform fields, so the
    /// CPU and GPU layouts agree.
    #[test]
    fn uniform_sizes_match_rust_fields() {
        let expected = [
            ("ripple.wesl", "RippleMaterial", 32u32),
            ("shimmer.wesl", "ShimmerMaterial", 48),
            ("circular_progress.wesl", "CircularProgressMaterial", 48),
            ("aurora.wesl", "AuroraMaterial", 64),
            ("glass.wesl", "GlassMaterial", 48),
            ("liquid_glass.wesl", "LiquidGlassMaterial", 48),
            ("gradient_border.wesl", "GradientBorderMaterial", 64),
            ("noise.wesl", "NoiseMaterial", 48),
            ("scroll_edge.wesl", "ScrollEdgeMaterial", 32),
        ];
        for (file, struct_name, size) in expected {
            let src = sources::ALL.iter().find(|(n, _)| *n == file).unwrap().1;
            let module = parse(file, src);
            let mut layouter = naga::proc::Layouter::default();
            layouter.update(module.to_ctx()).unwrap();
            let (handle, _) = module
                .types
                .iter()
                .find(|(_, t)| t.name.as_deref() == Some(struct_name))
                .unwrap_or_else(|| panic!("{struct_name} in {file}"));
            assert_eq!(layouter[handle].size, size, "{struct_name}");
        }
    }

    /// The Rust structs carry the expected number of bytes of uniform data.
    #[test]
    fn rust_uniform_byte_counts() {
        assert_eq!(
            size_of::<LinearRgba>() + size_of::<Vec2>() + 2 * size_of::<f32>(),
            32
        );
        assert_eq!(2 * size_of::<LinearRgba>() + 4 * size_of::<f32>(), 48);
        assert_eq!(3 * size_of::<LinearRgba>() + 4 * size_of::<f32>(), 64);
    }

    /// The backdrop passes parse, validate and expose both entry points.
    #[test]
    fn backdrop_shader_validates() {
        let module = parse("backdrop.wesl", sources::BACKDROP);
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .unwrap();
        for entry in ["downsample", "blur"] {
            assert!(
                module.entry_points.iter().any(|e| e.name == entry),
                "{entry}"
            );
        }
    }

    /// Picking hits map to UV space.
    #[test]
    fn ripple_center_from_hit() {
        assert_eq!(
            WingsRippleMaterial::center_from_hit(Vec2::ZERO),
            Vec2::splat(0.5)
        );
        assert_eq!(
            WingsRippleMaterial::center_from_hit(Vec2::new(-0.5, 0.5)),
            Vec2::new(0.0, 1.0)
        );
        assert_eq!(
            WingsRippleMaterial::center_from_hit(Vec2::splat(9.0)),
            Vec2::ONE
        );
    }

    /// The ripple covers the farthest corner at full progress.
    #[test]
    fn ripple_radius_reaches_far_corner() {
        let size = Vec2::new(100.0, 40.0);
        let r = WingsRippleMaterial::radius_at(size, Vec2::ZERO, 1.0);
        assert!((r - size.length()).abs() < 1e-3);
        assert_eq!(WingsRippleMaterial::radius_at(size, Vec2::ZERO, 0.0), 0.0);
        let half = WingsRippleMaterial::radius_at(size, Vec2::splat(0.5), 0.5);
        assert!(
            half > 0.5 * (size * 0.5).length(),
            "ease out is ahead of linear"
        );
    }

    /// Progress ring modes.
    #[test]
    fn progress_modes() {
        let mut m = WingsCircularProgressMaterial::indeterminate(Color::WHITE);
        assert!(m.is_indeterminate());
        m.set_progress(1.5);
        assert!(!m.is_indeterminate());
        assert_eq!(m.progress, 1.0);
    }

    /// Shader paths point into the embedded asset source of this crate.
    #[test]
    fn shader_refs_are_embedded() {
        match WingsGlassMaterial::fragment_shader() {
            ShaderRef::Path(p) => {
                assert_eq!(
                    p.to_string(),
                    "embedded://bevy_wings_ui_shaders/shaders/glass.wesl"
                )
            }
            _ => panic!("expected a path"),
        }
    }
}
