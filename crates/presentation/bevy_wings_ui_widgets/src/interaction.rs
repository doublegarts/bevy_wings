//! Shared interaction styling.
//!
//! Every clickable Wings widget carries [`WingsInteractive`]. One system turns the interaction
//! state (hovered, pressed, disabled) into a [`WingsStateLayer`] opacity and, for disabled
//! widgets, a reduced [`WingsOpacity`]. The theme painter then blends the state layer over the
//! widget fill, so hover and press feedback works identically for buttons, chips, list items
//! and navigation items, in light and dark schemes.
//!
//! The feedback itself comes from [`WingsPressFeedback`], which the skins choose per design:
//! state layers for Material, dimming for iOS, a darker fill for macOS.

use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::{InteractionDisabled, Pressed};

use bevy_wings_ui_animation::prelude::{AnimationSpec, OpacityProperty, WingsAnimateExt};
use bevy_wings_ui_core::WingsOpacity;
use bevy_wings_ui_theme::prelude::{WingsColorTransition, WingsStateLayer};

use crate::skin::WingsPressFeedback;

/// Opacity applied to disabled widgets.
pub const DISABLED_OPACITY: f32 = 0.38;

/// Marks a widget whose hover, press and disabled states are styled automatically.
#[derive(Component, Debug, Default, Clone, Copy)]
#[require(Hovered, WingsStateLayer, WingsColorTransition, WingsOpacity)]
pub struct WingsInteractive;

/// Computes the Material state layer opacity for an interaction state.
pub fn state_layer_opacity(hovered: bool, pressed: bool, disabled: bool) -> f32 {
    feedback_layer(&WingsPressFeedback::MATERIAL, hovered, pressed, disabled)
}

/// State layer opacity for an interaction state with a given feedback.
pub fn feedback_layer(
    feedback: &WingsPressFeedback,
    hovered: bool,
    pressed: bool,
    disabled: bool,
) -> f32 {
    if disabled {
        0.0
    } else if pressed {
        feedback.press_layer
    } else if hovered {
        feedback.hover_layer
    } else {
        0.0
    }
}

/// Widget opacity for an interaction state with a given feedback.
pub fn feedback_opacity(feedback: &WingsPressFeedback, pressed: bool, disabled: bool) -> f32 {
    if disabled {
        DISABLED_OPACITY
    } else if pressed {
        feedback.press_opacity
    } else {
        1.0
    }
}

type InteractiveChanged = Or<(
    Added<WingsInteractive>,
    Changed<Hovered>,
    Added<Pressed>,
    Added<InteractionDisabled>,
    Changed<WingsPressFeedback>,
)>;

/// Updates state layers and disabled opacity. Only widgets whose interaction state changed
/// this frame are visited.
pub fn interaction_style_system(
    mut commands: Commands,
    changed: Query<Entity, (With<WingsInteractive>, InteractiveChanged)>,
    mut widgets: Query<
        (
            &Hovered,
            Has<Pressed>,
            Has<InteractionDisabled>,
            Option<&WingsPressFeedback>,
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
        let Ok((hovered, pressed, disabled, feedback, mut layer, mut opacity)) =
            widgets.get_mut(entity)
        else {
            continue;
        };
        let feedback = feedback.copied().unwrap_or_default();
        let o = feedback_layer(&feedback, hovered.0, pressed, disabled);
        if layer.opacity != o {
            layer.opacity = o;
        }
        let target = feedback_opacity(&feedback, pressed, disabled);
        if feedback.press_opacity < 1.0 && !disabled {
            // Apple style dimming eases in and out instead of jumping.
            let spec = if pressed {
                AnimationSpec::millis(80)
            } else {
                AnimationSpec::millis(220)
            };
            commands
                .entity(entity)
                .animate::<OpacityProperty>(target, spec);
        } else if opacity.0 != target {
            opacity.0 = target;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Dimming feedback only changes opacity.
    #[test]
    fn dim_feedback() {
        let f = WingsPressFeedback::DIM;
        assert_eq!(feedback_layer(&f, true, true, false), 0.0);
        assert_eq!(feedback_opacity(&f, true, false), f.press_opacity);
        assert_eq!(feedback_opacity(&f, false, false), 1.0);
        assert_eq!(feedback_opacity(&f, true, true), DISABLED_OPACITY);
    }

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
