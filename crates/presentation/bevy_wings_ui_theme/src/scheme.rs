//! Color roles and color schemes.
//!
//! Widgets never hard code colors. They reference a semantic [`ColorRole`] such as
//! [`ColorRole::Primary`] or [`ColorRole::Surface`], and the active [`WingsColorScheme`] maps
//! roles to concrete colors. Switching the scheme recolors every widget in one pass.
//!
//! Schemes can be generated from a single seed color with
//! [`WingsColorScheme::from_seed`], which builds tonal palettes in the perceptual Oklch space.

use bevy::color::{Color, Oklcha};
use bevy::platform::collections::HashMap;

/// A semantic color slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorRole {
    /// Main brand color for prominent components (filled buttons, active states).
    Primary,
    /// Content on top of [`ColorRole::Primary`].
    OnPrimary,
    /// Lower emphasis brand fill (tonal buttons, chips).
    PrimaryContainer,
    /// Content on top of [`ColorRole::PrimaryContainer`].
    OnPrimaryContainer,
    /// Secondary accent.
    Secondary,
    /// Content on top of [`ColorRole::Secondary`].
    OnSecondary,
    /// Lower emphasis secondary fill.
    SecondaryContainer,
    /// Content on top of [`ColorRole::SecondaryContainer`].
    OnSecondaryContainer,
    /// Tertiary, contrasting accent.
    Tertiary,
    /// Content on top of [`ColorRole::Tertiary`].
    OnTertiary,
    /// Lower emphasis tertiary fill.
    TertiaryContainer,
    /// Content on top of [`ColorRole::TertiaryContainer`].
    OnTertiaryContainer,
    /// Errors and destructive actions.
    Error,
    /// Content on top of [`ColorRole::Error`].
    OnError,
    /// Lower emphasis error fill.
    ErrorContainer,
    /// Content on top of [`ColorRole::ErrorContainer`].
    OnErrorContainer,
    /// App background.
    Background,
    /// Content on top of [`ColorRole::Background`].
    OnBackground,
    /// Default surface for cards, sheets and menus.
    Surface,
    /// Content on top of [`ColorRole::Surface`].
    OnSurface,
    /// Alternative surface.
    SurfaceVariant,
    /// Lower emphasis content on surfaces.
    OnSurfaceVariant,
    /// Lowest container surface.
    SurfaceContainerLowest,
    /// Low container surface.
    SurfaceContainerLow,
    /// Default container surface.
    SurfaceContainer,
    /// High container surface.
    SurfaceContainerHigh,
    /// Highest container surface.
    SurfaceContainerHighest,
    /// Inverse surface (snackbars).
    InverseSurface,
    /// Content on top of [`ColorRole::InverseSurface`].
    InverseOnSurface,
    /// Primary color usable on inverse surfaces.
    InversePrimary,
    /// Borders and dividers with emphasis.
    Outline,
    /// Subtle borders and dividers.
    OutlineVariant,
    /// Modal scrim behind dialogs.
    Scrim,
    /// Shadow color.
    Shadow,
    /// Positive status.
    Success,
    /// Content on top of [`ColorRole::Success`].
    OnSuccess,
    /// Warning status.
    Warning,
    /// Content on top of [`ColorRole::Warning`].
    OnWarning,
    /// An application defined role, looked up by name.
    Custom(&'static str),
}

impl ColorRole {
    /// Every built in role (excluding [`ColorRole::Custom`]).
    pub const BUILT_IN: [ColorRole; 38] = [
        ColorRole::Primary,
        ColorRole::OnPrimary,
        ColorRole::PrimaryContainer,
        ColorRole::OnPrimaryContainer,
        ColorRole::Secondary,
        ColorRole::OnSecondary,
        ColorRole::SecondaryContainer,
        ColorRole::OnSecondaryContainer,
        ColorRole::Tertiary,
        ColorRole::OnTertiary,
        ColorRole::TertiaryContainer,
        ColorRole::OnTertiaryContainer,
        ColorRole::Error,
        ColorRole::OnError,
        ColorRole::ErrorContainer,
        ColorRole::OnErrorContainer,
        ColorRole::Background,
        ColorRole::OnBackground,
        ColorRole::Surface,
        ColorRole::OnSurface,
        ColorRole::SurfaceVariant,
        ColorRole::OnSurfaceVariant,
        ColorRole::SurfaceContainerLowest,
        ColorRole::SurfaceContainerLow,
        ColorRole::SurfaceContainer,
        ColorRole::SurfaceContainerHigh,
        ColorRole::SurfaceContainerHighest,
        ColorRole::InverseSurface,
        ColorRole::InverseOnSurface,
        ColorRole::InversePrimary,
        ColorRole::Outline,
        ColorRole::OutlineVariant,
        ColorRole::Scrim,
        ColorRole::Shadow,
        ColorRole::Success,
        ColorRole::OnSuccess,
        ColorRole::Warning,
        ColorRole::OnWarning,
    ];

    /// Returns the role meant for content drawn on top of this role, when one exists.
    pub fn on(self) -> Option<ColorRole> {
        use ColorRole::*;
        Some(match self {
            Primary => OnPrimary,
            PrimaryContainer => OnPrimaryContainer,
            Secondary => OnSecondary,
            SecondaryContainer => OnSecondaryContainer,
            Tertiary => OnTertiary,
            TertiaryContainer => OnTertiaryContainer,
            Error => OnError,
            ErrorContainer => OnErrorContainer,
            Background => OnBackground,
            Surface
            | SurfaceContainerLowest
            | SurfaceContainerLow
            | SurfaceContainer
            | SurfaceContainerHigh
            | SurfaceContainerHighest => OnSurface,
            SurfaceVariant => OnSurfaceVariant,
            InverseSurface => InverseOnSurface,
            Success => OnSuccess,
            Warning => OnWarning,
            _ => return None,
        })
    }

    fn index(self) -> Option<usize> {
        Self::BUILT_IN.iter().position(|r| *r == self)
    }
}

/// Maps every [`ColorRole`] to a concrete color.
#[derive(Debug, Clone, PartialEq)]
pub struct WingsColorScheme {
    colors: [Color; 38],
    custom: HashMap<&'static str, Color>,
}

impl Default for WingsColorScheme {
    fn default() -> Self {
        Self::light()
    }
}

/// Builds a color from an Oklch hue, chroma and lightness.
fn tone(hue: f32, chroma: f32, lightness: f32) -> Color {
    Color::Oklcha(Oklcha::new(
        lightness.clamp(0.0, 1.0),
        chroma.max(0.0),
        hue.rem_euclid(360.0),
        1.0,
    ))
}

impl WingsColorScheme {
    /// The default seed: Wings indigo.
    pub const DEFAULT_SEED: Color = Color::srgb(0.24, 0.35, 0.99);

    /// Default light scheme.
    pub fn light() -> Self {
        Self::from_seed(Self::DEFAULT_SEED, false)
    }

    /// Default dark scheme.
    pub fn dark() -> Self {
        Self::from_seed(Self::DEFAULT_SEED, true)
    }

    /// Generates a full scheme from a single seed color.
    ///
    /// The seed hue drives the primary palette, the secondary palette is a desaturated version
    /// of it, and the tertiary palette is rotated by 60 degrees. Neutral surfaces carry a hint
    /// of the seed hue. Light schemes use dark content on light surfaces and dark schemes the
    /// opposite, with tones chosen to keep text contrast high in both.
    pub fn from_seed(seed: Color, dark: bool) -> Self {
        let lch = Oklcha::from(seed);
        let h = lch.hue;
        let c = lch.chroma.clamp(0.04, 0.2);
        let p = |l: f32| tone(h, c, l);
        let s = |l: f32| tone(h, c * 0.35, l);
        let t = |l: f32| tone(h + 60.0, c * 0.7, l);
        let e = |l: f32| tone(25.0, 0.19, l);
        let n = |l: f32| tone(h, 0.01, l);
        let nv = |l: f32| tone(h, 0.025, l);
        let ok = |l: f32| tone(145.0, 0.15, l);
        let warn = |l: f32| tone(75.0, 0.15, l);

        // (light tone, dark tone) for each built in role, in BUILT_IN order.
        let pick = |light: f32, dark_l: f32| if dark { dark_l } else { light };
        let colors = [
            p(pick(0.48, 0.80)),
            p(pick(1.0, 0.25)),
            p(pick(0.92, 0.36)),
            p(pick(0.22, 0.93)),
            s(pick(0.50, 0.80)),
            s(pick(1.0, 0.27)),
            s(pick(0.92, 0.37)),
            s(pick(0.23, 0.92)),
            t(pick(0.50, 0.80)),
            t(pick(1.0, 0.27)),
            t(pick(0.92, 0.37)),
            t(pick(0.23, 0.92)),
            e(pick(0.52, 0.80)),
            e(pick(1.0, 0.27)),
            e(pick(0.92, 0.38)),
            e(pick(0.27, 0.92)),
            n(pick(0.985, 0.17)),
            n(pick(0.20, 0.92)),
            n(pick(0.985, 0.17)),
            n(pick(0.20, 0.92)),
            nv(pick(0.91, 0.33)),
            nv(pick(0.40, 0.82)),
            n(pick(1.0, 0.13)),
            n(pick(0.965, 0.20)),
            n(pick(0.95, 0.22)),
            n(pick(0.93, 0.26)),
            n(pick(0.91, 0.30)),
            n(pick(0.30, 0.92)),
            n(pick(0.96, 0.30)),
            p(pick(0.80, 0.48)),
            nv(pick(0.55, 0.62)),
            nv(pick(0.83, 0.38)),
            Color::BLACK,
            Color::BLACK,
            ok(pick(0.55, 0.80)),
            ok(pick(1.0, 0.25)),
            warn(pick(0.70, 0.82)),
            warn(pick(0.22, 0.25)),
        ];
        Self {
            colors,
            custom: HashMap::default(),
        }
    }

    /// Returns the color for `role`. Unknown custom roles fall back to magenta so they are
    /// easy to spot during development.
    pub fn get(&self, role: ColorRole) -> Color {
        match role {
            ColorRole::Custom(name) => self
                .custom
                .get(name)
                .copied()
                .unwrap_or(Color::srgb(1.0, 0.0, 1.0)),
            built_in => self.colors[built_in.index().expect("built in role")],
        }
    }

    /// Overrides a role.
    pub fn set(&mut self, role: ColorRole, color: Color) -> &mut Self {
        match role {
            ColorRole::Custom(name) => {
                self.custom.insert(name, color);
            }
            built_in => self.colors[built_in.index().expect("built in role")] = color,
        }
        self
    }

    /// Builder style override.
    pub fn with(mut self, role: ColorRole, color: Color) -> Self {
        self.set(role, color);
        self
    }

    /// True when a custom role with that name is defined.
    pub fn has_custom(&self, name: &str) -> bool {
        self.custom.contains_key(name)
    }
}

/// Relative luminance of a color (WCAG definition).
pub fn relative_luminance(color: Color) -> f32 {
    let c = color.to_linear();
    0.2126 * c.red + 0.7152 * c.green + 0.0722 * c.blue
}

/// WCAG contrast ratio between two colors, from `1.0` (none) to `21.0` (black on white).
pub fn contrast_ratio(a: Color, b: Color) -> f32 {
    let (la, lb) = (relative_luminance(a), relative_luminance(b));
    let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every "on" role must be readable (WCAG AA large text, 3:1) on its container role in
    /// both default schemes.
    #[test]
    fn content_roles_have_contrast() {
        for scheme in [WingsColorScheme::light(), WingsColorScheme::dark()] {
            for role in ColorRole::BUILT_IN {
                if let Some(on) = role.on() {
                    let ratio = contrast_ratio(scheme.get(role), scheme.get(on));
                    assert!(ratio >= 3.0, "{role:?} vs {on:?}: {ratio}");
                }
            }
        }
    }

    /// Light surfaces are light and dark surfaces are dark.
    #[test]
    fn surfaces_follow_mode() {
        let light = WingsColorScheme::light().get(ColorRole::Surface);
        let dark = WingsColorScheme::dark().get(ColorRole::Surface);
        assert!(relative_luminance(light) > 0.8);
        assert!(relative_luminance(dark) < 0.1);
    }

    /// Custom roles round trip and unknown ones fall back to magenta.
    #[test]
    fn custom_roles() {
        let scheme = WingsColorScheme::light().with(ColorRole::Custom("brand"), Color::WHITE);
        assert!(scheme.has_custom("brand"));
        assert_eq!(scheme.get(ColorRole::Custom("brand")), Color::WHITE);
        assert_eq!(
            scheme.get(ColorRole::Custom("missing")),
            Color::srgb(1.0, 0.0, 1.0)
        );
    }

    /// Built in roles can be overridden.
    #[test]
    fn override_built_in() {
        let mut scheme = WingsColorScheme::dark();
        scheme.set(ColorRole::Primary, Color::BLACK);
        assert_eq!(scheme.get(ColorRole::Primary), Color::BLACK);
    }

    /// Different seeds give different primaries but keep contrast.
    #[test]
    fn seeds_change_primary() {
        let a = WingsColorScheme::from_seed(Color::srgb(1.0, 0.2, 0.2), false);
        let b = WingsColorScheme::from_seed(Color::srgb(0.2, 0.8, 0.3), false);
        assert_ne!(a.get(ColorRole::Primary), b.get(ColorRole::Primary));
        assert!(contrast_ratio(a.get(ColorRole::Primary), a.get(ColorRole::OnPrimary)) >= 3.0);
    }

    /// Contrast ratio sanity check.
    #[test]
    fn contrast_black_white() {
        assert!((contrast_ratio(Color::BLACK, Color::WHITE) - 21.0).abs() < 0.1);
    }

    /// `BUILT_IN` contains unique roles.
    #[test]
    fn built_in_unique() {
        let set: std::collections::HashSet<_> = ColorRole::BUILT_IN.iter().collect();
        assert_eq!(set.len(), ColorRole::BUILT_IN.len());
    }
}
