//! # bevy_wings_ui_core
//!
//! Shared foundation for every Wings UI crate:
//!
//! * [`WingsUiSystems`]: the ordered system sets that all Wings plugins schedule into, so that
//!   state changes, navigation, responsiveness, styling, theming and animation always run in a
//!   predictable order inside a single frame.
//! * [`WingsState`] and [`WingsMemo`]: tiny, allocation free reactive primitives that track a
//!   version number. They are the building blocks used by view models and bindings to skip work
//!   when nothing changed.
//! * [`WingsOpacity`]: a subtree opacity multiplier understood by the theme painter and the
//!   animation system (Bevy UI has no node opacity, Wings adds one).
//! * [`back`]: back input (Escape, browser back, gamepad B) shared by overlays and navigation,
//!   so one press closes the topmost menu, dialog or sheet before navigation goes back.
//! * [`hierarchy`]: helpers to walk up the entity hierarchy (for example to find the closest
//!   ancestor that owns a view model or a theme override).
//!
//! Add [`WingsCorePlugin`] once (the umbrella `WingsUiPlugins` group does this for you).

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/doublegarts/bevy_wings/main/assets/branding/bevy-wings-icon.png",
    html_favicon_url = "https://raw.githubusercontent.com/doublegarts/bevy_wings/main/assets/branding/bevy-wings-icon.png"
)]

pub mod back;
pub mod hierarchy;
pub mod opacity;
pub mod state;

use bevy::app::{App, Plugin, Update};
use bevy::ecs::schedule::{IntoScheduleConfigs, SystemSet};

pub use opacity::WingsOpacity;
pub use state::{WingsMemo, WingsState};

/// Ordered system sets used by every Wings UI plugin inside the [`Update`] schedule.
///
/// The sets are chained in declaration order, which gives the following frame pipeline:
///
/// 1. [`WingsUiSystems::Input`]: translate raw input (keys, back button) into Wings events.
/// 2. [`WingsUiSystems::ViewModel`]: user view model logic and intents.
/// 3. [`WingsUiSystems::Bind`]: push view model state into bound widgets.
/// 4. [`WingsUiSystems::Navigation`]: react to back stack changes and spawn or hide screens.
/// 5. [`WingsUiSystems::Responsive`]: compute window size classes and responsive values.
/// 6. [`WingsUiSystems::Skin`]: widgets apply the active design's metrics and parts.
/// 7. [`WingsUiSystems::Modifier`]: fold modifier chains into Bevy UI components.
/// 8. [`WingsUiSystems::Style`]: widgets compute their state dependent paint (hover, pressed).
/// 9. [`WingsUiSystems::Theme`]: resolve theme roles into concrete colors.
/// 10. [`WingsUiSystems::Animate`]: advance tweens and springs and write animated values.
///
/// User systems can be ordered relative to these sets, for example
/// `app.add_systems(Update, my_system.in_set(WingsUiSystems::ViewModel))`.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WingsUiSystems {
    /// Raw input translation.
    Input,
    /// View model logic.
    ViewModel,
    /// View model to widget bindings.
    Bind,
    /// Navigation back stack processing.
    Navigation,
    /// Responsive layout evaluation.
    Responsive,
    /// Design skins (Material, iOS, macOS) for newly spawned widgets and design changes.
    Skin,
    /// Modifier application.
    Modifier,
    /// Widget state styling.
    Style,
    /// Theme color resolution.
    Theme,
    /// Animation stepping.
    Animate,
}

impl WingsUiSystems {
    /// All sets in execution order.
    pub const ORDER: [WingsUiSystems; 10] = [
        WingsUiSystems::Input,
        WingsUiSystems::ViewModel,
        WingsUiSystems::Bind,
        WingsUiSystems::Navigation,
        WingsUiSystems::Responsive,
        WingsUiSystems::Skin,
        WingsUiSystems::Modifier,
        WingsUiSystems::Style,
        WingsUiSystems::Theme,
        WingsUiSystems::Animate,
    ];
}

/// Configures the [`WingsUiSystems`] ordering. Safe to add more than once: every Wings plugin
/// adds it on demand when it is missing.
#[derive(Default)]
pub struct WingsCorePlugin;

impl Plugin for WingsCorePlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            Update,
            (
                WingsUiSystems::Input,
                WingsUiSystems::ViewModel,
                WingsUiSystems::Bind,
                WingsUiSystems::Navigation,
                WingsUiSystems::Responsive,
                WingsUiSystems::Skin,
                WingsUiSystems::Modifier,
                WingsUiSystems::Style,
                WingsUiSystems::Theme,
                WingsUiSystems::Animate,
            )
                .chain(),
        )
        .configure_sets(
            Update,
            (
                back::WingsBackSystems::Detect,
                back::WingsBackSystems::Overlays,
                back::WingsBackSystems::Navigation,
            )
                .chain()
                .in_set(WingsUiSystems::Input),
        )
        .init_resource::<back::WingsBack>()
        .add_systems(
            Update,
            back::detect_back.in_set(back::WingsBackSystems::Detect),
        );
    }

    fn is_unique(&self) -> bool {
        true
    }
}

/// Adds [`WingsCorePlugin`] to the app if it has not been added yet.
///
/// Every Wings plugin calls this in its `build`, which lets you add individual Wings plugins in
/// any order without worrying about the shared configuration.
pub fn ensure_core(app: &mut App) {
    if !app.is_plugin_added::<WingsCorePlugin>() {
        app.add_plugins(WingsCorePlugin);
    }
}

/// Commonly used items.
pub mod prelude {
    pub use crate::back::{WingsBack, WingsBackSource, WingsBackSystems};
    pub use crate::hierarchy::{ancestors, find_ancestor_with};
    pub use crate::{WingsCorePlugin, WingsMemo, WingsOpacity, WingsState, WingsUiSystems};
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;

    #[derive(Resource, Default)]
    struct Log(Vec<&'static str>);

    /// The sets must run in the documented order, whatever order systems were added in.
    #[test]
    fn sets_run_in_documented_order() {
        let mut app = App::new();
        app.add_plugins(WingsCorePlugin).init_resource::<Log>();
        app.add_systems(
            Update,
            (
                (|mut l: ResMut<Log>| l.0.push("animate")).in_set(WingsUiSystems::Animate),
                (|mut l: ResMut<Log>| l.0.push("theme")).in_set(WingsUiSystems::Theme),
                (|mut l: ResMut<Log>| l.0.push("input")).in_set(WingsUiSystems::Input),
                (|mut l: ResMut<Log>| l.0.push("nav")).in_set(WingsUiSystems::Navigation),
            ),
        );
        app.update();
        assert_eq!(
            app.world().resource::<Log>().0,
            vec!["input", "nav", "theme", "animate"]
        );
    }

    /// `ensure_core` can be called many times without panicking on duplicate plugins.
    #[test]
    fn ensure_core_is_idempotent() {
        let mut app = App::new();
        ensure_core(&mut app);
        ensure_core(&mut app);
        assert!(app.is_plugin_added::<WingsCorePlugin>());
    }

    /// The `ORDER` constant lists every set exactly once.
    #[test]
    fn order_constant_is_complete() {
        let mut seen = std::collections::HashSet::new();
        for s in WingsUiSystems::ORDER {
            assert!(seen.insert(s));
        }
        assert_eq!(seen.len(), 10);
    }
}
