//! Gamepad (and arrow key) navigation.
//!
//! Every interactive Wings widget takes part in Bevy's automatic directional navigation, so a
//! gamepad can drive any Wings UI with no extra setup:
//!
//! | Input | Action |
//! | --- | --- |
//! | D-pad, left stick | move focus to the nearest widget in that direction |
//! | South (A, Cross) | activate the focused widget |
//! | East (B, Circle) | close the open popup or sheet (navigation pops the back stack) |
//!
//! Moving focus shows the design's focus ring and scrolls the focused widget into view.
//! Arrow keys move focus too unless a text field or slider has focus.

use bevy::input::gamepad::{Gamepad, GamepadButton};
use bevy::input_focus::InputFocusVisible;
use bevy::math::CompassOctant;
use bevy::prelude::*;
use bevy::ui::auto_directional_navigation::{AutoDirectionalNavigation, AutoDirectionalNavigator};
use bevy::ui_widgets::{Activate, ScrollIntoView, Slider};

use crate::interaction::WingsInteractive;
use crate::popup::WingsPopupOwner;
use crate::text_field::WingsTextFieldInput;

/// Gamepad navigation settings.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct WingsGamepadNavigation {
    /// Move focus with the gamepad.
    pub enabled: bool,
    /// Move focus with the arrow keys.
    pub arrow_keys: bool,
    /// Stick deflection that counts as a direction.
    pub stick_threshold: f32,
    /// Seconds between repeated moves while a direction is held.
    pub repeat: f32,
}

impl Default for WingsGamepadNavigation {
    fn default() -> Self {
        Self {
            enabled: true,
            arrow_keys: true,
            stick_threshold: 0.6,
            repeat: 0.18,
        }
    }
}

/// Makes interactive widgets reachable by directional navigation.
pub fn enable_directional_navigation(
    mut commands: Commands,
    added: Query<Entity, (Added<WingsInteractive>, Without<AutoDirectionalNavigation>)>,
) {
    for entity in &added {
        commands
            .entity(entity)
            .insert(AutoDirectionalNavigation::default());
    }
}

/// The compass direction of a stick or D-pad vector (y up), if it is deflected enough.
pub fn direction_of(v: Vec2, threshold: f32) -> Option<CompassOctant> {
    if v.length() < threshold {
        return None;
    }
    Some(if v.x.abs() > v.y.abs() {
        if v.x > 0.0 {
            CompassOctant::East
        } else {
            CompassOctant::West
        }
    } else if v.y > 0.0 {
        CompassOctant::North
    } else {
        CompassOctant::South
    })
}

/// Moves focus, activates and goes back from gamepads and arrow keys.
pub fn gamepad_navigation_system(
    mut commands: Commands,
    settings: Res<WingsGamepadNavigation>,
    time: Res<Time>,
    gamepads: Query<&Gamepad>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    mut visible: Option<ResMut<InputFocusVisible>>,
    mut navigator: AutoDirectionalNavigator,
    text_inputs: Query<(), Or<(With<WingsTextFieldInput>, With<Slider>)>>,
    mut owners: Query<&mut WingsPopupOwner>,
    mut cooldown: Local<f32>,
) {
    if !settings.enabled {
        return;
    }
    *cooldown -= time.delta_secs();
    let focused = navigator.manual_directional_navigation.focus.get();
    let mut direction = None;
    let mut activate = false;
    let mut back = false;
    for gamepad in &gamepads {
        let mut v = gamepad.left_stick();
        if gamepad.pressed(GamepadButton::DPadUp) {
            v.y = 1.0;
        }
        if gamepad.pressed(GamepadButton::DPadDown) {
            v.y = -1.0;
        }
        if gamepad.pressed(GamepadButton::DPadLeft) {
            v.x = -1.0;
        }
        if gamepad.pressed(GamepadButton::DPadRight) {
            v.x = 1.0;
        }
        direction = direction.or(direction_of(v, settings.stick_threshold));
        activate |= gamepad.just_pressed(GamepadButton::South);
        back |= gamepad.just_pressed(GamepadButton::East);
    }
    if settings.arrow_keys
        && direction.is_none()
        && !focused.is_some_and(|f| text_inputs.contains(f))
        && let Some(keys) = keys.as_ref()
    {
        let mut v = Vec2::ZERO;
        if keys.just_pressed(KeyCode::ArrowUp) {
            v.y = 1.0;
        } else if keys.just_pressed(KeyCode::ArrowDown) {
            v.y = -1.0;
        } else if keys.just_pressed(KeyCode::ArrowLeft) {
            v.x = -1.0;
        } else if keys.just_pressed(KeyCode::ArrowRight) {
            v.x = 1.0;
        }
        if let Some(d) = direction_of(v, 0.5) {
            direction = Some(d);
            *cooldown = 0.0;
        }
    }
    match direction {
        Some(d) if *cooldown <= 0.0 => {
            *cooldown = settings.repeat;
            if let Ok(target) = navigator.navigate(d) {
                if let Some(v) = visible.as_mut() {
                    v.0 = true;
                }
                commands.trigger(ScrollIntoView { entity: target });
            }
        }
        None => *cooldown = 0.0,
        _ => {}
    }
    if activate && let Some(entity) = focused {
        commands.trigger(Activate { entity });
    }
    if back {
        for mut owner in &mut owners {
            if owner.open {
                owner.open = false;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Stick vectors map to the dominant axis direction.
    #[test]
    fn directions() {
        assert_eq!(
            direction_of(Vec2::new(0.9, 0.1), 0.6),
            Some(CompassOctant::East)
        );
        assert_eq!(
            direction_of(Vec2::new(-0.2, -0.8), 0.6),
            Some(CompassOctant::South)
        );
        assert_eq!(direction_of(Vec2::new(0.1, 0.2), 0.6), None);
    }
}
