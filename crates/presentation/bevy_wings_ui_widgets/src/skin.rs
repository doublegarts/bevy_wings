//! Design skins.
//!
//! Widget scenes describe *what* a widget is (a filled button labelled "Save", a checked
//! switch). How it looks depends on the active [`WingsDesign`]: Material, iOS or macOS. Skin
//! systems run in [`WingsUiSystems::Skin`](bevy_wings_ui_core::WingsUiSystems::Skin) and apply
//! the design's sizes, shapes, palettes and decorative parts:
//!
//! * to every newly spawned widget, and
//! * to every live widget when [`WingsTheme::design`] changes, so an app can switch between the
//!   Material, iOS and macOS looks at runtime.
//!
//! Skins only write the [`Node`] fields they own and run before the modifier set, so
//! [`WingsModifier`]s still win. Decorative children
//! created by a skin carry [`WingsSkinPart`] and are rebuilt when the design changes.

use bevy::prelude::*;

use bevy_wings_ui_modifiers::WingsModifier;
use bevy_wings_ui_theme::prelude::{WingsDesign, WingsStateLayer, WingsTheme};

/// The design the widgets are currently skinned for. Follows [`WingsTheme::design`] and only
/// changes (which re-skins every widget) when the design really changes.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WingsSkin {
    /// Active design.
    pub design: WingsDesign,
}

impl WingsSkin {
    /// True for the iOS design.
    pub fn ios(&self) -> bool {
        self.design == WingsDesign::Ios
    }

    /// True for the macOS design.
    pub fn macos(&self) -> bool {
        self.design == WingsDesign::MacOs
    }

    /// True for the Fluent design.
    pub fn fluent(&self) -> bool {
        self.design == WingsDesign::Fluent
    }

    /// True for both Apple designs.
    pub fn apple(&self) -> bool {
        self.design.is_apple()
    }

    /// Picks a value per design.
    pub fn pick<T>(&self, material: T, ios: T, macos: T, fluent: T) -> T {
        pick(self.design, material, ios, macos, fluent)
    }
}

/// Picks a value per design.
pub fn pick<T>(design: WingsDesign, material: T, ios: T, macos: T, fluent: T) -> T {
    match design {
        WingsDesign::Material => material,
        WingsDesign::Ios => ios,
        WingsDesign::MacOs => macos,
        WingsDesign::Fluent => fluent,
    }
}

/// Marks a decorative child created by a skin (a list chevron, a spinner spoke). Skin parts
/// are despawned and rebuilt when the design changes.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct WingsSkinPart;

/// How a widget reacts to hover and press. Skins choose it per design; the shared
/// interaction system applies it.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct WingsPressFeedback {
    /// State layer opacity while hovered.
    pub hover_layer: f32,
    /// State layer opacity while pressed.
    pub press_layer: f32,
    /// Widget opacity while pressed (`1.0` for no dimming).
    pub press_opacity: f32,
}

impl Default for WingsPressFeedback {
    fn default() -> Self {
        Self::MATERIAL
    }
}

impl WingsPressFeedback {
    /// Material: hover and press state layers.
    pub const MATERIAL: Self = Self {
        hover_layer: WingsStateLayer::HOVER,
        press_layer: WingsStateLayer::PRESSED,
        press_opacity: 1.0,
    };
    /// iOS controls: no hover, the whole control dims while pressed.
    pub const DIM: Self = Self {
        hover_layer: 0.0,
        press_layer: 0.0,
        press_opacity: 0.55,
    };
    /// macOS controls: no hover, the fill darkens while held.
    pub const DARKEN: Self = Self {
        hover_layer: 0.0,
        press_layer: 0.14,
        press_opacity: 1.0,
    };
    /// Fluent controls: a subtle fill on hover, lighter content while pressed.
    pub const FLUENT: Self = Self {
        hover_layer: 0.04,
        press_layer: 0.02,
        press_opacity: 0.8,
    };
    /// Apple list rows: a gray highlight while pressed.
    pub const HIGHLIGHT: Self = Self {
        hover_layer: 0.0,
        press_layer: 0.12,
        press_opacity: 1.0,
    };

    /// Default feedback of controls in a design.
    pub fn for_design(design: WingsDesign) -> Self {
        match design {
            WingsDesign::Material => Self::MATERIAL,
            WingsDesign::Ios => Self::DIM,
            WingsDesign::MacOs => Self::DARKEN,
            WingsDesign::Fluent => Self::FLUENT,
        }
    }
}

/// Keeps [`WingsSkin`] in sync with the theme's design.
pub fn sync_skin_design(theme: Res<WingsTheme>, mut skin: ResMut<WingsSkin>) {
    if skin.design != theme.design {
        skin.design = theme.design;
    }
}

/// Entities a skin system must visit this frame: every widget of the kind after a design
/// change, otherwise only the newly spawned ones.
pub(crate) fn skin_targets(
    skin: &Res<WingsSkin>,
    added: impl IntoIterator<Item = Entity>,
    all: impl IntoIterator<Item = Entity>,
) -> Vec<Entity> {
    if skin.is_changed() {
        all.into_iter().collect()
    } else {
        added.into_iter().collect()
    }
}

/// Despawns the [`WingsSkinPart`] children of `entity`.
pub(crate) fn clear_skin_parts(
    commands: &mut Commands,
    entity: Entity,
    children: &Query<&Children>,
    parts: &Query<(), With<WingsSkinPart>>,
) {
    if let Ok(children) = children.get(entity) {
        for child in children.iter() {
            if parts.contains(child) {
                commands.entity(child).despawn();
            }
        }
    }
}

/// After a design change, folds every modifier again so user modifiers win over skins.
pub fn refold_modifiers_after_skin(skin: Res<WingsSkin>, mut modifiers: Query<&mut WingsModifier>) {
    if skin.is_changed() && !skin.is_added() {
        for mut m in &mut modifiers {
            m.set_changed();
        }
    }
}

/// Gives interactive widgets the press feedback of the design.
pub fn skin_press_feedback(
    mut commands: Commands,
    skin: Res<WingsSkin>,
    added: Query<Entity, Added<crate::interaction::WingsInteractive>>,
    all: Query<Entity, With<crate::interaction::WingsInteractive>>,
) {
    let feedback = WingsPressFeedback::for_design(skin.design);
    for entity in skin_targets(&skin, added, all) {
        commands.entity(entity).insert(feedback);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `pick` selects per design.
    #[test]
    fn pick_per_design() {
        let s = |design| WingsSkin { design };
        assert_eq!(s(WingsDesign::Material).pick(1, 2, 3, 4), 1);
        assert_eq!(s(WingsDesign::Ios).pick(1, 2, 3, 4), 2);
        assert_eq!(s(WingsDesign::MacOs).pick(1, 2, 3, 4), 3);
        assert_eq!(s(WingsDesign::Fluent).pick(1, 2, 3, 4), 4);
        assert!(s(WingsDesign::MacOs).apple() && !s(WingsDesign::Material).apple());
    }

    /// Apple feedback never shows hover layers.
    #[test]
    fn apple_feedback_has_no_hover() {
        for d in [WingsDesign::Ios, WingsDesign::MacOs] {
            assert_eq!(WingsPressFeedback::for_design(d).hover_layer, 0.0);
        }
        const { assert!(WingsPressFeedback::DIM.press_opacity < 1.0) };
    }
}
