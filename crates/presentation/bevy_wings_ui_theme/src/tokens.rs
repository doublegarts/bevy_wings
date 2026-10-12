//! Design tokens other than color: typography, shapes, spacing, elevation and motion.

use bevy::color::Color;
use bevy::text::{FontSource, FontWeight};
use bevy::ui::{BorderRadius, BoxShadow, ShadowStyle, Val};

use bevy_wings_ui_animation::spec::AnimationSpec;

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

impl WingsTypography {
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
