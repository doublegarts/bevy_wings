//! # bevy_wings
//!
//! A layered app framework for **Bevy 0.20**, split into three layers plus shared
//! infrastructure:
//!
//! * **Domain** ([`domain`]): entities, typed ids, use cases, validation, repository contracts
//!   and loading states. Plain Rust, no engine dependency.
//! * **Data** ([`data`]): data sources, offline first repositories with caching policies,
//!   mappers and ECS integrated background loading.
//! * **Presentation**: `bsn!` widgets, an ECS theme, animation, responsiveness, modifiers,
//!   custom WESL UI shaders, MVVM and navigation.
//! * **Shared** ([`di`]): dependency injection that wires the layers together.
//!
//! | Module | Crate | Layer | Purpose |
//! | --- | --- | --- | --- |
//! | [`domain`] | `bevy_wings_domain` | Domain | Entities, use cases, validation, repository traits, `Loadable` |
//! | [`data`] | `bevy_wings_data` | Data | Sources, offline first repositories, caching, background loads |
//! | [`di`] | `bevy_wings_di` | Shared | Modules, singletons, factories, scopes, `Inject` |
//! | [`foundation`] | `bevy_wings_ui_core` | Presentation | System sets, reactive state, opacity, hierarchy helpers |
//! | [`animation`] | `bevy_wings_ui_animation` | Presentation | Tweens, springs, keyframes, animated visibility |
//! | [`theme`] | `bevy_wings_ui_theme` | Presentation | Color schemes, typography, shapes, elevation, ECS painter |
//! | [`responsive`] | `bevy_wings_ui_responsive` | Presentation | Width classes, responsive nodes and visibility |
//! | [`modifier`] | `bevy_wings_ui_modifiers` | Presentation | Chainable layout and style modifiers |
//! | [`widgets`] | `bevy_wings_ui_widgets` | Presentation | `WingsButton`, `WingsText`, `WingsCard` and friends |
//! | [`mvvm`] | `bevy_wings_ui_mvvm` | Presentation | View models, bindings, one shot effects |
//! | [`navigation`] | `bevy_wings_ui_navigation` | Presentation | Back stack, nav display, adaptive panes |
//! | `shaders` | `bevy_wings_ui_shaders` | Presentation | Ripple, shimmer, circular progress, aurora, glass |
//!
//! ## Quick start
//!
//! ```ignore
//! use bevy::prelude::*;
//! use bevy_wings::prelude::*;
//!
//! fn main() {
//!     App::new()
//!         .add_plugins(DefaultPlugins)
//!         .add_plugins(WingsUiPlugins)
//!         .add_systems(Startup, setup)
//!         .run();
//! }
//!
//! fn setup(mut commands: Commands) {
//!     commands.spawn(Camera2d);
//!     let content = modifier().fill_max_width().padding(24.0);
//!     commands.spawn_scene(bsn! {
//!         @WingsScaffold
//!         Children [
//!             @WingsTopAppBar { @title: "Wings" }
//!             --
//!             @WingsColumn { @gap: 12.0 }
//!             content
//!             Children [
//!                 @WingsText { @text: "Hello Bevy", @role: TypeRole::HeadlineMedium }
//!                 --
//!                 @WingsButton { @text: "Toggle theme" }
//!                 on(|_: On<Activate>, mut theme: ResMut<WingsTheme>| theme.toggle_dark())
//!             ]
//!         ]
//!     });
//! }
//! ```
//!
//! Add [`WingsUiPlugins`] after `DefaultPlugins`, because the widgets reuse the headless widget
//! plugins that `DefaultPlugins` already registers.

pub use bevy_wings_data as data;
pub use bevy_wings_di as di;
pub use bevy_wings_domain as domain;
pub use bevy_wings_ui_animation as animation;
pub use bevy_wings_ui_core as foundation;
pub use bevy_wings_ui_modifiers as modifier;
pub use bevy_wings_ui_mvvm as mvvm;
pub use bevy_wings_ui_navigation as navigation;
pub use bevy_wings_ui_responsive as responsive;
#[cfg(feature = "shaders")]
pub use bevy_wings_ui_shaders as shaders;
pub use bevy_wings_ui_theme as theme;
pub use bevy_wings_ui_widgets as widgets;

use bevy::app::{PluginGroup, PluginGroupBuilder};

/// Every Wings plugin that needs no configuration: core, animation, theme, responsive,
/// modifier and widgets (plus shaders with the `shaders` feature).
///
/// Dependency injection ([`di::WingsDiPlugin`]) and navigation
/// ([`navigation::WingsNavigationPlugin`]) take app specific configuration and are added
/// separately. Configure members with the usual plugin group API:
///
/// ```ignore
/// app.add_plugins(
///     WingsUiPlugins.set(WingsThemePlugin::with_theme(WingsTheme::from_seed(Color::srgb(0.9, 0.3, 0.2)))),
/// );
/// ```
pub struct WingsUiPlugins;

impl PluginGroup for WingsUiPlugins {
    fn build(self) -> PluginGroupBuilder {
        let group = PluginGroupBuilder::start::<Self>()
            .add(foundation::WingsCorePlugin)
            .add(animation::WingsAnimationPlugin)
            .add(theme::WingsThemePlugin::default())
            .add(responsive::WingsResponsivePlugin::default())
            .add(modifier::WingsModifierPlugin);
        #[cfg(feature = "shaders")]
        let group = group.add(shaders::WingsShadersPlugin);
        group.add(widgets::WingsWidgetsPlugin::default())
    }
}

/// Everything needed to build apps with Wings (all layers), in one import.
pub mod prelude {
    pub use crate::WingsUiPlugins;
    pub use bevy_wings_data::prelude::*;
    pub use bevy_wings_di::prelude::*;
    pub use bevy_wings_domain::prelude::*;
    pub use bevy_wings_ui_animation::prelude::*;
    pub use bevy_wings_ui_core::prelude::*;
    pub use bevy_wings_ui_modifiers::prelude::*;
    pub use bevy_wings_ui_mvvm::prelude::*;
    pub use bevy_wings_ui_navigation::prelude::*;
    pub use bevy_wings_ui_responsive::prelude::*;
    #[cfg(feature = "shaders")]
    pub use bevy_wings_ui_shaders::{
        WingsAuroraMaterial, WingsCircularProgressMaterial, WingsGlassMaterial,
        WingsRippleMaterial, WingsShimmerMaterial,
    };
    pub use bevy_wings_ui_theme::prelude::*;
    pub use bevy_wings_ui_widgets::prelude::*;
}

#[cfg(test)]
mod tests {
    use super::prelude::*;
    use bevy::prelude::*;

    /// The facade prelude exposes every crate without name clashes, and the plugin group
    /// contains the expected members in dependency order.
    #[test]
    fn plugin_group_order() {
        let group = WingsUiPlugins.build();
        let core = group.contains::<WingsCorePlugin>();
        assert!(core);
        assert!(group.contains::<WingsAnimationPlugin>());
        assert!(group.contains::<WingsThemePlugin>());
        assert!(group.contains::<WingsResponsivePlugin>());
        assert!(group.contains::<WingsModifierPlugin>());
        assert!(group.contains::<WingsWidgetsPlugin>());
        let _ = TypeRole::BodyLarge;
        let _ = Color::WHITE;
    }
}
