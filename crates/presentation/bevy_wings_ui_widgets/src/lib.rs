//! # bevy_wings_ui_widgets
//!
//! Themed, animated, accessible widgets for Bevy UI, written as `bsn!` scene components.
//!
//! | Group | Widgets |
//! | --- | --- |
//! | Text | [`WingsText`], [`WingsIcon`] |
//! | Layout | [`WingsColumn`], [`WingsRow`], [`WingsBox`], [`WingsSpacer`], [`WingsDivider`], [`WingsScrollColumn`], [`WingsGrid`], [`WingsSurface`], [`WingsCard`] |
//! | Buttons | [`WingsButton`], [`WingsIconButton`], [`WingsFab`] |
//! | Selection | [`WingsCheckbox`], [`WingsSwitch`], [`WingsRadioGroup`], [`WingsRadioButton`], [`WingsSlider`], [`WingsChip`] |
//! | Input | [`WingsTextField`] |
//! | Feedback | [`WingsLinearProgress`], `WingsCircularProgress`, [`WingsSkeleton`], [`WingsBadge`], [`WingsDialog`], snackbars ([`WingsShowSnackbar`]) |
//! | Structure | [`WingsScaffold`], [`WingsTopAppBar`], [`WingsNavigationBar`], [`WingsNavigationRail`], [`WingsNavigationItem`], [`WingsListItem`] |
//!
//! Every widget is an ordinary entity with ordinary components. Styling is expressed as theme
//! roles ([`WingsBackground`](bevy_wings_ui_theme::prelude::WingsBackground) and friends),
//! so the theme painter recolors all widgets when the theme changes, and interaction feedback
//! (hover, press, disabled, ripple) is driven by a handful of change detected systems instead of
//! per widget logic.
//!
//! ```ignore
//! use bevy::prelude::*;
//! use bevy_wings_ui_widgets::prelude::*;
//!
//! fn setup(mut commands: Commands) {
//!     commands.spawn(Camera2d);
//!     commands.spawn_scene(bsn! {
//!         @WingsScaffold
//!         Children [
//!             @WingsTopAppBar { @title: "Hello" }
//!             --
//!             @WingsColumn { @gap: 12.0, @padding: 16.0 }
//!             Children [
//!                 @WingsText { @text: "Welcome", @role: TypeRole::HeadlineMedium }
//!                 --
//!                 @WingsButton { @text: "Get started" }
//!                 on(|_: On<Activate>| info!("clicked"))
//!             ]
//!         ]
//!     });
//! }
//! ```

pub mod button;
pub mod feedback;
pub mod interaction;
pub mod layout;
#[cfg(feature = "shaders")]
pub mod ripple;
pub mod scaffold;
pub mod selection;
pub mod text;
pub mod text_field;

use bevy::input_focus::InputFocus;
use bevy::prelude::*;
use bevy::ui_widgets::{ButtonPlugin, UiWidgetsPlugins};

use bevy_wings_ui_animation::prelude::WingsAnimationAppExt;
use bevy_wings_ui_core::{WingsUiSystems, ensure_core};
use bevy_wings_ui_responsive::WingsResponsivePlugin;
use bevy_wings_ui_theme::WingsThemePlugin;

pub use button::{WingsButton, WingsButtonVariant, WingsFab, WingsFabSize, WingsIconButton};
#[cfg(feature = "shaders")]
pub use feedback::WingsCircularProgress;
pub use feedback::{
    WingsBadge, WingsDialog, WingsDialogResult, WingsLinearProgress, WingsProgress,
    WingsShowSnackbar, WingsSkeleton, WingsSnackbarAction, WingsSnackbarHost,
};
pub use interaction::WingsInteractive;
pub use layout::{
    WingsBox, WingsCard, WingsCardStyle, WingsColumn, WingsDivider, WingsGrid, WingsRow,
    WingsScrollColumn, WingsSpacer, WingsSurface,
};
pub use scaffold::{
    WingsListItem, WingsNavigationBar, WingsNavigationItem, WingsNavigationRail,
    WingsNavigationSelected, WingsScaffold, WingsTopAppBar,
};
pub use selection::{
    WingsCheckbox, WingsCheckedOwner, WingsCheckedPart, WingsChip, WingsRadioButton,
    WingsRadioGroup, WingsSlider, WingsSwitch,
};
pub use text::{WingsIcon, WingsText, wings_text};
pub use text_field::{WingsTextChanged, WingsTextField};

/// Registers widget systems, observers and the headless widget plugins.
///
/// Adds [`WingsThemePlugin`], [`WingsResponsivePlugin`] and `UiWidgetsPlugins` when they are not
/// present yet. With the `shaders` feature it also adds the shader materials unless
/// [`WingsWidgetsPlugin::headless`] is used.
pub struct WingsWidgetsPlugin {
    /// Use custom shader effects (ripple, shimmer, circular progress).
    pub shader_effects: bool,
}

impl Default for WingsWidgetsPlugin {
    fn default() -> Self {
        Self {
            shader_effects: true,
        }
    }
}

impl WingsWidgetsPlugin {
    /// Without shader materials: for tests, servers and apps without a renderer. Skeletons fall
    /// back to an opacity pulse.
    pub fn headless() -> Self {
        Self {
            shader_effects: false,
        }
    }
}

impl Plugin for WingsWidgetsPlugin {
    fn build(&self, app: &mut App) {
        ensure_core(app);
        if !app.is_plugin_added::<WingsThemePlugin>() {
            app.add_plugins(WingsThemePlugin::default());
        }
        if !app.is_plugin_added::<WingsResponsivePlugin>() {
            app.add_plugins(WingsResponsivePlugin::default());
        }
        if !app.is_plugin_added::<ButtonPlugin>() {
            app.add_plugins(UiWidgetsPlugins);
        }
        #[cfg(feature = "shaders")]
        if self.shader_effects
            && !app.is_plugin_added::<bevy_wings_ui_shaders::WingsShadersPlugin>()
        {
            app.add_plugins(bevy_wings_ui_shaders::WingsShadersPlugin);
        }
        app.init_resource::<InputFocus>()
            .add_message::<WingsShowSnackbar>()
            .register_wings_property::<feedback::SweepProperty>()
            .register_wings_property::<feedback::BarWidthProperty>()
            .register_wings_property::<selection::CheckMarkProperty>()
            .add_systems(
                Update,
                (
                    interaction::interaction_style_system,
                    button::button_variant_system,
                    button::fab_state_layer_system,
                    selection::checked_parts_system,
                    selection::slider_layout_system,
                    layout::box_layout_system,
                    feedback::linear_progress_system,
                    feedback::skeleton_system,
                    feedback::show_snackbars,
                    feedback::tick_snackbars,
                    feedback::dialog_open_system,
                    text_field::text_field_focus_system,
                )
                    .in_set(WingsUiSystems::Style),
            )
            .add_systems(
                Update,
                text::icon_tint_system.in_set(WingsUiSystems::Animate),
            )
            .add_systems(
                Update,
                selection::draw_checkmarks.after(WingsUiSystems::Animate),
            );
        #[cfg(feature = "shaders")]
        {
            app.add_systems(
                Update,
                (
                    ripple::add_default_ripples,
                    ripple::tick_ripples,
                    feedback::circular_progress_system,
                )
                    .in_set(WingsUiSystems::Style),
            )
            .add_observer(ripple::spawn_ripple);
        }
    }
}

/// Commonly used items, including the Bevy widget events.
pub mod prelude {
    pub use crate::WingsWidgetsPlugin;
    pub use crate::button::{
        WingsButton, WingsButtonVariant, WingsFab, WingsFabSize, WingsIconButton,
    };
    #[cfg(feature = "shaders")]
    pub use crate::feedback::WingsCircularProgress;
    pub use crate::feedback::{
        WingsBadge, WingsDialog, WingsDialogResult, WingsLinearProgress, WingsProgress,
        WingsShowSnackbar, WingsSkeleton, WingsSnackbarAction, WingsSnackbarHost,
    };
    pub use crate::interaction::WingsInteractive;
    pub use crate::layout::{
        WingsBox, WingsCard, WingsCardStyle, WingsColumn, WingsDivider, WingsGrid, WingsRow,
        WingsScrollColumn, WingsSpacer, WingsSurface,
    };
    #[cfg(feature = "shaders")]
    pub use crate::ripple::{WingsNoRipple, WingsRipple};
    pub use crate::scaffold::{
        WingsListItem, WingsNavigationBar, WingsNavigationItem, WingsNavigationRail,
        WingsNavigationSelected, WingsScaffold, WingsTopAppBar,
    };
    pub use crate::selection::{
        WingsCheckbox, WingsCheckedOwner, WingsCheckedPart, WingsChip, WingsRadioButton,
        WingsRadioGroup, WingsSlider, WingsSwitch,
    };
    pub use crate::text::{WingsIcon, WingsText, wings_text};
    pub use crate::text_field::{WingsTextChanged, WingsTextField};
    pub use bevy::ui::{Checked, InteractionDisabled};
    pub use bevy::ui_widgets::{Activate, ValueChange};
}

#[cfg(test)]
mod tests;
