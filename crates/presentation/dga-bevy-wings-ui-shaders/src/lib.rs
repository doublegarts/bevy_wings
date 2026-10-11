//! # dga-bevy-wings-ui-shaders
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

use bevy::asset::embedded_asset;
use bevy::color::{Color, ColorToComponents, LinearRgba};
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;
use bevy::ui_render::UiMaterialPlugin;
use bevy::ui_render::ui_material::UiMaterial;

/// Embedded path of a Wings shader.
macro_rules! shader_path {
    ($file:literal) => {
        concat!("embedded://dga_bevy_wings_ui_shaders/shaders/", $file)
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
    /// All sources with their file names.
    pub const ALL: [(&str, &str); 5] = [
        ("ripple.wesl", RIPPLE),
        ("shimmer.wesl", SHIMMER),
        ("circular_progress.wesl", CIRCULAR_PROGRESS),
        ("aurora.wesl", AURORA),
        ("glass.wesl", GLASS),
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
        app.add_plugins((
            UiMaterialPlugin::<WingsRippleMaterial>::default(),
            UiMaterialPlugin::<WingsShimmerMaterial>::default(),
            UiMaterialPlugin::<WingsCircularProgressMaterial>::default(),
            UiMaterialPlugin::<WingsAuroraMaterial>::default(),
            UiMaterialPlugin::<WingsGlassMaterial>::default(),
        ));
    }
}

/// Commonly used items.
pub mod prelude {
    pub use crate::{
        WingsAuroraMaterial, WingsCircularProgressMaterial, WingsGlassMaterial,
        WingsRippleMaterial, WingsShadersPlugin, WingsShimmerMaterial,
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
                    "embedded://dga_bevy_wings_ui_shaders/shaders/glass.wesl"
                )
            }
            _ => panic!("expected a path"),
        }
    }
}
