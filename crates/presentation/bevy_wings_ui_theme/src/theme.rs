//! The [`WingsTheme`] resource.

use bevy::prelude::*;

use crate::scheme::{ColorRole, WingsColorScheme};
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
#[derive(Resource, Debug, Clone)]
pub struct WingsTheme {
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
        Self {
            mode: WingsThemeMode::Light,
            light: WingsColorScheme::light(),
            dark: WingsColorScheme::dark(),
            typography: WingsTypography::default(),
            shapes: WingsShapes::default(),
            spacing: WingsSpacing::default(),
            elevation: WingsElevation::default(),
            motion: WingsMotion::default(),
            system_prefers_dark: false,
        }
    }
}

impl WingsTheme {
    /// A theme generated from one seed color.
    pub fn from_seed(seed: Color) -> Self {
        Self {
            light: WingsColorScheme::from_seed(seed, false),
            dark: WingsColorScheme::from_seed(seed, true),
            ..Default::default()
        }
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
}
