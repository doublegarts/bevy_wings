//! Focus rings for keyboard navigation.
//!
//! When the keyboard moves focus (Tab, arrow keys, gamepad), the focused widget gets the
//! focus ring of the active design. Pointer clicks do not show rings, like on every platform.
//!
//! | Design | Ring |
//! | --- | --- |
//! | Material | 3 px primary outline, 2 px away |
//! | iOS | 3 px tint outline, 2 px away (iPad keyboards) |
//! | macOS | soft accent ring hugging the control |
//! | Fluent | 2 px contrast ring, 1 px away |

use bevy::input_focus::{InputFocus, InputFocusVisible};
use bevy::prelude::*;

use bevy_wings_ui_theme::prelude::{ColorRole, WingsDesign, WingsTheme};

use crate::text_field::WingsTextFieldInput;

/// A focus ring drawn by Wings (so it can be removed when focus leaves).
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct WingsFocusRing;

/// Opts a widget out of the focus ring (it shows focus itself).
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct WingsNoFocusRing;

/// Ring of a design: `(width, offset, color role, alpha)`.
pub fn focus_ring_style(design: WingsDesign) -> (f32, f32, ColorRole, f32) {
    match design {
        WingsDesign::Material => (3.0, 2.0, ColorRole::Primary, 1.0),
        WingsDesign::Ios => (3.0, 2.0, ColorRole::Primary, 0.8),
        WingsDesign::MacOs => (3.0, 1.0, ColorRole::Primary, 0.5),
        WingsDesign::Fluent => (2.0, 1.0, ColorRole::OnSurface, 1.0),
    }
}

/// Moves the focus ring to the keyboard focused entity.
pub fn focus_ring_system(
    mut commands: Commands,
    focus: Option<Res<InputFocus>>,
    visible: Option<Res<InputFocusVisible>>,
    theme: Res<WingsTheme>,
    rings: Query<Entity, With<WingsFocusRing>>,
    candidates: Query<
        (),
        (
            With<Node>,
            Without<WingsNoFocusRing>,
            Without<WingsTextFieldInput>,
        ),
    >,
    mut last: Local<Option<(Entity, Color)>>,
) {
    let target = match (&focus, &visible) {
        (Some(focus), Some(visible)) if visible.0 => {
            focus.get().filter(|e| candidates.contains(*e))
        }
        _ => None,
    };
    let (width, offset, role, alpha) = focus_ring_style(theme.design);
    let color = theme.color(role).with_alpha(alpha);
    if *last == target.map(|e| (e, color)) {
        return;
    }
    for ring in &rings {
        if Some(ring) != target {
            commands
                .entity(ring)
                .remove::<WingsFocusRing>()
                .insert(Outline {
                    width: px(0),
                    offset: px(0),
                    color: Color::NONE,
                });
        }
    }
    if let Some(entity) = target {
        commands.entity(entity).insert((
            WingsFocusRing,
            Outline {
                width: px(width),
                offset: px(offset),
                color,
            },
        ));
    }
    *last = target.map(|e| (e, color));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// macOS draws a soft ring, Fluent a contrast ring.
    #[test]
    fn ring_styles() {
        assert!(focus_ring_style(WingsDesign::MacOs).3 < 1.0);
        assert_eq!(
            focus_ring_style(WingsDesign::Fluent).2,
            ColorRole::OnSurface
        );
    }
}
