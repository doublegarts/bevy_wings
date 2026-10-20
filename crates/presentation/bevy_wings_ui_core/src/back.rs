//! Back input shared by overlays and navigation.
//!
//! Escape, the browser back key, the gamepad East button (B, Circle) and the system back
//! (Android's back button and back gesture) close the topmost layer first: an open menu, then
//! a dialog, then a sheet. Only a press that no overlay used takes navigation back a screen, so
//! one press never does two things, and at the first screen the app stays open.
//!
//! Systems that close something on back run in [`WingsBackSystems::Overlays`] and call
//! [`WingsBack::consume`]; navigation runs in [`WingsBackSystems::Navigation`] and skips a
//! press that [`WingsBack::is_handled`].
//!
//! ```ignore
//! fn close_my_panel(mut back: ResMut<WingsBack>, mut panel: Single<&mut MyPanel>) {
//!     if panel.open && back.consume() {
//!         panel.open = false;
//!     }
//! }
//! app.add_systems(Update, close_my_panel.in_set(WingsBackSystems::Overlays));
//! ```

use bevy::ecs::schedule::SystemSet;
use bevy::ecs::system::{Query, Res, ResMut};
use bevy::input::ButtonInput;
use bevy::input::gamepad::{Gamepad, GamepadButton};
use bevy::input::keyboard::{Key, KeyCode};
use bevy::prelude::Resource;

/// Ordered steps of back handling inside [`WingsUiSystems::Input`](crate::WingsUiSystems).
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WingsBackSystems {
    /// Reads this frame's back input.
    Detect,
    /// Overlays (menus, dialogs, sheets, your own panels) close on back.
    Overlays,
    /// Navigation goes back when no overlay used the press.
    Navigation,
}

/// Where a back press came from.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum WingsBackSource {
    /// Escape or the browser back key of a keyboard.
    #[default]
    Keyboard,
    /// The gamepad East button (B, Circle).
    Gamepad,
    /// The operating system: Android's back button and back gesture.
    System,
    /// [`WingsBack::press`], for example from a toolbar button.
    Custom,
}

/// This frame's back press and whether something already used it.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct WingsBack {
    pressed: bool,
    handled: bool,
    queued: bool,
    source: WingsBackSource,
}

impl WingsBack {
    /// A back press that nothing used yet.
    pub fn pressed(&self) -> bool {
        self.pressed && !self.handled
    }

    /// Where this frame's back press came from.
    pub fn source(&self) -> WingsBackSource {
        self.source
    }

    /// Something used this frame's back press.
    pub fn is_handled(&self) -> bool {
        self.handled
    }

    /// Uses the back press. Returns `false` when there was none or it was already used.
    pub fn consume(&mut self) -> bool {
        let available = self.pressed();
        self.handled |= available;
        available
    }

    /// Requests a back press, handled from the next frame's [`WingsBackSystems::Detect`] on
    /// (for custom input such as a toolbar button, and for tests).
    pub fn press(&mut self) {
        self.queued = true;
    }
}

/// Reads Escape, the browser back key, the system back and the gamepad East button.
pub fn detect_back(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    logical: Option<Res<ButtonInput<Key>>>,
    gamepads: Query<&Gamepad>,
    mut back: ResMut<WingsBack>,
) {
    let keyboard =
        keys.is_some_and(|k| k.any_just_pressed([KeyCode::Escape, KeyCode::BrowserBack]));
    // Android's back button and back gesture: winit reports them only as the logical "browser
    // back" key, without a physical key code.
    let system =
        !keyboard && logical.is_some_and(|k| k.any_just_pressed([Key::BrowserBack, Key::GoBack]));
    let gamepad = gamepads.iter().any(|g| g.just_pressed(GamepadButton::East));
    let source = if back.queued {
        WingsBackSource::Custom
    } else if system {
        WingsBackSource::System
    } else if gamepad && !keyboard {
        WingsBackSource::Gamepad
    } else {
        WingsBackSource::Keyboard
    };
    *back = WingsBack {
        pressed: keyboard || system || gamepad || back.queued,
        handled: false,
        queued: false,
        source,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A press can be used once.
    #[test]
    fn consume_once() {
        let mut back = WingsBack {
            pressed: true,
            ..Default::default()
        };
        assert!(back.pressed());
        assert!(back.consume());
        assert!(!back.consume());
        assert!(back.is_handled());
        assert!(!WingsBack::default().clone().consume());
    }

    /// Android's back key (logical "browser back" with no physical key code) is a system back.
    #[test]
    fn android_back_is_system_back() {
        use bevy::app::{App, Update};
        use bevy::ecs::schedule::IntoScheduleConfigs;
        let mut app = App::new();
        app.init_resource::<WingsBack>()
            .init_resource::<ButtonInput<Key>>()
            .add_systems(Update, detect_back.in_set(WingsBackSystems::Detect));
        app.world_mut()
            .resource_mut::<ButtonInput<Key>>()
            .press(Key::BrowserBack);
        app.update();
        let back = *app.world().resource::<WingsBack>();
        assert!(back.pressed());
        assert_eq!(back.source(), WingsBackSource::System);
        // A request from code is a custom back.
        app.world_mut().resource_mut::<ButtonInput<Key>>().clear();
        app.world_mut().resource_mut::<WingsBack>().press();
        app.update();
        let back = *app.world().resource::<WingsBack>();
        assert!(back.pressed());
        assert_eq!(back.source(), WingsBackSource::Custom);
        app.update();
        assert!(!app.world().resource::<WingsBack>().pressed());
    }
}
