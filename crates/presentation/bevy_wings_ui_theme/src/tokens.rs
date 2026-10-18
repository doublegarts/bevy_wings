//! Design tokens other than color: typography, shapes, spacing, elevation and motion.

use bevy::color::Color;
use bevy::text::{FontSource, FontWeight, GenericFontFamily};
use bevy::ui::{BorderRadius, BoxShadow, ShadowStyle, Val};

use bevy_wings_ui_animation::easing::WingsEasing;
use bevy_wings_ui_animation::spec::{AnimationSpec, SpringSpec};

use crate::design::WingsDesign;

/// One text style of the type scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WingsTextStyle {
    /// Font size in logical pixels.
    pub size: f32,
    /// Font weight.
    pub weight: FontWeight,
    /// Line height as a multiple of the font size.
    pub line_height: f32,
    /// Extra letter spacing in pixels.
    pub letter_spacing: f32,
}

impl WingsTextStyle {
    /// Creates a style.
    pub const fn new(size: f32, weight: FontWeight, line_height: f32, letter_spacing: f32) -> Self {
        Self {
            size,
            weight,
            line_height,
            letter_spacing,
        }
    }
}

/// A slot of the type scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TypeRole {
    /// Hero numbers and very short text.
    DisplayLarge,
    /// Large display text.
    DisplayMedium,
    /// Small display text.
    DisplaySmall,
    /// Page titles.
    HeadlineLarge,
    /// Section titles.
    HeadlineMedium,
    /// Small headlines.
    HeadlineSmall,
    /// Prominent titles (top bars).
    TitleLarge,
    /// Card titles.
    TitleMedium,
    /// Small titles.
    TitleSmall,
    /// Long form body text.
    BodyLarge,
    /// Default body text.
    #[default]
    BodyMedium,
    /// Captions.
    BodySmall,
    /// Prominent labels (buttons).
    LabelLarge,
    /// Labels.
    LabelMedium,
    /// Small labels (badges, chips).
    LabelSmall,
}

/// The type scale.
#[derive(Debug, Clone, PartialEq)]
pub struct WingsTypography {
    /// Font source shared by all roles. The default handle uses the embedded Inter font.
    pub font: FontSource,
    styles: [WingsTextStyle; 15],
}

impl Default for WingsTypography {
    fn default() -> Self {
        use FontWeight as W;
        Self {
            font: FontSource::default(),
            styles: [
                WingsTextStyle::new(57.0, W::NORMAL, 1.12, -0.25),
                WingsTextStyle::new(45.0, W::NORMAL, 1.16, 0.0),
                WingsTextStyle::new(36.0, W::NORMAL, 1.22, 0.0),
                WingsTextStyle::new(32.0, W::NORMAL, 1.25, 0.0),
                WingsTextStyle::new(28.0, W::NORMAL, 1.29, 0.0),
                WingsTextStyle::new(24.0, W::NORMAL, 1.33, 0.0),
                WingsTextStyle::new(22.0, W::NORMAL, 1.27, 0.0),
                WingsTextStyle::new(16.0, W::MEDIUM, 1.5, 0.15),
                WingsTextStyle::new(14.0, W::MEDIUM, 1.43, 0.1),
                WingsTextStyle::new(16.0, W::NORMAL, 1.5, 0.5),
                WingsTextStyle::new(14.0, W::NORMAL, 1.43, 0.25),
                WingsTextStyle::new(12.0, W::NORMAL, 1.33, 0.4),
                WingsTextStyle::new(14.0, W::MEDIUM, 1.43, 0.1),
                WingsTextStyle::new(12.0, W::MEDIUM, 1.33, 0.5),
                WingsTextStyle::new(11.0, W::MEDIUM, 1.45, 0.5),
            ],
        }
    }
}

/// Builds an Apple text style from a point size, weight, line height in points and tracking
/// in points.
const fn apple(size: f32, weight: FontWeight, line: f32, tracking: f32) -> WingsTextStyle {
    WingsTextStyle::new(size, weight, line / size, tracking)
}

/// The font Apple designs use: the system UI font (San Francisco) when the platform font
/// database can provide it, then the bundled Inter.
///
/// Wings never bundles Apple fonts. On Apple targets Bevy's `system_font_discovery` resolves
/// the system UI family to San Francisco; elsewhere (previewing the Apple look on Linux or
/// Windows) the list falls through to Inter.
pub fn apple_font() -> FontSource {
    if cfg!(target_vendor = "apple") {
        system_font()
    } else {
        FontSource::default()
    }
}

/// The platform UI font (San Francisco on Apple platforms, Segoe UI Variable on Windows) when
/// Bevy's system font discovery finds it, then the bundled Inter. Without system font
/// discovery this is Inter.
pub fn system_font() -> FontSource {
    if cfg!(any(target_vendor = "apple", target_os = "windows")) {
        FontSource::List(vec![
            FontSource::Generic(GenericFontFamily::SystemUi),
            FontSource::default(),
        ])
    } else {
        FontSource::default()
    }
}

impl WingsTypography {
    /// The Material type scale (the default).
    pub fn material() -> Self {
        Self::default()
    }

    /// The iOS text styles (Large Title 34 down to Caption 12) at the default Dynamic Type
    /// size, with San Francisco tracking.
    pub fn ios() -> Self {
        use FontWeight as W;
        Self {
            font: apple_font(),
            styles: [
                apple(34.0, W::BOLD, 41.0, 0.40),
                apple(34.0, W::NORMAL, 41.0, 0.40),
                apple(28.0, W::NORMAL, 34.0, 0.38),
                apple(28.0, W::BOLD, 34.0, 0.38),
                apple(22.0, W::NORMAL, 28.0, -0.26),
                apple(20.0, W::NORMAL, 25.0, -0.45),
                apple(17.0, W::SEMIBOLD, 22.0, -0.43),
                apple(17.0, W::SEMIBOLD, 22.0, -0.43),
                apple(15.0, W::SEMIBOLD, 20.0, -0.23),
                apple(17.0, W::NORMAL, 22.0, -0.43),
                apple(17.0, W::NORMAL, 22.0, -0.43),
                apple(13.0, W::NORMAL, 18.0, -0.08),
                apple(17.0, W::NORMAL, 22.0, -0.43),
                apple(15.0, W::NORMAL, 20.0, -0.23),
                apple(12.0, W::NORMAL, 16.0, 0.0),
            ],
        }
    }

    /// The macOS text styles (Large Title 26 down to Caption 10).
    pub fn macos() -> Self {
        use FontWeight as W;
        Self {
            font: apple_font(),
            styles: [
                apple(26.0, W::BOLD, 32.0, 0.22),
                apple(26.0, W::NORMAL, 32.0, 0.22),
                apple(22.0, W::NORMAL, 26.0, -0.26),
                apple(22.0, W::BOLD, 26.0, -0.26),
                apple(17.0, W::NORMAL, 22.0, -0.43),
                apple(15.0, W::NORMAL, 20.0, -0.23),
                apple(13.0, W::BOLD, 16.0, -0.08),
                apple(13.0, W::BOLD, 16.0, -0.08),
                apple(11.0, W::SEMIBOLD, 14.0, 0.06),
                apple(13.0, W::NORMAL, 16.0, -0.08),
                apple(13.0, W::NORMAL, 16.0, -0.08),
                apple(10.0, W::NORMAL, 13.0, 0.12),
                apple(13.0, W::NORMAL, 16.0, -0.08),
                apple(11.0, W::NORMAL, 14.0, 0.06),
                apple(10.0, W::NORMAL, 13.0, 0.12),
            ],
        }
    }

    /// The Fluent 2 type ramp (Segoe UI Variable: Caption 12, Body 14, Subtitle 20, Title 28,
    /// Title Large 40, Display 68).
    pub fn fluent() -> Self {
        use FontWeight as W;
        Self {
            font: system_font(),
            styles: [
                apple(68.0, W::SEMIBOLD, 92.0, 0.0),
                apple(40.0, W::SEMIBOLD, 52.0, 0.0),
                apple(28.0, W::SEMIBOLD, 36.0, 0.0),
                apple(28.0, W::SEMIBOLD, 36.0, 0.0),
                apple(20.0, W::SEMIBOLD, 28.0, 0.0),
                apple(18.0, W::SEMIBOLD, 24.0, 0.0),
                apple(20.0, W::SEMIBOLD, 28.0, 0.0),
                apple(14.0, W::SEMIBOLD, 20.0, 0.0),
                apple(12.0, W::SEMIBOLD, 16.0, 0.0),
                apple(14.0, W::NORMAL, 20.0, 0.0),
                apple(14.0, W::NORMAL, 20.0, 0.0),
                apple(12.0, W::NORMAL, 16.0, 0.0),
                apple(14.0, W::NORMAL, 20.0, 0.0),
                apple(12.0, W::NORMAL, 16.0, 0.0),
                apple(11.0, W::NORMAL, 16.0, 0.0),
            ],
        }
    }

    /// The type scale of a design.
    pub fn for_design(design: WingsDesign) -> Self {
        match design {
            WingsDesign::Material => Self::material(),
            WingsDesign::Ios => Self::ios(),
            WingsDesign::MacOs => Self::macos(),
            WingsDesign::Fluent => Self::fluent(),
        }
    }

    fn index(role: TypeRole) -> usize {
        role as usize
    }

    /// Style for a role.
    pub fn get(&self, role: TypeRole) -> WingsTextStyle {
        self.styles[Self::index(role)]
    }

    /// Overrides a role.
    pub fn set(&mut self, role: TypeRole, style: WingsTextStyle) -> &mut Self {
        self.styles[Self::index(role)] = style;
        self
    }

    /// Multiplies every size by `factor` (accessibility text scaling).
    pub fn scaled(&self, factor: f32) -> Self {
        let mut out = self.clone();
        for s in &mut out.styles {
            s.size *= factor;
        }
        out
    }
}

/// A corner radius slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ShapeRole {
    /// Square corners.
    None,
    /// 4 px.
    ExtraSmall,
    /// 8 px.
    Small,
    /// 12 px.
    #[default]
    Medium,
    /// 16 px.
    Large,
    /// 28 px.
    ExtraLarge,
    /// Fully rounded (pill or circle).
    Full,
}

/// Corner radius tokens.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WingsShapes {
    /// Radii in pixels for `None` through `ExtraLarge`.
    pub radii: [f32; 6],
}

impl Default for WingsShapes {
    fn default() -> Self {
        Self {
            radii: [0.0, 4.0, 8.0, 12.0, 16.0, 28.0],
        }
    }
}

impl WingsShapes {
    /// Corner radii of a design. Apple designs use larger, softer corners.
    pub fn for_design(design: WingsDesign) -> Self {
        let radii = match design {
            WingsDesign::Material => [0.0, 4.0, 8.0, 12.0, 16.0, 28.0],
            WingsDesign::Ios => [0.0, 6.0, 10.0, 16.0, 26.0, 34.0],
            WingsDesign::MacOs => [0.0, 4.0, 6.0, 10.0, 16.0, 26.0],
            WingsDesign::Fluent => [0.0, 2.0, 4.0, 4.0, 8.0, 8.0],
        };
        Self { radii }
    }

    /// Border radius for a shape role.
    pub fn radius(&self, role: ShapeRole) -> BorderRadius {
        match role {
            ShapeRole::Full => BorderRadius::MAX,
            other => BorderRadius::all(Val::Px(self.radii[other as usize])),
        }
    }

    /// Radius in pixels (`f32::MAX` for [`ShapeRole::Full`]).
    pub fn px(&self, role: ShapeRole) -> f32 {
        match role {
            ShapeRole::Full => f32::MAX,
            other => self.radii[other as usize],
        }
    }
}

/// Spacing scale in pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WingsSpacing {
    /// 4 px.
    pub xs: f32,
    /// 8 px.
    pub sm: f32,
    /// 16 px.
    pub md: f32,
    /// 24 px.
    pub lg: f32,
    /// 32 px.
    pub xl: f32,
}

impl WingsSpacing {
    /// Spacing scale of a design.
    pub fn for_design(design: WingsDesign) -> Self {
        let [xs, sm, md, lg, xl] = match design {
            WingsDesign::Material => [4.0, 8.0, 16.0, 24.0, 32.0],
            WingsDesign::Ios => [4.0, 8.0, 16.0, 20.0, 32.0],
            WingsDesign::MacOs => [4.0, 8.0, 12.0, 20.0, 28.0],
            WingsDesign::Fluent => [4.0, 8.0, 12.0, 24.0, 36.0],
        };
        Self { xs, sm, md, lg, xl }
    }
}

impl Default for WingsSpacing {
    fn default() -> Self {
        Self {
            xs: 4.0,
            sm: 8.0,
            md: 16.0,
            lg: 24.0,
            xl: 32.0,
        }
    }
}

/// Elevation tokens: levels `0..=5` mapped to shadows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WingsElevation {
    /// `(y offset, blur)` per level, in pixels.
    pub levels: [(f32, f32); 6],
    /// Shadow alpha in the light scheme.
    pub light_alpha: f32,
    /// Shadow alpha in the dark scheme.
    pub dark_alpha: f32,
}

impl Default for WingsElevation {
    fn default() -> Self {
        Self {
            levels: [
                (0.0, 0.0),
                (1.0, 3.0),
                (2.0, 6.0),
                (4.0, 8.0),
                (6.0, 12.0),
                (8.0, 16.0),
            ],
            light_alpha: 0.22,
            dark_alpha: 0.5,
        }
    }
}

impl WingsElevation {
    /// Shadows of a design. Apple shadows are larger and much softer.
    pub fn for_design(design: WingsDesign) -> Self {
        match design {
            WingsDesign::Material => Self::default(),
            WingsDesign::Ios | WingsDesign::MacOs => Self {
                levels: [
                    (0.0, 0.0),
                    (1.0, 3.0),
                    (2.0, 8.0),
                    (4.0, 16.0),
                    (8.0, 28.0),
                    (12.0, 40.0),
                ],
                light_alpha: 0.12,
                dark_alpha: 0.45,
            },
            WingsDesign::Fluent => Self {
                levels: [
                    (0.0, 0.0),
                    (1.0, 2.0),
                    (2.0, 4.0),
                    (8.0, 16.0),
                    (16.0, 32.0),
                    (32.0, 64.0),
                ],
                light_alpha: 0.14,
                dark_alpha: 0.28,
            },
        }
    }

    /// Shadow for a level (clamped to `0..=5`).
    pub fn shadow(&self, level: u8, shadow: Color, dark: bool) -> BoxShadow {
        let (y, blur) = self.levels[level.min(5) as usize];
        if level == 0 {
            return BoxShadow(Vec::new());
        }
        let alpha = if dark {
            self.dark_alpha
        } else {
            self.light_alpha
        };
        BoxShadow(vec![ShadowStyle {
            color: shadow.with_alpha(alpha),
            x_offset: Val::Px(0.0),
            y_offset: Val::Px(y),
            spread_radius: Val::Px(0.0),
            blur_radius: Val::Px(blur),
        }])
    }
}

use bevy::color::Alpha;

/// Motion tokens.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WingsMotion {
    /// Hover, focus, small state changes.
    pub short: AnimationSpec,
    /// Most transitions.
    pub medium: AnimationSpec,
    /// Large surfaces and screens.
    pub long: AnimationSpec,
    /// Color crossfade used when the theme changes. `None` switches instantly.
    pub theme_transition: Option<AnimationSpec>,
}

impl WingsMotion {
    /// Motion of a design. Apple designs use critically damped springs (SwiftUI `.smooth`).
    pub fn for_design(design: WingsDesign) -> Self {
        match design {
            WingsDesign::Material => Self::default(),
            WingsDesign::Ios | WingsDesign::MacOs => Self {
                short: AnimationSpec::spring_with(SpringSpec::from_response(0.3, 1.0)),
                medium: AnimationSpec::spring_with(SpringSpec::from_response(0.5, 1.0)),
                long: AnimationSpec::spring_with(SpringSpec::from_response(0.65, 1.0)),
                theme_transition: Some(AnimationSpec::millis(250)),
            },
            // Fluent "fast invoke" motion: 167, 250 and 333 ms with a decelerating curve.
            WingsDesign::Fluent => Self {
                short: AnimationSpec::millis(167)
                    .with_easing(WingsEasing::CubicBezier(0.0, 0.0, 0.0, 1.0)),
                medium: AnimationSpec::millis(250)
                    .with_easing(WingsEasing::CubicBezier(0.0, 0.0, 0.0, 1.0)),
                long: AnimationSpec::millis(333)
                    .with_easing(WingsEasing::CubicBezier(0.0, 0.0, 0.0, 1.0)),
                theme_transition: Some(AnimationSpec::millis(167)),
            },
        }
    }
}

impl Default for WingsMotion {
    fn default() -> Self {
        Self {
            short: AnimationSpec::short(),
            medium: AnimationSpec::medium(),
            long: AnimationSpec::long(),
            theme_transition: Some(AnimationSpec::millis(300)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Type roles index the matching style.
    #[test]
    fn typography_lookup() {
        let t = WingsTypography::default();
        assert_eq!(t.get(TypeRole::DisplayLarge).size, 57.0);
        assert_eq!(t.get(TypeRole::LabelSmall).size, 11.0);
        assert_eq!(t.get(TypeRole::BodyMedium).size, 14.0);
    }

    /// Scaling multiplies every size.
    #[test]
    fn typography_scaling() {
        let t = WingsTypography::default().scaled(2.0);
        assert_eq!(t.get(TypeRole::BodyLarge).size, 32.0);
    }

    /// Overriding a type role.
    #[test]
    fn typography_override() {
        let mut t = WingsTypography::default();
        t.set(
            TypeRole::TitleLarge,
            WingsTextStyle::new(30.0, FontWeight::BOLD, 1.2, 0.0),
        );
        assert_eq!(t.get(TypeRole::TitleLarge).weight, FontWeight::BOLD);
    }

    /// Shape roles map to radii.
    #[test]
    fn apple_type_scale() {
        let ios = WingsTypography::ios();
        assert_eq!(ios.get(TypeRole::BodyMedium).size, 17.0);
        assert_eq!(ios.get(TypeRole::DisplayMedium).size, 34.0);
        assert_eq!(ios.get(TypeRole::TitleLarge).weight, FontWeight::SEMIBOLD);
        assert!((ios.get(TypeRole::BodyLarge).line_height - 22.0 / 17.0).abs() < 1e-6);
        let mac = WingsTypography::macos();
        assert_eq!(mac.get(TypeRole::BodyMedium).size, 13.0);
        assert_eq!(mac.get(TypeRole::TitleLarge).weight, FontWeight::BOLD);
        assert_eq!(
            WingsTypography::for_design(WingsDesign::Material),
            WingsTypography::default()
        );
    }

    #[test]
    fn fluent_tokens() {
        let t = WingsTypography::fluent();
        assert_eq!(t.get(TypeRole::BodyMedium).size, 14.0);
        assert_eq!(t.get(TypeRole::DisplaySmall).size, 28.0);
        assert_eq!(
            WingsShapes::for_design(WingsDesign::Fluent).px(ShapeRole::Small),
            4.0
        );
        assert_eq!(
            WingsShapes::for_design(WingsDesign::Fluent).px(ShapeRole::Large),
            8.0
        );
    }

    #[test]
    fn design_tokens_differ() {
        assert_eq!(
            WingsShapes::for_design(WingsDesign::Ios).px(ShapeRole::Large),
            26.0
        );
        assert_eq!(
            WingsShapes::for_design(WingsDesign::MacOs).px(ShapeRole::Medium),
            10.0
        );
        assert_eq!(WingsSpacing::for_design(WingsDesign::MacOs).md, 12.0);
        assert_eq!(
            WingsElevation::for_design(WingsDesign::Ios).light_alpha,
            0.12
        );
        assert!(matches!(
            WingsMotion::for_design(WingsDesign::Ios).short,
            AnimationSpec::Spring { .. }
        ));
    }

    #[test]
    fn shape_radius() {
        let s = WingsShapes::default();
        assert_eq!(s.px(ShapeRole::Medium), 12.0);
        assert_eq!(s.radius(ShapeRole::Full), BorderRadius::MAX);
        assert_eq!(s.radius(ShapeRole::Small), BorderRadius::all(Val::Px(8.0)));
    }

    /// Level 0 has no shadow, higher levels grow.
    #[test]
    fn elevation_levels() {
        let e = WingsElevation::default();
        assert!(e.shadow(0, Color::BLACK, false).0.is_empty());
        let s1 = e.shadow(1, Color::BLACK, false).0[0];
        let s5 = e.shadow(9, Color::BLACK, true).0[0];
        assert!(s5.blur_radius != s1.blur_radius);
        assert_eq!(s5.color.alpha(), 0.5);
    }
}
