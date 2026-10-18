//! `wings.toml`: choose between the platform's own look and your custom design.
//!
//! ```toml
//! # wings.toml, next to your app's Cargo.toml
//!
//! # "system": the platform's own design. Material on Android and Linux, Fluent on Windows,
//! #           Cupertino on iOS, iPadOS and macOS.
//! # "custom": your design from [custom], identical on every platform.
//! ui = "system"
//!
//! # "light", "dark" or "system" (follow the operating system).
//! appearance = "system"
//!
//! # Optional brand color. Material builds its palettes from it, Cupertino and Fluent use it as
//! # the accent (tint).
//! accent = "#3D59FC"
//!
//! [custom]
//! # Skin the custom design starts from: "material", "cupertino", "ios", "macos" or "fluent".
//! design = "material"
//! font_family = "Brand Sans"   # a font family the platform font database knows
//! font_scale = 1.0             # multiplies every text size
//! radii = [0, 6, 10, 14, 20, 28]  # none, extra small, small, medium, large, extra large
//! spacing = [4, 8, 16, 24, 32]    # xs, sm, md, lg, xl
//!
//! [custom.light]               # any color role by its snake_case name
//! primary = "#3D59FC"
//! background = "#FAFAFF"
//!
//! [custom.dark]
//! primary = "#9AA8FF"
//! ```
//!
//! Embed it at compile time with [`wings_config!`](crate::wings_config) and hand it to the theme
//! plugin, so the setting also travels inside iOS and Android packages:
//!
//! ```ignore
//! app.add_plugins(WingsUiPlugins.set(WingsThemePlugin::from_config(wings_config!())));
//! ```
//!
//! Without a `wings.toml` the default theme behaves like `ui = "system"`.

use std::collections::BTreeMap;
use std::fmt;

use bevy::color::{Color, Srgba};
use bevy::text::FontSource;
use serde::Deserialize;

use crate::design::WingsDesign;
use crate::scheme::{ColorRole, WingsColorScheme};
use crate::theme::{WingsTheme, WingsThemeMode};

/// How the app's look is chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WingsUiMode {
    /// The platform's own design (see [`WingsDesign::native`]).
    #[default]
    System,
    /// One design, configured in `[custom]`, on every platform.
    Custom,
}

/// Light, dark or follow the operating system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WingsAppearance {
    /// Always light.
    Light,
    /// Always dark.
    Dark,
    /// Follow the operating system.
    #[default]
    System,
}

impl From<WingsAppearance> for WingsThemeMode {
    fn from(a: WingsAppearance) -> Self {
        match a {
            WingsAppearance::Light => WingsThemeMode::Light,
            WingsAppearance::Dark => WingsThemeMode::Dark,
            WingsAppearance::System => WingsThemeMode::System,
        }
    }
}

/// The `[custom]` table: a design that looks the same on every platform.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WingsCustomUi {
    /// Skin the design starts from (`material`, `cupertino`, `ios`, `macos`, `fluent`).
    pub design: String,
    /// Font family resolved through the platform font database.
    pub font_family: Option<String>,
    /// Multiplier for every text size.
    pub font_scale: f32,
    /// Corner radii: none, extra small, small, medium, large, extra large.
    pub radii: Option<[f32; 6]>,
    /// Spacing scale: xs, sm, md, lg, xl.
    pub spacing: Option<[f32; 5]>,
    /// Light scheme overrides, color role name to `#RRGGBB` or `#RRGGBBAA`.
    pub light: BTreeMap<String, String>,
    /// Dark scheme overrides.
    pub dark: BTreeMap<String, String>,
}

impl Default for WingsCustomUi {
    fn default() -> Self {
        Self {
            design: "material".into(),
            font_family: None,
            font_scale: 1.0,
            radii: None,
            spacing: None,
            light: BTreeMap::new(),
            dark: BTreeMap::new(),
        }
    }
}

/// The contents of a `wings.toml` file.
#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WingsConfig {
    /// `system` (the platform's design) or `custom`.
    pub ui: WingsUiMode,
    /// Light, dark or system appearance.
    pub appearance: WingsAppearance,
    /// Optional brand color (`#RRGGBB`).
    pub accent: Option<String>,
    /// The custom design, used when [`ui`](Self::ui) is `custom`.
    pub custom: WingsCustomUi,
}

/// A problem in `wings.toml`.
#[derive(Debug, Clone, PartialEq)]
pub enum WingsConfigError {
    /// The file is not valid TOML or has unknown keys.
    Toml(String),
    /// A design name is unknown.
    UnknownDesign(String),
    /// A color is not `#RRGGBB` or `#RRGGBBAA`.
    BadColor(String),
}

impl fmt::Display for WingsConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Toml(e) => write!(f, "invalid wings.toml: {e}"),
            Self::UnknownDesign(d) => write!(
                f,
                "unknown design `{d}` in wings.toml (use material, cupertino, ios, macos or fluent)"
            ),
            Self::BadColor(c) => write!(f, "invalid color `{c}` in wings.toml (use #RRGGBB)"),
        }
    }
}

impl std::error::Error for WingsConfigError {}

/// Parses `#RRGGBB`, `#RRGGBBAA`, `RRGGBB` or `#RGB`.
pub fn parse_hex_color(text: &str) -> Result<Color, WingsConfigError> {
    Srgba::hex(text.trim())
        .map(Color::Srgba)
        .map_err(|_| WingsConfigError::BadColor(text.to_string()))
}

impl WingsConfig {
    /// Parses and validates a `wings.toml` document.
    pub fn from_toml(text: &str) -> Result<Self, WingsConfigError> {
        let config: Self =
            toml::from_str(text).map_err(|e| WingsConfigError::Toml(e.to_string()))?;
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<(), WingsConfigError> {
        if self.ui == WingsUiMode::Custom && WingsDesign::parse(&self.custom.design).is_none() {
            return Err(WingsConfigError::UnknownDesign(self.custom.design.clone()));
        }
        if let Some(accent) = &self.accent {
            parse_hex_color(accent)?;
        }
        for value in self.custom.light.values().chain(self.custom.dark.values()) {
            parse_hex_color(value)?;
        }
        Ok(())
    }

    /// The design this configuration selects on the current platform.
    pub fn design(&self) -> WingsDesign {
        match self.ui {
            WingsUiMode::System => WingsDesign::native(),
            WingsUiMode::Custom => {
                WingsDesign::parse(&self.custom.design).unwrap_or(WingsDesign::Material)
            }
        }
    }

    /// The accent color, if one is set and valid.
    pub fn accent(&self) -> Option<Color> {
        self.accent.as_deref().and_then(|a| parse_hex_color(a).ok())
    }

    /// Builds the theme this configuration describes.
    pub fn theme(&self) -> WingsTheme {
        let mut theme = WingsTheme::for_design(self.design());
        theme.seed = self.accent();
        theme.set_design(theme.design);
        theme.mode = self.appearance.into();
        if self.ui == WingsUiMode::Custom {
            self.apply_custom(&mut theme);
        }
        theme
    }

    /// Applies the `[custom]` tokens to a theme.
    pub fn apply_custom(&self, theme: &mut WingsTheme) {
        let c = &self.custom;
        if let Some(family) = &c.font_family {
            theme.typography.font = FontSource::List(vec![
                FontSource::family(family.clone()),
                theme.typography.font.clone(),
            ]);
        }
        if (c.font_scale - 1.0).abs() > f32::EPSILON && c.font_scale > 0.0 {
            theme.typography = theme.typography.scaled(c.font_scale);
        }
        if let Some(radii) = c.radii {
            theme.shapes.radii = radii;
        }
        if let Some([xs, sm, md, lg, xl]) = c.spacing {
            theme.spacing = crate::tokens::WingsSpacing { xs, sm, md, lg, xl };
        }
        apply_colors(&mut theme.light, &c.light);
        apply_colors(&mut theme.dark, &c.dark);
    }
}

fn apply_colors(scheme: &mut WingsColorScheme, colors: &BTreeMap<String, String>) {
    for (name, value) in colors {
        let Ok(color) = parse_hex_color(value) else {
            continue;
        };
        let role = ColorRole::from_name(name)
            // Unknown names become custom roles that widgets can reference by name.
            .unwrap_or_else(|| ColorRole::Custom(Box::leak(name.clone().into_boxed_str())));
        scheme.set(role, color);
    }
}

/// Embeds the `wings.toml` next to the calling crate's `Cargo.toml` (or the given path,
/// relative to that manifest) and parses it into a [`WingsConfig`]. Panics with a readable
/// message when the file is invalid.
///
/// ```ignore
/// app.add_plugins(WingsUiPlugins.set(WingsThemePlugin::from_config(wings_config!())));
/// ```
#[macro_export]
macro_rules! wings_config {
    () => {
        $crate::wings_config!("wings.toml")
    };
    ($path:literal) => {
        match $crate::config::WingsConfig::from_toml(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/",
            $path
        ))) {
            Ok(config) => config,
            Err(error) => panic!("{error}"),
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An empty file means the platform's own design.
    #[test]
    fn empty_is_system() {
        let c = WingsConfig::from_toml("").unwrap();
        assert_eq!(c.ui, WingsUiMode::System);
        assert_eq!(c.design(), WingsDesign::native());
        assert_eq!(c.theme().mode, WingsThemeMode::System);
    }

    /// Custom mode uses one design everywhere and applies the tokens.
    #[test]
    fn custom_design() {
        let c = WingsConfig::from_toml(
            r##"
            ui = "custom"
            appearance = "dark"
            accent = "#FF3366"
            [custom]
            design = "fluent"
            font_scale = 1.5
            radii = [0, 1, 2, 3, 4, 5]
            spacing = [1, 2, 3, 4, 5]
            [custom.light]
            primary = "#112233"
            brand_glow = "#445566"
            "##,
        )
        .unwrap();
        assert_eq!(c.design(), WingsDesign::Fluent);
        let t = c.theme();
        assert_eq!(t.design, WingsDesign::Fluent);
        assert_eq!(t.mode, WingsThemeMode::Dark);
        assert_eq!(t.shapes.radii, [0.0, 1.0, 2.0, 3.0, 4.0, 5.0]);
        assert_eq!(t.spacing.md, 3.0);
        assert_eq!(
            t.typography.get(crate::tokens::TypeRole::BodyMedium).size,
            21.0
        );
        assert_eq!(
            t.light.get(ColorRole::Primary),
            parse_hex_color("#112233").unwrap()
        );
        assert!(t.light.has_custom("brand_glow"));
        assert!(t.seed.is_some());
    }

    /// System mode keeps the platform design but honours the accent.
    #[test]
    fn system_with_accent() {
        let c = WingsConfig::from_toml(r##"accent = "#00AA55""##).unwrap();
        let t = c.theme();
        assert_eq!(t.design, WingsDesign::native());
        assert_eq!(t.seed, parse_hex_color("#00AA55").ok());
    }

    /// Mistakes are reported clearly.
    #[test]
    fn errors() {
        assert!(matches!(
            WingsConfig::from_toml("ui = \"custom\"\n[custom]\ndesign = \"metro\""),
            Err(WingsConfigError::UnknownDesign(_))
        ));
        assert!(matches!(
            WingsConfig::from_toml("accent = \"blue\""),
            Err(WingsConfigError::BadColor(_))
        ));
        assert!(matches!(
            WingsConfig::from_toml("colour = 1"),
            Err(WingsConfigError::Toml(_))
        ));
    }
}
