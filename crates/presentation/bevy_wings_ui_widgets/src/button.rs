//! Buttons: [`WingsButton`], [`WingsIconButton`] and [`WingsFab`].
//!
//! Buttons are built on the headless `bevy_ui_widgets::Button`, so they emit
//! [`Activate`](bevy::ui_widgets::Activate) on click, Enter and Space, and support keyboard
//! focus through [`TabIndex`]. Attach behavior directly in the scene:
//!
//! ```ignore
//! bsn! {
//!     @WingsButton { @text: "Save", @variant: WingsButtonVariant::Filled }
//!     on(|_: On<Activate>, mut vm: Query<&mut EditorViewModel>| { /* ... */ })
//! }
//! ```
//!
//! Colors are never hard coded: [`WingsButtonVariant`] maps to theme roles, so every button
//! follows light, dark and seeded schemes, and a theme switch repaints them all at once.

use bevy::input_focus::tab_navigation::TabIndex;
use bevy::picking::cursor::EntityCursor;
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::Button;
use bevy::window::SystemCursorIcon;

use bevy_wings_ui_theme::prelude::{
    ColorRole, ShapeRole, TypeRole, WingsBackground, WingsBorderPaint, WingsContentColor,
    WingsElevationLevel, WingsPaint, WingsShape, WingsStateLayer,
};

use crate::interaction::WingsInteractive;
use crate::text::{WingsIcon, WingsText};

/// Visual emphasis of a button. Also the marker the styling system uses to find buttons.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum WingsButtonVariant {
    /// Highest emphasis: primary fill.
    #[default]
    Filled,
    /// Medium emphasis: secondary container fill.
    Tonal,
    /// Medium emphasis: outline only.
    Outlined,
    /// Low emphasis: text only.
    Text,
    /// Raised surface with a shadow.
    Elevated,
    /// Destructive action: error fill.
    Danger,
}

/// Theme roles used to paint a button variant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WingsButtonPalette {
    /// Fill.
    pub container: WingsPaint,
    /// Label and icon color.
    pub content: ColorRole,
    /// Outline.
    pub border: WingsPaint,
    /// Elevation level.
    pub elevation: u8,
}

impl WingsButtonVariant {
    /// Returns the theme roles of this variant.
    pub fn palette(self) -> WingsButtonPalette {
        use ColorRole::*;
        let (container, content, border, elevation) = match self {
            Self::Filled => (
                WingsPaint::Role(Primary),
                OnPrimary,
                WingsPaint::Transparent,
                0,
            ),
            Self::Tonal => (
                WingsPaint::Role(SecondaryContainer),
                OnSecondaryContainer,
                WingsPaint::Transparent,
                0,
            ),
            Self::Outlined => (
                WingsPaint::Transparent,
                Primary,
                WingsPaint::Role(Outline),
                0,
            ),
            Self::Text => (WingsPaint::Transparent, Primary, WingsPaint::Transparent, 0),
            Self::Elevated => (
                WingsPaint::Role(SurfaceContainerLow),
                Primary,
                WingsPaint::Transparent,
                1,
            ),
            Self::Danger => (WingsPaint::Role(Error), OnError, WingsPaint::Transparent, 0),
        };
        WingsButtonPalette {
            container,
            content,
            border,
            elevation,
        }
    }

    /// Picks [`Filled`](Self::Filled) when `selected`, [`Outlined`](Self::Outlined) otherwise.
    /// Handy for segmented or toggle style buttons.
    pub fn selected(selected: bool) -> Self {
        if selected {
            Self::Filled
        } else {
            Self::Outlined
        }
    }
}

/// Props of [`WingsButton`].
pub struct WingsButtonProps {
    /// Label text. Empty for no label.
    pub text: String,
    /// Optional leading icon asset path. Empty for no icon.
    pub icon: String,
    /// Visual emphasis.
    pub variant: WingsButtonVariant,
    /// When `false` the button starts disabled ([`InteractionDisabled`]).
    pub enabled: bool,
    /// Expand to the width of the parent.
    pub fill_width: bool,
}

impl Default for WingsButtonProps {
    fn default() -> Self {
        Self {
            text: String::new(),
            icon: String::new(),
            variant: WingsButtonVariant::Filled,
            enabled: true,
            fill_width: false,
        }
    }
}

/// A themed, animated push button.
///
/// ```ignore
/// bsn! { @WingsButton { @text: "Continue", @variant: WingsButtonVariant::Tonal } }
/// ```
#[derive(SceneComponent, Default, Clone)]
#[scene(WingsButtonProps)]
pub struct WingsButton;

/// Scene for a conditional disabled marker.
pub(crate) fn disabled_scene(enabled: bool) -> Box<dyn Scene> {
    if enabled {
        Box::new(bsn! {})
    } else {
        Box::new(bsn! { InteractionDisabled })
    }
}

/// Common interactive part of every clickable widget.
pub(crate) fn clickable(enabled: bool) -> impl Scene {
    let disabled = disabled_scene(enabled);
    bsn! {
        Button
        WingsInteractive
        TabIndex(0)
        EntityCursor::System(SystemCursorIcon::Pointer)
        @{disabled}
    }
}

impl WingsButton {
    fn scene(props: WingsButtonProps) -> impl Scene {
        let icon: Box<dyn Scene> = if props.icon.is_empty() {
            Box::new(bsn! {})
        } else {
            let image = props.icon.clone();
            Box::new(bsn! { Children [ @WingsIcon { @image: image, @size: 18.0 } ] })
        };
        let label: Box<dyn Scene> = if props.text.is_empty() {
            Box::new(bsn! {})
        } else {
            let text = props.text.clone();
            Box::new(bsn! { Children [ @WingsText { @text: text, @role: TypeRole::LabelLarge } ] })
        };
        let node = Node {
            min_height: px(40),
            padding: UiRect::horizontal(px(if props.variant == WingsButtonVariant::Text {
                12
            } else {
                24
            })),
            column_gap: px(8),
            border: UiRect::all(px(1)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            width: if props.fill_width {
                Val::Percent(100.0)
            } else {
                Val::Auto
            },
            ..default()
        };
        let variant = props.variant;
        let interactive = clickable(props.enabled);
        bsn! {
            node
            variant
            WingsShape(ShapeRole::Full)
            WingsBackground
            WingsContentColor
            WingsBorderPaint
            WingsElevationLevel
            @{interactive}
            @{icon}
            @{label}
        }
    }
}

/// Icon only button (toolbar actions, close buttons).
#[derive(SceneComponent, Default, Clone)]
#[scene(WingsIconButtonProps)]
pub struct WingsIconButton;

/// Props of [`WingsIconButton`].
pub struct WingsIconButtonProps {
    /// Icon asset path.
    pub icon: String,
    /// Visual emphasis, [`WingsButtonVariant::Text`] (no fill) by default.
    pub variant: WingsButtonVariant,
    /// When `false` the button starts disabled.
    pub enabled: bool,
}

impl Default for WingsIconButtonProps {
    fn default() -> Self {
        Self {
            icon: String::new(),
            variant: WingsButtonVariant::Text,
            enabled: true,
        }
    }
}

impl WingsIconButton {
    fn scene(props: WingsIconButtonProps) -> impl Scene {
        let variant = props.variant;
        let image = props.icon;
        let interactive = clickable(props.enabled);
        bsn! {
            Node {
                width: px(40),
                height: px(40),
                border: px(1),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
            }
            variant
            WingsShape(ShapeRole::Full)
            WingsBackground
            WingsContentColor
            WingsBorderPaint
            WingsElevationLevel
            @{interactive}
            Children [ @WingsIcon { @image: image, @size: 24.0 } ]
        }
    }
}

/// Floating action button size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WingsFabSize {
    /// 40 px.
    Small,
    /// 56 px.
    #[default]
    Regular,
    /// 96 px.
    Large,
}

impl WingsFabSize {
    /// Edge length in pixels.
    pub fn px(self) -> f32 {
        match self {
            Self::Small => 40.0,
            Self::Regular => 56.0,
            Self::Large => 96.0,
        }
    }
}

/// Marker for floating action buttons (painted with the primary container role).
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct WingsFabMarker;

/// Floating action button: the primary action of a screen.
#[derive(SceneComponent, Default, Clone)]
#[scene(WingsFabProps)]
pub struct WingsFab;

/// Props of [`WingsFab`].
#[derive(Default)]
pub struct WingsFabProps {
    /// Icon asset path.
    pub icon: String,
    /// Optional label. A non empty label makes it an extended FAB.
    pub text: String,
    /// Size of the button.
    pub size: WingsFabSize,
}

impl WingsFab {
    fn scene(props: WingsFabProps) -> impl Scene {
        let edge = props.size.px();
        let extended = !props.text.is_empty();
        let node = Node {
            min_width: px(edge),
            height: px(edge),
            padding: UiRect::horizontal(px(if extended { 16.0 } else { 0.0 })),
            column_gap: px(12),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        };
        let icon: Box<dyn Scene> = if props.icon.is_empty() {
            Box::new(bsn! {})
        } else {
            let image = props.icon;
            let size = if props.size == WingsFabSize::Large {
                36.0_f32
            } else {
                24.0
            };
            Box::new(bsn! { Children [ @WingsIcon { @image: image, @size: size } ] })
        };
        let label: Box<dyn Scene> = if extended {
            let text = props.text;
            Box::new(bsn! { Children [ @WingsText { @text: text, @role: TypeRole::LabelLarge } ] })
        } else {
            Box::new(bsn! {})
        };
        let interactive = clickable(true);
        bsn! {
            node
            WingsFabMarker
            WingsShape(ShapeRole::Large)
            WingsBackground({WingsPaint::Role(ColorRole::PrimaryContainer)})
            WingsContentColor({WingsPaint::Role(ColorRole::OnPrimaryContainer)})
            WingsElevationLevel(3)
            @{interactive}
            @{icon}
            @{label}
        }
    }
}

/// Applies the palette of changed [`WingsButtonVariant`]s.
pub fn button_variant_system(
    mut buttons: Query<
        (
            &WingsButtonVariant,
            &mut WingsBackground,
            &mut WingsContentColor,
            &mut WingsBorderPaint,
            &mut WingsElevationLevel,
            &mut WingsStateLayer,
        ),
        Changed<WingsButtonVariant>,
    >,
) {
    for (variant, mut bg, mut content, mut border, mut elevation, mut layer) in &mut buttons {
        let p = variant.palette();
        bg.set_if_neq(WingsBackground(p.container));
        content.set_if_neq(WingsContentColor(WingsPaint::Role(p.content)));
        border.set_if_neq(WingsBorderPaint(p.border));
        elevation.set_if_neq(WingsElevationLevel(p.elevation));
        let layer_color = WingsPaint::Role(p.content);
        if layer.color != layer_color {
            layer.color = layer_color;
        }
    }
}

/// Uses the content color for the FAB state layer.
pub fn fab_state_layer_system(
    mut fabs: Query<&mut WingsStateLayer, (With<WingsFabMarker>, Added<WingsFabMarker>)>,
) {
    for mut layer in &mut fabs {
        layer.color = WingsPaint::Role(ColorRole::OnPrimaryContainer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Filled and danger buttons pair a fill with its readable `on` role.
    #[test]
    fn palettes_pair_fill_and_content() {
        let p = WingsButtonVariant::Filled.palette();
        assert_eq!(p.container, WingsPaint::Role(ColorRole::Primary));
        assert_eq!(p.content, ColorRole::OnPrimary);
        let d = WingsButtonVariant::Danger.palette();
        assert_eq!(d.container, WingsPaint::Role(ColorRole::Error));
        assert_eq!(d.content, ColorRole::OnError);
    }

    /// Only outlined buttons draw a border, only elevated buttons cast a shadow.
    #[test]
    fn outline_and_elevation() {
        for v in [
            WingsButtonVariant::Filled,
            WingsButtonVariant::Tonal,
            WingsButtonVariant::Outlined,
            WingsButtonVariant::Text,
            WingsButtonVariant::Elevated,
            WingsButtonVariant::Danger,
        ] {
            let p = v.palette();
            assert_eq!(
                p.border != WingsPaint::Transparent,
                v == WingsButtonVariant::Outlined
            );
            assert_eq!(p.elevation > 0, v == WingsButtonVariant::Elevated);
        }
    }

    /// `selected` maps toggles to filled and outlined.
    #[test]
    fn selected_helper() {
        assert_eq!(
            WingsButtonVariant::selected(true),
            WingsButtonVariant::Filled
        );
        assert_eq!(
            WingsButtonVariant::selected(false),
            WingsButtonVariant::Outlined
        );
    }
}
