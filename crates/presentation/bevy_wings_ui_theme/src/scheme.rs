//! Color roles and color schemes.
//!
//! Widgets never hard code colors. They reference a semantic [`ColorRole`] such as
//! [`ColorRole::Primary`] or [`ColorRole::Surface`], and the active [`WingsColorScheme`] maps
//! roles to concrete colors. Switching the scheme recolors every widget in one pass.
//!
//! Schemes can be generated from a single seed color with
//! [`WingsColorScheme::from_seed`], which builds tonal palettes in the perceptual Oklch space.

use bevy::color::{Alpha, Color, Oklcha, Srgba};
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
    /// Neutral fill for thin and small shapes (slider tracks, switch tracks). Apple
    /// `systemFill`.
    Fill,
    /// Neutral fill for medium shapes (bordered buttons, segmented controls). Apple
    /// `secondarySystemFill`.
    SecondaryFill,
    /// Neutral fill for large shapes (text fields, search fields). Apple `tertiarySystemFill`.
    TertiaryFill,
    /// Lowest emphasis text (placeholders, disabled labels). Apple `tertiaryLabel`.
    TertiaryLabel,
    /// Background behind grouped content (settings style lists). Apple
    /// `systemGroupedBackground`.
    GroupedBackground,
    /// Background of the groups themselves. Apple `secondarySystemGroupedBackground`.
    SecondaryGroupedBackground,
    /// An application defined role, looked up by name.
    Custom(&'static str),
}

impl ColorRole {
    /// Every built in role (excluding [`ColorRole::Custom`]).
    pub const BUILT_IN: [ColorRole; 44] = [
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
        ColorRole::Fill,
        ColorRole::SecondaryFill,
        ColorRole::TertiaryFill,
        ColorRole::TertiaryLabel,
        ColorRole::GroupedBackground,
        ColorRole::SecondaryGroupedBackground,
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
            | GroupedBackground
            | SecondaryGroupedBackground
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

    /// The `snake_case` name of a role, as used in `wings.toml` (`primary`, `on_surface`,
    /// `secondary_grouped_background`, ...). Custom roles return their own name.
    pub fn name(self) -> &'static str {
        match self {
            ColorRole::Primary => "primary",
            ColorRole::OnPrimary => "on_primary",
            ColorRole::PrimaryContainer => "primary_container",
            ColorRole::OnPrimaryContainer => "on_primary_container",
            ColorRole::Secondary => "secondary",
            ColorRole::OnSecondary => "on_secondary",
            ColorRole::SecondaryContainer => "secondary_container",
            ColorRole::OnSecondaryContainer => "on_secondary_container",
            ColorRole::Tertiary => "tertiary",
            ColorRole::OnTertiary => "on_tertiary",
            ColorRole::TertiaryContainer => "tertiary_container",
            ColorRole::OnTertiaryContainer => "on_tertiary_container",
            ColorRole::Error => "error",
            ColorRole::OnError => "on_error",
            ColorRole::ErrorContainer => "error_container",
            ColorRole::OnErrorContainer => "on_error_container",
            ColorRole::Background => "background",
            ColorRole::OnBackground => "on_background",
            ColorRole::Surface => "surface",
            ColorRole::OnSurface => "on_surface",
            ColorRole::SurfaceVariant => "surface_variant",
            ColorRole::OnSurfaceVariant => "on_surface_variant",
            ColorRole::SurfaceContainerLowest => "surface_container_lowest",
            ColorRole::SurfaceContainerLow => "surface_container_low",
            ColorRole::SurfaceContainer => "surface_container",
            ColorRole::SurfaceContainerHigh => "surface_container_high",
            ColorRole::SurfaceContainerHighest => "surface_container_highest",
            ColorRole::InverseSurface => "inverse_surface",
            ColorRole::InverseOnSurface => "inverse_on_surface",
            ColorRole::InversePrimary => "inverse_primary",
            ColorRole::Outline => "outline",
            ColorRole::OutlineVariant => "outline_variant",
            ColorRole::Scrim => "scrim",
            ColorRole::Shadow => "shadow",
            ColorRole::Success => "success",
            ColorRole::OnSuccess => "on_success",
            ColorRole::Warning => "warning",
            ColorRole::OnWarning => "on_warning",
            ColorRole::Fill => "fill",
            ColorRole::SecondaryFill => "secondary_fill",
            ColorRole::TertiaryFill => "tertiary_fill",
            ColorRole::TertiaryLabel => "tertiary_label",
            ColorRole::GroupedBackground => "grouped_background",
            ColorRole::SecondaryGroupedBackground => "secondary_grouped_background",
            ColorRole::Custom(name) => name,
        }
    }

    /// Looks up a built in role by its [`name`](Self::name).
    pub fn from_name(name: &str) -> Option<ColorRole> {
        Self::BUILT_IN.into_iter().find(|r| r.name() == name)
    }

    fn index(self) -> Option<usize> {
        Self::BUILT_IN.iter().position(|r| *r == self)
    }
}

/// Maps every [`ColorRole`] to a concrete color.
#[derive(Debug, Clone, PartialEq)]
pub struct WingsColorScheme {
    colors: [Color; ColorRole::BUILT_IN.len()],
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
            // Neutral fills and labels derive from the content color.
            n(pick(0.20, 0.92)).with_alpha(0.12),
            n(pick(0.20, 0.92)).with_alpha(0.08),
            n(pick(0.20, 0.92)).with_alpha(0.05),
            nv(pick(0.40, 0.82)).with_alpha(0.6),
            n(pick(0.965, 0.13)),
            n(pick(1.0, 0.20)),
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

/// `0xRRGGBB` as an opaque sRGB color.
const fn rgb(hex: u32) -> Color {
    Color::Srgba(Srgba {
        red: ((hex >> 16) & 0xff) as f32 / 255.0,
        green: ((hex >> 8) & 0xff) as f32 / 255.0,
        blue: (hex & 0xff) as f32 / 255.0,
        alpha: 1.0,
    })
}

/// `0xRRGGBB` with an alpha.
const fn rgba(hex: u32, alpha: f32) -> Color {
    Color::Srgba(Srgba {
        red: ((hex >> 16) & 0xff) as f32 / 255.0,
        green: ((hex >> 8) & 0xff) as f32 / 255.0,
        blue: (hex & 0xff) as f32 / 255.0,
        alpha,
    })
}

/// Composites `top` with opacity `t` over the opaque `base` in sRGB space, the way Apple
/// platforms blend translucent fills.
pub fn mix_srgb(base: Color, top: Color, t: f32) -> Color {
    let (b, f) = (base.to_srgba(), top.to_srgba());
    let t = t.clamp(0.0, 1.0);
    let m = |b: f32, f: f32| b + (f - b) * t;
    Color::srgb(m(b.red, f.red), m(b.green, f.green), m(b.blue, f.blue))
}

/// Re-expresses a translucent color made for sRGB blending (Apple, Windows) for Bevy, which
/// blends in linear space.
///
/// Apple's `label` is 85% black: blended in sRGB over white it is a near black gray, but the
/// same alpha blended in linear space comes out mid gray. This returns `color` with the alpha
/// that makes a linear blend over `background` match the sRGB blend exactly, so text and fills
/// look like on the platform while staying translucent over images and other content.
pub fn srgb_blend_alpha(color: Color, background: Color) -> Color {
    let alpha = color.alpha();
    if alpha >= 1.0 || alpha <= 0.0 {
        return color;
    }
    let opaque = color.with_alpha(1.0);
    let target = relative_luminance(mix_srgb(background, opaque, alpha));
    let (lb, lf) = (relative_luminance(background), relative_luminance(opaque));
    if (lf - lb).abs() < 1e-4 {
        return color;
    }
    color.with_alpha(((target - lb) / (lf - lb)).clamp(0.0, 1.0))
}

/// Apple's semantic system colors for one platform and appearance.
///
/// The values are the published UIKit and AppKit system colors of the Liquid Glass releases
/// (iOS 26 and macOS 26 and later), checked against a SwiftUI reference app.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AppleSystemColors {
    /// The default tint: `systemBlue` on iOS, `controlAccentColor` on macOS.
    pub blue: Color,
    /// `systemRed`.
    pub red: Color,
    /// `systemOrange`.
    pub orange: Color,
    /// `systemGreen`.
    pub green: Color,
    /// `systemIndigo`.
    pub indigo: Color,
    /// `systemGray`.
    pub gray: Color,
    /// `systemGray4`.
    pub gray4: Color,
    /// `systemGray5`.
    pub gray5: Color,
    /// `systemGray6`.
    pub gray6: Color,
    /// `systemBackground` (iOS) or `windowBackgroundColor` (macOS).
    pub background: Color,
    /// `secondarySystemBackground` or `controlBackgroundColor`.
    pub secondary_background: Color,
    /// `tertiarySystemBackground`.
    pub tertiary_background: Color,
    /// `systemGroupedBackground`.
    pub grouped_background: Color,
    /// `secondarySystemGroupedBackground`.
    pub secondary_grouped_background: Color,
    /// `label`.
    pub label: Color,
    /// `secondaryLabel`.
    pub secondary_label: Color,
    /// `tertiaryLabel`.
    pub tertiary_label: Color,
    /// `separator` (translucent).
    pub separator: Color,
    /// `opaqueSeparator`.
    pub opaque_separator: Color,
    /// `systemFill`.
    pub fill: Color,
    /// `secondarySystemFill`.
    pub secondary_fill: Color,
    /// `tertiarySystemFill`.
    pub tertiary_fill: Color,
}

impl AppleSystemColors {
    /// iOS and iPadOS system colors.
    pub const fn ios(dark: bool) -> Self {
        if dark {
            Self {
                blue: rgb(0x0091FF),
                red: rgb(0xFF4245),
                orange: rgb(0xFF9230),
                green: rgb(0x30D158),
                indigo: rgb(0x6D7CFF),
                gray: rgb(0x8E8E93),
                gray4: rgb(0x3A3A3C),
                gray5: rgb(0x2C2C2E),
                gray6: rgb(0x1C1C1E),
                background: rgb(0x000000),
                secondary_background: rgb(0x1C1C1E),
                tertiary_background: rgb(0x2C2C2E),
                grouped_background: rgb(0x000000),
                secondary_grouped_background: rgb(0x1C1C1E),
                label: rgb(0xFFFFFF),
                secondary_label: rgba(0xEBEBF5, 0.6),
                tertiary_label: rgba(0xEBEBF5, 0.3),
                separator: rgba(0x545458, 0.65),
                opaque_separator: rgb(0x38383A),
                fill: rgba(0x787880, 0.36),
                secondary_fill: rgba(0x787880, 0.32),
                tertiary_fill: rgba(0x767680, 0.24),
            }
        } else {
            Self {
                blue: rgb(0x0088FF),
                red: rgb(0xFF383C),
                orange: rgb(0xFF8D28),
                green: rgb(0x34C759),
                indigo: rgb(0x6155F5),
                gray: rgb(0x8E8E93),
                gray4: rgb(0xD1D1D6),
                gray5: rgb(0xE5E5EA),
                gray6: rgb(0xF2F2F7),
                background: rgb(0xFFFFFF),
                secondary_background: rgb(0xF2F2F7),
                tertiary_background: rgb(0xFFFFFF),
                grouped_background: rgb(0xF2F2F7),
                secondary_grouped_background: rgb(0xFFFFFF),
                label: rgb(0x000000),
                secondary_label: rgba(0x3C3C43, 0.6),
                tertiary_label: rgba(0x3C3C43, 0.3),
                separator: rgba(0x3C3C43, 0.29),
                opaque_separator: rgb(0xC6C6C8),
                fill: rgba(0x787880, 0.2),
                secondary_fill: rgba(0x787880, 0.16),
                tertiary_fill: rgba(0x767680, 0.12),
            }
        }
    }

    /// macOS system colors.
    pub const fn macos(dark: bool) -> Self {
        if dark {
            Self {
                blue: rgb(0x007AFF),
                red: rgb(0xFF4245),
                orange: rgb(0xFF9230),
                green: rgb(0x30D158),
                indigo: rgb(0x6D7CFF),
                gray: rgb(0x98989D),
                gray4: rgb(0x3A3A3C),
                gray5: rgb(0x2C2C2E),
                gray6: rgb(0x1C1C1E),
                background: rgb(0x1E1E1E),
                secondary_background: rgb(0x282828),
                tertiary_background: rgb(0x2D2D2D),
                grouped_background: rgb(0x1E1E1E),
                secondary_grouped_background: rgb(0x282828),
                label: rgba(0xFFFFFF, 0.847),
                secondary_label: rgba(0xFFFFFF, 0.549),
                tertiary_label: rgba(0xFFFFFF, 0.247),
                separator: rgba(0xFFFFFF, 0.098),
                opaque_separator: rgb(0x3D3D3D),
                fill: rgba(0xFFFFFF, 0.098),
                secondary_fill: rgba(0xFFFFFF, 0.078),
                tertiary_fill: rgba(0xFFFFFF, 0.047),
            }
        } else {
            Self {
                blue: rgb(0x007AFF),
                red: rgb(0xFF383C),
                orange: rgb(0xFF8D28),
                green: rgb(0x34C759),
                indigo: rgb(0x6155F5),
                gray: rgb(0x8E8E93),
                gray4: rgb(0xD1D1D6),
                gray5: rgb(0xE5E5EA),
                gray6: rgb(0xF2F2F7),
                background: rgb(0xFFFFFF),
                secondary_background: rgb(0xF6F6F6),
                tertiary_background: rgb(0xFFFFFF),
                grouped_background: rgb(0xFFFFFF),
                secondary_grouped_background: rgb(0xF6F6F6),
                label: rgba(0x000000, 0.847),
                secondary_label: rgba(0x000000, 0.498),
                tertiary_label: rgba(0x000000, 0.259),
                separator: rgba(0x000000, 0.098),
                opaque_separator: rgb(0xE6E6E6),
                fill: rgba(0x000000, 0.098),
                secondary_fill: rgba(0x000000, 0.078),
                tertiary_fill: rgba(0x000000, 0.047),
            }
        }
    }
}

/// The dark appearance variant Apple uses for a custom tint: a little lighter, same hue.
pub fn apple_dark_tint(tint: Color) -> Color {
    let mut lch = Oklcha::from(tint);
    lch.lightness = (lch.lightness + 0.04).min(0.92);
    Color::Oklcha(lch)
}

impl WingsColorScheme {
    /// iOS colors with the default blue tint.
    pub fn ios(dark: bool) -> Self {
        Self::apple(AppleSystemColors::ios(dark), None, dark)
    }

    /// macOS colors with the default blue tint.
    pub fn macos(dark: bool) -> Self {
        Self::apple(AppleSystemColors::macos(dark), None, dark)
    }

    /// Maps Apple system colors onto every built in role. `tint` replaces `systemBlue` as the
    /// accent (like `.tint()` in SwiftUI); in the dark appearance it is lightened a little.
    pub fn apple(system: AppleSystemColors, tint: Option<Color>, dark: bool) -> Self {
        let s = system;
        let tint = match tint {
            Some(t) if dark => apple_dark_tint(t),
            Some(t) => t,
            None => s.blue,
        };
        let readable_on = |fill: Color| {
            if contrast_ratio(fill, Color::WHITE) >= 3.0 {
                Color::WHITE
            } else {
                Color::BLACK
            }
        };
        let opaque_label = if dark { Color::WHITE } else { Color::BLACK };
        let container_alpha = if dark { 0.22 } else { 0.12 };
        let container = |c: Color| mix_srgb(s.background, c, container_alpha);
        // Content on a tinted container: the tint itself, nudged darker (light) or lighter
        // (dark) only as far as needed to stay readable.
        let on_container = |c: Color| {
            let fill = container(c);
            let mut lch = Oklcha::from(c);
            for _ in 0..20 {
                let candidate = Color::Oklcha(lch);
                if contrast_ratio(fill, candidate) >= 3.0 {
                    return candidate;
                }
                lch.lightness = if dark {
                    (lch.lightness + 0.02).min(1.0)
                } else {
                    (lch.lightness - 0.02).max(0.0)
                };
            }
            Color::Oklcha(lch)
        };
        let inverse_background = if dark { rgb(0xF2F2F7) } else { rgb(0x1C1C1E) };
        let colors = [
            // Primary family: the tint.
            tint,
            readable_on(tint),
            container(tint),
            on_container(tint),
            // Secondary family: neutral gray (Apple has no secondary brand color).
            s.gray,
            readable_on(s.gray),
            mix_srgb(s.background, s.gray, if dark { 0.32 } else { 0.16 }),
            opaque_label,
            // Tertiary family: indigo.
            s.indigo,
            readable_on(s.indigo),
            container(s.indigo),
            on_container(s.indigo),
            // Error family: red.
            s.red,
            readable_on(s.red),
            container(s.red),
            on_container(s.red),
            // Background and surfaces.
            s.background,
            s.label,
            s.background,
            s.label,
            s.secondary_background,
            s.secondary_label,
            s.secondary_grouped_background,
            s.secondary_background,
            s.gray6,
            s.gray5,
            s.gray4,
            inverse_background,
            if dark { Color::BLACK } else { Color::WHITE },
            if dark { s.blue } else { rgb(0x0091FF) },
            s.opaque_separator,
            s.separator,
            Color::BLACK,
            Color::BLACK,
            // Status.
            s.green,
            readable_on(s.green),
            s.orange,
            readable_on(s.orange),
            // Fills, labels and grouped backgrounds.
            s.fill,
            s.secondary_fill,
            s.tertiary_fill,
            s.tertiary_label,
            s.grouped_background,
            s.secondary_grouped_background,
        ];
        let colors = colors.map(|c| srgb_blend_alpha(c, s.background));
        Self {
            colors,
            custom: HashMap::default(),
        }
    }
}

/// Fluent 2 accent for a seed: the accent fill Windows uses in the light theme
/// (`SystemAccentColorDark1`) or the dark theme (`SystemAccentColorLight2`).
pub fn fluent_accent(seed: Color, dark: bool) -> Color {
    let mut lch = Oklcha::from(seed);
    lch.lightness = if dark {
        lch.lightness.max(0.82)
    } else {
        lch.lightness.min(0.5)
    };
    Color::Oklcha(lch)
}

impl WingsColorScheme {
    /// Fluent 2 (Windows 11, WinUI 3) colors. `accent` replaces the default Windows blue.
    ///
    /// Values are the WinUI 3 theme resources (`TextFillColorPrimary`,
    /// `ControlFillColorDefault`, `SolidBackgroundFillColorBase`, ...). Translucent control
    /// fills are kept translucent so they blend with Mica like on Windows.
    pub fn fluent(dark: bool, accent: Option<Color>) -> Self {
        let accent = match accent {
            Some(seed) => fluent_accent(seed, dark),
            None if dark => rgb(0x60CDFF),
            None => rgb(0x005FB8),
        };
        let on_accent = if contrast_ratio(accent, Color::WHITE) >= 3.0 {
            Color::WHITE
        } else {
            Color::BLACK
        };
        let (base, text, text2, text3) = if dark {
            (
                rgb(0x202020),
                rgb(0xFFFFFF),
                rgba(0xFFFFFF, 0.786),
                rgba(0xFFFFFF, 0.545),
            )
        } else {
            (
                rgb(0xF3F3F3),
                rgba(0x000000, 0.896),
                rgba(0x000000, 0.62),
                rgba(0x000000, 0.447),
            )
        };
        let pick = |light: Color, dark_c: Color| if dark { dark_c } else { light };
        let container = |c: Color| mix_srgb(base, c, if dark { 0.22 } else { 0.12 });
        let critical = pick(rgb(0xC42B1C), rgb(0xFF99A4));
        let success = pick(rgb(0x0F7B0F), rgb(0x6CCB5F));
        let caution = pick(rgb(0x9D5D00), rgb(0xFCE100));
        let purple = pick(rgb(0x8764B8), rgb(0xB696E2));
        let on = |fill: Color| {
            if contrast_ratio(fill, Color::WHITE) >= 3.0 {
                Color::WHITE
            } else {
                Color::BLACK
            }
        };
        let accent_text = pick(rgb(0x003E92), rgb(0x99EBFF));
        let colors = [
            accent,
            on_accent,
            container(accent),
            if accent_is_default(accent) {
                accent_text
            } else {
                readable(container(accent), accent, dark)
            },
            pick(rgb(0x5D5D5D), rgb(0xA0A0A0)),
            on(pick(rgb(0x5D5D5D), rgb(0xA0A0A0))),
            pick(rgb(0xEAEAEA), rgb(0x3A3A3A)),
            text,
            purple,
            on(purple),
            container(purple),
            readable(container(purple), purple, dark),
            critical,
            on(critical),
            pick(rgb(0xFDE7E9), rgb(0x442726)),
            pick(rgb(0xC42B1C), rgb(0xFF99A4)),
            base,
            text,
            pick(rgb(0xFBFBFB), rgb(0x2B2B2B)),
            text,
            pick(rgb(0xF9F9F9), rgb(0x282828)),
            text2,
            pick(rgb(0xFFFFFF), rgb(0x1C1C1C)),
            pick(rgb(0xF9F9F9), rgb(0x202020)),
            pick(rgb(0xF3F3F3), rgb(0x282828)),
            pick(rgb(0xEEEEEE), rgb(0x2C2C2C)),
            pick(rgb(0xE5E5E5), rgb(0x333333)),
            pick(rgb(0x2B2B2B), rgb(0xF3F3F3)),
            pick(rgb(0xFFFFFF), rgb(0x000000)),
            pick(rgb(0x60CDFF), rgb(0x005FB8)),
            pick(rgba(0x000000, 0.447), rgba(0xFFFFFF, 0.544)),
            pick(rgba(0x000000, 0.0803), rgba(0xFFFFFF, 0.0837)),
            Color::BLACK,
            Color::BLACK,
            success,
            on(success),
            caution,
            on(caution),
            pick(rgba(0x000000, 0.447), rgba(0xFFFFFF, 0.544)),
            pick(rgba(0x000000, 0.0373), rgba(0xFFFFFF, 0.0605)),
            pick(rgba(0x000000, 0.0241), rgba(0xFFFFFF, 0.0419)),
            text3,
            base,
            pick(rgb(0xFBFBFB), rgb(0x2B2B2B)),
        ];
        let colors = colors.map(|c| srgb_blend_alpha(c, base));
        Self {
            colors,
            custom: HashMap::default(),
        }
    }
}

/// True for the two default Windows accents.
fn accent_is_default(accent: Color) -> bool {
    let a = accent.to_srgba();
    [rgb(0x005FB8), rgb(0x60CDFF)].iter().any(|d| {
        let d = d.to_srgba();
        (a.red - d.red).abs() < 1e-3
            && (a.green - d.green).abs() < 1e-3
            && (a.blue - d.blue).abs() < 1e-3
    })
}

/// `color`, nudged darker (light) or lighter (dark) until it reads on `fill` at 3:1.
fn readable(fill: Color, color: Color, dark: bool) -> Color {
    let mut lch = Oklcha::from(color);
    for _ in 0..30 {
        let candidate = Color::Oklcha(lch);
        if contrast_ratio(fill, candidate) >= 3.0 {
            return candidate;
        }
        lch.lightness = if dark {
            (lch.lightness + 0.02).min(1.0)
        } else {
            (lch.lightness - 0.02).max(0.0)
        };
    }
    Color::Oklcha(lch)
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

    /// Apple schemes keep every content role readable on its container.
    #[test]
    fn apple_content_roles_have_contrast() {
        for scheme in [
            WingsColorScheme::ios(false),
            WingsColorScheme::ios(true),
            WingsColorScheme::macos(false),
            WingsColorScheme::macos(true),
        ] {
            for role in ColorRole::BUILT_IN {
                if let Some(on) = role.on() {
                    let ratio = contrast_ratio(scheme.get(role), scheme.get(on));
                    assert!(ratio >= 3.0, "{role:?} vs {on:?}: {ratio}");
                }
            }
        }
    }

    fn same(a: Color, b: Color) -> bool {
        let (a, b) = (a.to_srgba(), b.to_srgba());
        (a.red - b.red).abs() < 1e-4
            && (a.green - b.green).abs() < 1e-4
            && (a.blue - b.blue).abs() < 1e-4
            && (a.alpha - b.alpha).abs() < 1e-4
    }

    /// Fluent schemes keep content readable and use the Windows accents.
    #[test]
    fn fluent_scheme() {
        for dark in [false, true] {
            let scheme = WingsColorScheme::fluent(dark, None);
            for role in ColorRole::BUILT_IN {
                if let Some(on) = role.on() {
                    let ratio = contrast_ratio(scheme.get(role), scheme.get(on));
                    assert!(
                        ratio >= 3.0,
                        "fluent dark={dark} {role:?} vs {on:?}: {ratio}"
                    );
                }
            }
        }
        assert!(same(
            WingsColorScheme::fluent(false, None).get(ColorRole::Primary),
            rgb(0x005FB8)
        ));
        assert!(same(
            WingsColorScheme::fluent(true, None).get(ColorRole::OnPrimary),
            Color::BLACK
        ));
        let custom = WingsColorScheme::fluent(false, Some(Color::srgb(0.9, 0.2, 0.3)));
        assert!(
            contrast_ratio(
                custom.get(ColorRole::Primary),
                custom.get(ColorRole::OnPrimary)
            ) >= 3.0
        );
    }

    /// Apple schemes use the system colors and honour a custom tint.
    #[test]
    fn apple_scheme_maps_system_colors() {
        let light = WingsColorScheme::ios(false);
        assert!(same(light.get(ColorRole::Primary), rgb(0x0088FF)));
        assert!(same(light.get(ColorRole::Surface), Color::WHITE));
        assert!(same(light.get(ColorRole::GroupedBackground), rgb(0xF2F2F7)));
        assert!(same(light.get(ColorRole::Success), rgb(0x34C759)));
        let dark = WingsColorScheme::ios(true);
        assert!(same(dark.get(ColorRole::Background), Color::BLACK));
        let tinted = WingsColorScheme::apple(
            AppleSystemColors::ios(false),
            Some(Color::srgb(0.9, 0.1, 0.4)),
            false,
        );
        assert!(same(
            tinted.get(ColorRole::Primary),
            Color::srgb(0.9, 0.1, 0.4)
        ));
        // A light tint gets dark content.
        let yellow =
            WingsColorScheme::apple(AppleSystemColors::ios(false), Some(rgb(0xFFCC00)), false);
        assert!(same(yellow.get(ColorRole::OnPrimary), Color::BLACK));
    }

    /// A corrected alpha blended linearly over the background matches the sRGB blend.
    #[test]
    fn linear_alpha_matches_srgb_blend() {
        let label = rgba(0x000000, 0.85);
        let fixed = srgb_blend_alpha(label, Color::WHITE);
        assert!(fixed.alpha() > 0.95, "{}", fixed.alpha());
        // Linear blend of the fixed color over white.
        let a = fixed.alpha();
        let linear = Color::linear_rgb(1.0 - a, 1.0 - a, 1.0 - a);
        let want = mix_srgb(Color::WHITE, Color::BLACK, 0.85);
        assert!((relative_luminance(linear) - relative_luminance(want)).abs() < 1e-3);
        // White text on black needs less alpha in linear space.
        let dark = srgb_blend_alpha(rgba(0xFFFFFF, 0.55), Color::BLACK);
        assert!(dark.alpha() < 0.55);
        assert_eq!(srgb_blend_alpha(Color::WHITE, Color::BLACK), Color::WHITE);
    }

    /// sRGB mixing hits both ends.
    #[test]
    fn srgb_mix_ends() {
        assert!(same(
            mix_srgb(Color::WHITE, Color::BLACK, 0.0),
            Color::WHITE
        ));
        assert!(same(
            mix_srgb(Color::WHITE, Color::BLACK, 1.0),
            Color::BLACK
        ));
    }

    /// Role names round trip.
    #[test]
    fn role_names_round_trip() {
        for role in ColorRole::BUILT_IN {
            assert_eq!(ColorRole::from_name(role.name()), Some(role));
        }
        assert_eq!(ColorRole::OnSurfaceVariant.name(), "on_surface_variant");
        assert_eq!(ColorRole::from_name("nope"), None);
    }

    /// `BUILT_IN` contains unique roles.
    #[test]
    fn built_in_unique() {
        let set: std::collections::HashSet<_> = ColorRole::BUILT_IN.iter().collect();
        assert_eq!(set.len(), ColorRole::BUILT_IN.len());
    }
}
