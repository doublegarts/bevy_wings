//! Shared interaction styling.
//!
//! Every clickable Wings widget carries [`WingsInteractive`]. One system turns the interaction
//! state (hovered, pressed, disabled) into a [`WingsStateLayer`] opacity and, for disabled
//! widgets, a reduced [`WingsOpacity`]. The theme painter then blends the state layer over the
//! widget fill, so hover and press feedback works identically for buttons, chips, list items
//! and navigation items, in light and dark schemes.

use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::{InteractionDisabled, Pressed};

use bevy_wings_ui_core::WingsOpacity;
use bevy_wings_ui_theme::prelude::{WingsColorTransition, WingsStateLayer};

/// Opacity applied to disabled widgets.
pub const DISABLED_OPACITY: f32 = 0.38;

/// Marks a widget whose hover, press and disabled states are styled automatically.
#[derive(Component, Debug, Default, Clone, Copy)]
#[require(Hovered, WingsStateLayer, WingsColorTransition, WingsOpacity)]
pub struct WingsInteractive;

/// Computes the state layer opacity for an interaction state.
pub fn state_layer_opacity(hovered: bool, pressed: bool, disabled: bool) -> f32 {
    if disabled {
        0.0
    } else if pressed {
        WingsStateLayer::PRESSED
    } else if hovered {
        WingsStateLayer::HOVER
    } else {
        0.0
    }
}

type InteractiveChanged = Or<(
    Added<WingsInteractive>,
    Changed<Hovered>,
    Added<Pressed>,
    Added<InteractionDisabled>,
)>;

/// Updates state layers and disabled opacity. Only widgets whose interaction state changed
/// this frame are visited.
pub fn interaction_style_system(
    changed: Query<Entity, (With<WingsInteractive>, InteractiveChanged)>,
    mut widgets: Query<
        (
            &Hovered,
            Has<Pressed>,
            Has<InteractionDisabled>,
            &mut WingsStateLayer,
            &mut WingsOpacity,
        ),
        With<WingsInteractive>,
    >,
    mut removed_pressed: RemovedComponents<Pressed>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
) {
    let dirty = changed
        .iter()
        .chain(removed_pressed.read())
        .chain(removed_disabled.read());
    for entity in dirty {
        let Ok((hovered, pressed, disabled, mut layer, mut opacity)) = widgets.get_mut(entity)
        else {
            continue;
        };
        let o = state_layer_opacity(hovered.0, pressed, disabled);
        if layer.opacity != o {
            layer.opacity = o;
        }
        let target = if disabled { DISABLED_OPACITY } else { 1.0 };
        if opacity.0 != target {
            opacity.0 = target;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pressed wins over hovered, disabled suppresses everything.
    #[test]
    fn opacity_priorities() {
        assert_eq!(state_layer_opacity(false, false, false), 0.0);
        assert_eq!(
            state_layer_opacity(true, false, false),
            WingsStateLayer::HOVER
        );
        assert_eq!(
            state_layer_opacity(true, true, false),
            WingsStateLayer::PRESSED
        );
        assert_eq!(state_layer_opacity(true, true, true), 0.0);
    }
}
