//! The [`WingsTheme`] resource.

use bevy::prelude::*;

use crate::design::WingsDesign;
use crate::scheme::{AppleSystemColors, ColorRole, WingsColorScheme};
use crate::tokens::{WingsElevation, WingsMotion, WingsShapes, WingsSpacing, WingsTypography};

/// Which color scheme to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Reflect)]
pub enum WingsThemeMode {
    /// Always light.
    #[default]
    Light,
    /// Always dark.
    Dark,
    /// Follow the operating system preference reported by the primary window.
    System,
}

/// The application theme. Mutating this resource (for example with
/// [`WingsTheme::toggle_dark`]) recolors every themed entity in the next frame.
///
/// The theme carries a [`WingsDesign`]. [`WingsTheme::default`] uses the platform's own design
/// ([`WingsDesign::native`]): SwiftUI style on iPhone, iPad and Mac, Material elsewhere.
/// [`WingsTheme::set_design`] switches every token set at runtime and widgets re-skin
/// themselves.
#[derive(Resource, Debug, Clone)]
pub struct WingsTheme {
    /// Design language. Change it with [`WingsTheme::set_design`] so the tokens follow.
    pub design: WingsDesign,
    /// Seed or tint color the schemes were built from, `None` for the design's default.
    pub seed: Option<Color>,
    /// Selected mode.
    pub mode: WingsThemeMode,
    /// Scheme used in light mode.
    pub light: WingsColorScheme,
    /// Scheme used in dark mode.
    pub dark: WingsColorScheme,
    /// Type scale.
    pub typography: WingsTypography,
    /// Corner radii.
    pub shapes: WingsShapes,
    /// Spacing scale.
    pub spacing: WingsSpacing,
    /// Shadows.
    pub elevation: WingsElevation,
    /// Motion.
    pub motion: WingsMotion,
    /// Last operating system preference (`true` = dark). Updated automatically.
    pub system_prefers_dark: bool,
}

impl Default for WingsTheme {
    fn default() -> Self {
        Self::for_design(WingsDesign::native())
    }
}

impl WingsTheme {
    /// The default theme of a design.
    pub fn for_design(design: WingsDesign) -> Self {
        Self::build(design, None, WingsThemeMode::Light, false)
    }

    /// The Material theme (the Wings 0.1 look) on every platform.
    pub fn material() -> Self {
        Self::for_design(WingsDesign::Material)
    }

    /// The iOS (SwiftUI) theme on every platform.
    pub fn ios() -> Self {
        Self::for_design(WingsDesign::Ios)
    }

    /// The macOS (SwiftUI) theme on every platform.
    pub fn macos() -> Self {
        Self::for_design(WingsDesign::MacOs)
    }

    /// The Fluent (Windows 11) theme on every platform.
    pub fn fluent() -> Self {
        Self::for_design(WingsDesign::Fluent)
    }

    fn build(
        design: WingsDesign,
        seed: Option<Color>,
        mode: WingsThemeMode,
        system_prefers_dark: bool,
    ) -> Self {
        let (light, dark) = Self::schemes(design, seed);
        Self {
            design,
            seed,
            mode,
            light,
            dark,
            typography: WingsTypography::for_design(design),
            shapes: WingsShapes::for_design(design),
            spacing: WingsSpacing::for_design(design),
            elevation: WingsElevation::for_design(design),
            motion: WingsMotion::for_design(design),
            system_prefers_dark,
        }
    }

    fn schemes(design: WingsDesign, seed: Option<Color>) -> (WingsColorScheme, WingsColorScheme) {
        match design {
            WingsDesign::Material => {
                let seed = seed.unwrap_or(WingsColorScheme::DEFAULT_SEED);
                (
                    WingsColorScheme::from_seed(seed, false),
                    WingsColorScheme::from_seed(seed, true),
                )
            }
            WingsDesign::Ios => (
                WingsColorScheme::apple(AppleSystemColors::ios(false), seed, false),
                WingsColorScheme::apple(AppleSystemColors::ios(true), seed, true),
            ),
            WingsDesign::MacOs => (
                WingsColorScheme::apple(AppleSystemColors::macos(false), seed, false),
                WingsColorScheme::apple(AppleSystemColors::macos(true), seed, true),
            ),
            WingsDesign::Fluent => (
                WingsColorScheme::fluent(false, seed),
                WingsColorScheme::fluent(true, seed),
            ),
        }
    }

    /// A theme generated from one seed color in the native design. Material builds tonal
    /// palettes from the seed; Apple designs use it as the tint, like `.tint()` in SwiftUI.
    pub fn from_seed(seed: Color) -> Self {
        Self::build(
            WingsDesign::native(),
            Some(seed),
            WingsThemeMode::Light,
            false,
        )
    }

    /// Builder: sets the design (see [`WingsTheme::set_design`]).
    pub fn with_design(mut self, design: WingsDesign) -> Self {
        self.set_design(design);
        self
    }

    /// Switches the design language. Every token set (colors, type, shapes, spacing,
    /// elevation, motion) is replaced by the design's defaults; the mode, the seed and the
    /// system preference are kept. Widgets re-skin in the next frame.
    pub fn set_design(&mut self, design: WingsDesign) {
        *self = Self::build(design, self.seed, self.mode, self.system_prefers_dark);
    }

    /// Builder: sets the mode.
    pub fn with_mode(mut self, mode: WingsThemeMode) -> Self {
        self.mode = mode;
        self
    }

    /// True when the dark scheme is active.
    pub fn is_dark(&self) -> bool {
        match self.mode {
            WingsThemeMode::Light => false,
            WingsThemeMode::Dark => true,
            WingsThemeMode::System => self.system_prefers_dark,
        }
    }

    /// The active scheme.
    pub fn colors(&self) -> &WingsColorScheme {
        self.scheme(self.is_dark())
    }

    /// The light (`false`) or dark (`true`) scheme.
    pub fn scheme(&self, dark: bool) -> &WingsColorScheme {
        if dark { &self.dark } else { &self.light }
    }

    /// Active color for a role.
    pub fn color(&self, role: ColorRole) -> Color {
        self.colors().get(role)
    }

    /// Switches between light and dark. In `System` mode it pins the opposite of the
    /// current system preference.
    pub fn toggle_dark(&mut self) {
        self.mode = if self.is_dark() {
            WingsThemeMode::Light
        } else {
            WingsThemeMode::Dark
        };
    }
}

/// Sent whenever the effective light or dark state changes.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct WingsThemeChanged {
    /// True when the dark scheme is now active.
    pub dark: bool,
}

/// Forces a subtree into the light or dark scheme regardless of the global mode (for example
/// an always dark media player inside a light app).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub struct WingsLocalTheme {
    /// True for the dark scheme.
    pub dark: bool,
}

impl WingsLocalTheme {
    /// Always light subtree.
    pub const LIGHT: Self = Self { dark: false };
    /// Always dark subtree.
    pub const DARK: Self = Self { dark: true };
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Toggling flips between light and dark.
    #[test]
    fn toggle_flips() {
        let mut t = WingsTheme::default();
        assert!(!t.is_dark());
        t.toggle_dark();
        assert!(t.is_dark());
        assert_eq!(t.mode, WingsThemeMode::Dark);
        t.toggle_dark();
        assert!(!t.is_dark());
    }

    /// System mode follows the reported preference.
    #[test]
    fn system_mode_follows_preference() {
        let mut t = WingsTheme::default().with_mode(WingsThemeMode::System);
        assert!(!t.is_dark());
        t.system_prefers_dark = true;
        assert!(t.is_dark());
        assert_eq!(t.color(ColorRole::Surface), t.dark.get(ColorRole::Surface));
    }

    /// Seeded themes differ from the default.
    #[test]
    fn seeded_theme() {
        let t = WingsTheme::from_seed(Color::srgb(0.9, 0.3, 0.1));
        assert_ne!(
            t.light.get(ColorRole::Primary),
            WingsTheme::default().light.get(ColorRole::Primary)
        );
    }

    /// The default theme uses the native design.
    #[test]
    fn default_is_native() {
        assert_eq!(WingsTheme::default().design, WingsDesign::native());
        assert_eq!(WingsTheme::material().design, WingsDesign::Material);
    }

    /// Switching the design swaps tokens and keeps mode and seed.
    #[test]
    fn set_design_keeps_mode_and_seed() {
        let seed = Color::srgb(0.9, 0.1, 0.4);
        let mut t = WingsTheme::material().with_mode(WingsThemeMode::Dark);
        t.seed = Some(seed);
        t.set_design(WingsDesign::Ios);
        assert_eq!(t.design, WingsDesign::Ios);
        assert_eq!(t.mode, WingsThemeMode::Dark);
        assert_eq!(t.seed, Some(seed));
        assert_eq!(t.typography, WingsTypography::ios());
        assert_eq!(t.light.get(ColorRole::Primary), seed);
        t.set_design(WingsDesign::Material);
        assert_eq!(t.typography, WingsTypography::default());
    }
}
