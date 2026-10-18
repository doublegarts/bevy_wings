//! Design languages: Material, Cupertino (iOS and macOS, SwiftUI style) and Fluent (Windows).
//!
//! A [`WingsDesign`] decides which token sets a [`WingsTheme`](crate::theme::WingsTheme) uses
//! (colors, type scale, shapes, spacing, elevation, motion) and which skin widgets apply. The
//! default theme picks the platform's own design through [`WingsDesign::native`]:
//!
//! | Platform | Design |
//! | --- | --- |
//! | iPhone, iPad, Vision | [`WingsDesign::Ios`] (Cupertino) |
//! | Mac | [`WingsDesign::MacOs`] (Cupertino) |
//! | Windows | [`WingsDesign::Fluent`] |
//! | Android, Linux, web, others | [`WingsDesign::Material`] |

use bevy::prelude::*;

/// A design language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Reflect)]
pub enum WingsDesign {
    /// Material style (the Wings 0.1 look). Default outside Apple platforms.
    #[default]
    Material,
    /// SwiftUI on iPhone and iPad (Liquid Glass era).
    Ios,
    /// SwiftUI on Mac (Liquid Glass era).
    MacOs,
    /// Fluent 2 as in Windows 11 and WinUI 3.
    Fluent,
}

impl WingsDesign {
    /// Every design, in declaration order.
    pub const ALL: [WingsDesign; 4] = [
        WingsDesign::Material,
        WingsDesign::Ios,
        WingsDesign::MacOs,
        WingsDesign::Fluent,
    ];

    /// The design of the platform the app was compiled for: Cupertino on Apple platforms
    /// ([`MacOs`](Self::MacOs) on macOS, [`Ios`](Self::Ios) on iOS, iPadOS and visionOS),
    /// [`Fluent`](Self::Fluent) on Windows and [`Material`](Self::Material) on Android, Linux
    /// and everything else.
    pub const fn native() -> Self {
        if cfg!(target_os = "macos") {
            Self::MacOs
        } else if cfg!(any(target_os = "ios", target_os = "visionos")) {
            Self::Ios
        } else if cfg!(target_os = "windows") {
            Self::Fluent
        } else {
            Self::Material
        }
    }

    /// The Cupertino design of the current platform: [`MacOs`](Self::MacOs) on macOS,
    /// [`Ios`](Self::Ios) everywhere else.
    pub const fn cupertino() -> Self {
        if cfg!(target_os = "macos") {
            Self::MacOs
        } else {
            Self::Ios
        }
    }

    /// True for the Apple (Cupertino) designs.
    pub const fn is_apple(self) -> bool {
        matches!(self, Self::Ios | Self::MacOs)
    }

    /// True for designs made for pointer first desktops (hover feedback, denser controls).
    pub const fn is_desktop(self) -> bool {
        matches!(self, Self::MacOs | Self::Fluent)
    }

    /// Short human readable name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Material => "Material",
            Self::Ios => "iOS",
            Self::MacOs => "macOS",
            Self::Fluent => "Fluent",
        }
    }

    /// Parses a design name as used in `wings.toml`: `material`, `cupertino` (the platform's
    /// Cupertino variant), `ios`, `macos` or `fluent`. Case insensitive.
    pub fn parse(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "material" | "android" => Some(Self::Material),
            "cupertino" | "apple" => Some(Self::cupertino()),
            "ios" | "cupertino-ios" | "ipados" => Some(Self::Ios),
            "macos" | "mac" | "cupertino-macos" => Some(Self::MacOs),
            "fluent" | "windows" | "winui" => Some(Self::Fluent),
            _ => None,
        }
    }

    /// The next design in [`ALL`](Self::ALL), wrapping around. Handy for preview switches.
    pub fn next(self) -> Self {
        let i = Self::ALL.iter().position(|d| *d == self).unwrap_or(0);
        Self::ALL[(i + 1) % Self::ALL.len()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The native design matches the compile target.
    #[test]
    fn native_matches_target() {
        let d = WingsDesign::native();
        if cfg!(target_os = "macos") {
            assert_eq!(d, WingsDesign::MacOs);
        } else if cfg!(target_os = "ios") {
            assert_eq!(d, WingsDesign::Ios);
        } else if cfg!(target_os = "windows") {
            assert_eq!(d, WingsDesign::Fluent);
        } else {
            assert_eq!(d, WingsDesign::Material);
        }
    }

    /// `next` cycles through every design.
    #[test]
    fn next_cycles() {
        let mut d = WingsDesign::Material;
        for _ in 0..WingsDesign::ALL.len() {
            d = d.next();
        }
        assert_eq!(d, WingsDesign::Material);
        assert!(WingsDesign::Ios.is_apple() && !WingsDesign::Fluent.is_apple());
        assert!(WingsDesign::Fluent.is_desktop() && !WingsDesign::Ios.is_desktop());
    }

    /// Names from `wings.toml` parse.
    #[test]
    fn parse_names() {
        assert_eq!(WingsDesign::parse("Fluent"), Some(WingsDesign::Fluent));
        assert_eq!(WingsDesign::parse("ios"), Some(WingsDesign::Ios));
        assert_eq!(
            WingsDesign::parse("cupertino"),
            Some(WingsDesign::cupertino())
        );
        assert_eq!(WingsDesign::parse("nope"), None);
    }
}
