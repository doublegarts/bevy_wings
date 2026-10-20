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
//! | Structure | [`WingsScaffold`], [`WingsTopAppBar`], [`WingsNavigationBar`], [`WingsNavigationRail`], [`WingsNavigationItem`], [`WingsListItem`], [`WingsList`], [`WingsListSection`] |
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

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/doublegarts/bevy_wings/main/assets/branding/bevy-wings-icon.png",
    html_favicon_url = "https://raw.githubusercontent.com/doublegarts/bevy_wings/main/assets/branding/bevy-wings-icon.png"
)]

pub mod button;
pub mod feedback;
pub mod feedback_skin;
pub mod focus;
pub mod gamepad;
pub mod glass;
pub mod interaction;
pub mod layout;
pub mod list;
pub mod overlay;
pub mod picker;
pub mod popup;
#[cfg(feature = "shaders")]
pub mod ripple;
pub mod scaffold;
pub mod scaffold_skin;
pub mod segmented;
pub mod selection;
pub mod selection_skin;
pub mod sheet;
pub mod skin;
pub mod text;
pub mod text_field;
pub mod touch;
pub mod virtual_list;

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
pub use list::{WingsList, WingsListSection, WingsListStyle};
pub use scaffold::{
    WingsListAccessory, WingsListItem, WingsNavigationBar, WingsNavigationItem,
    WingsNavigationRail, WingsNavigationSelected, WingsScaffold, WingsTitleDisplay, WingsTopAppBar,
};
pub use selection::{
    WingsCheckbox, WingsCheckedOwner, WingsCheckedPart, WingsChip, WingsRadioButton,
    WingsRadioGroup, WingsSlider, WingsSwitch,
};
pub use skin::{WingsPressFeedback, WingsSkin, WingsSkinPart};
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
            .init_resource::<skin::WingsSkin>()
            .init_resource::<glass::WingsGlassSettings>()
            .init_resource::<overlay::WingsFloatingChrome>()
            .init_resource::<gamepad::WingsGamepadNavigation>()
            .init_resource::<touch::WingsTouchScroll>()
            .init_resource::<touch::WingsTouchPresses>()
            .add_observer(touch::record_touch_press)
            .init_resource::<bevy::input_focus::InputFocus>()
            .init_resource::<bevy::input_focus::directional_navigation::DirectionalNavigationMap>()
            .init_resource::<bevy::input_focus::directional_navigation::AutoNavigationConfig>()
            .add_message::<WingsShowSnackbar>()
            .register_wings_property::<feedback::SweepProperty>()
            .register_wings_property::<feedback::BarWidthProperty>()
            .register_wings_property::<selection::CheckMarkProperty>()
            .register_wings_property::<feedback_skin::SpinnerTurnProperty>()
            .add_systems(
                Update,
                (
                    skin::sync_skin_design,
                    skin::skin_press_feedback,
                    feedback_skin::skin_dialogs,
                    scaffold_skin::skin_bar_buttons,
                    (
                        button::button_variant_system,
                        button::fab_state_layer_system,
                        feedback_skin::skin_linear_progress,
                        feedback::skeleton_system,
                        selection_skin::skin_checkboxes,
                        selection_skin::skin_switches,
                        selection_skin::skin_radio_buttons,
                        selection_skin::skin_sliders,
                        selection_skin::skin_chips,
                        text_field::skin_text_fields,
                        layout::skin_cards,
                    ),
                    (
                        scaffold_skin::skin_scaffolds,
                        scaffold_skin::skin_top_app_bars,
                        scaffold_skin::skin_navigation_bars,
                        scaffold_skin::skin_navigation_rails,
                        scaffold_skin::skin_navigation_items,
                        scaffold_skin::skin_list_items,
                        list::skin_lists,
                        segmented::skin_segmented,
                        popup::skin_dropdowns,
                        sheet::skin_sheets,
                    ),
                    (
                        scaffold_skin::compact_sidebars,
                        scaffold_skin::collapse_large_titles,
                        scaffold_skin::layout_top_app_bars,
                        scaffold_skin::fit_inline_titles,
                        scaffold_skin::title_scroll_effect,
                        scaffold_skin::window_chrome_system,
                        scaffold_skin::scaffold_safe_area_system,
                        scaffold_skin::scaffold_background_system,
                        overlay::ensure_overlay_camera,
                        overlay::ensure_overlay_roots,
                        overlay::float_chrome,
                        overlay::chrome_insets_system,
                        overlay::scroll_under_chrome,
                        list::layout_list_sections,
                    )
                        .chain(),
                    skin::refold_modifiers_after_skin,
                )
                    .chain()
                    .in_set(WingsUiSystems::Skin),
            )
            .add_systems(
                Update,
                (
                    (
                        interaction::interaction_style_system,
                        selection::checked_parts_system,
                        selection::slider_layout_system,
                        layout::box_layout_system,
                        layout::scroll_children_keep_size,
                        feedback::linear_progress_system,
                        feedback::show_snackbars,
                        feedback::tick_snackbars,
                        feedback::dialog_open_system,
                        text_field::text_field_focus_system,
                        text_field::text_field_placeholder_system,
                        glass::glass_system,
                        glass::glass_removed,
                        glass::scroll_edge_system,
                        glass::scroll_edge_removed,
                    ),
                    (
                        segmented::segment_selection_system,
                        segmented::segment_elevation_system,
                        popup::popup_system,
                        popup::position_popups,
                        popup::dropdown_value_system,
                        popup::tooltip_system,
                        popup::position_tooltips,
                        sheet::sheet_scrims,
                        picker::date_picker_system,
                        picker::time_picker_keys,
                        picker::time_picker_system,
                        focus::focus_ring_system,
                    ),
                    (
                        virtual_list::virtual_list_system,
                        gamepad::enable_directional_navigation,
                    ),
                )
                    .in_set(WingsUiSystems::Style),
            )
            .add_systems(
                Update,
                (
                    gamepad::gamepad_navigation_system,
                    (
                        touch::complete_instant_taps,
                        touch::touch_scroll_system,
                        touch::scroll_momentum_system,
                    )
                        .chain(),
                )
                    .in_set(WingsUiSystems::Input),
            )
            // Back closes the topmost layer first: menus, then dialogs, then sheets.
            .add_systems(
                Update,
                (
                    popup::close_popups_on_back,
                    feedback::dismiss_dialogs_on_back,
                    sheet::dismiss_sheets_on_back,
                )
                    .chain()
                    .in_set(bevy_wings_ui_core::back::WingsBackSystems::Overlays),
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
                feedback_skin::skin_circular_progress.in_set(WingsUiSystems::Skin),
            )
            .add_systems(
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
    pub use crate::focus::{WingsFocusRing, WingsNoFocusRing};
    pub use crate::gamepad::WingsGamepadNavigation;
    pub use crate::glass::{WingsGlass, WingsGlassSettings, WingsGlassStyle};
    pub use crate::interaction::WingsInteractive;
    pub use crate::layout::{
        WingsBox, WingsCard, WingsCardStyle, WingsColumn, WingsDivider, WingsGrid, WingsRow,
        WingsScrollColumn, WingsSpacer, WingsSurface,
    };
    pub use crate::list::{WingsList, WingsListSection, WingsListStyle};
    pub use crate::overlay::{WingsChromeInsets, WingsFloatingChrome, WingsOverlayCamera};
    pub use crate::picker::{
        WingsDate, WingsDateChanged, WingsDatePicker, WingsTimeChanged, WingsTimePicker,
    };
    pub use crate::popup::{
        WingsDropdown, WingsMenuButton, WingsMenuSelected, WingsPopupOwner, WingsTooltip,
    };
    #[cfg(feature = "shaders")]
    pub use crate::ripple::{WingsNoRipple, WingsRipple};
    pub use crate::scaffold::{
        WingsListAccessory, WingsListItem, WingsNavigationBar, WingsNavigationItem,
        WingsNavigationRail, WingsNavigationSelected, WingsScaffold, WingsTitleDisplay,
        WingsTopAppBar,
    };
    pub use crate::segmented::{
        WingsSegmentChanged, WingsSegmented, WingsSelectedIndex, WingsTabs,
    };
    pub use crate::selection::{
        WingsCheckbox, WingsCheckedOwner, WingsCheckedPart, WingsChip, WingsRadioButton,
        WingsRadioGroup, WingsSlider, WingsSwitch,
    };
    pub use crate::sheet::{
        WingsBottomSheet, WingsSheetDetent, WingsSheetDismissed, WingsSideSheet,
    };
    pub use crate::skin::{WingsPressFeedback, WingsSkin, WingsSkinPart};
    pub use crate::text::{WingsIcon, WingsText, wings_text};
    pub use crate::text_field::{WingsTextChanged, WingsTextField};
    pub use crate::touch::{WingsScrollMomentum, WingsTouchScroll};
    pub use crate::virtual_list::{WingsVirtualGrid, WingsVirtualList, WingsVirtualRows};
    pub use bevy::ui::{Checked, InteractionDisabled};
    pub use bevy::ui_widgets::{Activate, ValueChange};
}

#[cfg(test)]
mod tests;
