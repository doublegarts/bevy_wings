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
//!
//! The variants map to the platform's own button styles in the Apple designs:
//!
//! | Variant | Material | iOS | macOS | Fluent |
//! | --- | --- | --- | --- | --- |
//! | Filled | filled | `.borderedProminent` | accent push button | accent button |
//! | Tonal | tonal | tinted `.bordered` | push button | accent tinted button |
//! | Outlined | outlined | `.bordered` | push button with a hairline | standard button |
//! | Text | text | `.borderless` | borderless | hyperlink button |
//! | Elevated | elevated | `.glass` | glass | standard button with a shadow |
//! | Danger | error fill | destructive prominent | destructive | critical fill |

use bevy::input_focus::tab_navigation::TabIndex;
use bevy::picking::cursor::EntityCursor;
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::Button;
use bevy::window::SystemCursorIcon;

use bevy_wings_ui_theme::prelude::{
    ColorRole, ShapeRole, TypeRole, WingsBackground, WingsBorderPaint, WingsContentColor,
    WingsDesign, WingsElevationLevel, WingsPaint, WingsShape, WingsStateLayer,
};

use crate::interaction::WingsInteractive;
use crate::skin::{WingsSkin, skin_targets};
use crate::text::{WingsIcon, WingsIconTint, WingsText};

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
    /// Returns the theme roles of this variant in a design.
    pub fn palette_for(self, design: WingsDesign) -> WingsButtonPalette {
        use ColorRole::*;
        let p = |container, content, border, elevation| WingsButtonPalette {
            container,
            content,
            border,
            elevation,
        };
        let t = WingsPaint::Transparent;
        match (design, self) {
            (WingsDesign::Material, _) => self.palette(),
            (WingsDesign::Fluent, Self::Tonal) => {
                p(WingsPaint::Role(PrimaryContainer), OnPrimaryContainer, t, 0)
            }
            (WingsDesign::Fluent, Self::Outlined) => p(
                WingsPaint::Role(SurfaceContainerLowest),
                OnSurface,
                WingsPaint::Role(OutlineVariant),
                0,
            ),
            (WingsDesign::Fluent, Self::Elevated) => p(
                WingsPaint::Role(SurfaceContainerLowest),
                OnSurface,
                WingsPaint::Role(OutlineVariant),
                1,
            ),
            (_, Self::Filled) => p(WingsPaint::Role(Primary), OnPrimary, t, 0),
            (_, Self::Danger) => p(WingsPaint::Role(Error), OnError, t, 0),
            (_, Self::Text) => p(t, Primary, t, 0),
            (_, Self::Elevated) => p(
                WingsPaint::Role(SurfaceContainerLowest),
                OnSurface,
                WingsPaint::Role(OutlineVariant),
                2,
            ),
            (WingsDesign::Ios, Self::Tonal) => {
                p(WingsPaint::Role(PrimaryContainer), OnPrimaryContainer, t, 0)
            }
            (WingsDesign::Ios, Self::Outlined) => p(WingsPaint::Role(SecondaryFill), Primary, t, 0),
            (_, Self::Tonal) => p(WingsPaint::Role(SecondaryFill), OnSurface, t, 0),
            (_, Self::Outlined) => p(
                WingsPaint::Role(SurfaceContainerLowest),
                OnSurface,
                WingsPaint::Role(Outline),
                1,
            ),
        }
    }

    /// Returns the theme roles of this variant in the Material design.
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

/// Edge length of a floating action button in the Material design.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct WingsFabEdge(pub f32);

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
        let fab_edge = WingsFabEdge(edge);
        bsn! {
            node
            WingsFabMarker
            fab_edge
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

/// Adjusts what the active skin gives a button. Dialogs use it for their platform style
/// action buttons.
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct WingsButtonOverride {
    /// Palette instead of the variant's.
    pub palette: Option<WingsButtonPalette>,
    /// Minimum height instead of the design's.
    pub min_height: Option<f32>,
    /// Share the available width with sibling buttons.
    pub grow: bool,
}

/// Sizes of a push button in a design.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WingsButtonMetrics {
    /// Minimum height.
    pub height: f32,
    /// Horizontal padding.
    pub padding: f32,
    /// Horizontal padding of text only buttons.
    pub text_padding: f32,
    /// Corner shape.
    pub shape: ShapeRole,
    /// Gap between icon and label.
    pub gap: f32,
    /// Edge length of an icon button.
    pub icon_button: f32,
    /// Icon size inside an icon button.
    pub icon: f32,
}

impl WingsButtonMetrics {
    /// Metrics of a design (iOS and macOS values measured on SwiftUI 27).
    pub fn for_design(design: WingsDesign) -> Self {
        match design {
            WingsDesign::Material => Self {
                height: 40.0,
                padding: 24.0,
                text_padding: 12.0,
                shape: ShapeRole::Full,
                gap: 8.0,
                icon_button: 40.0,
                icon: 24.0,
            },
            WingsDesign::Ios => Self {
                height: 34.0,
                padding: 14.0,
                text_padding: 4.0,
                shape: ShapeRole::Full,
                gap: 6.0,
                icon_button: 44.0,
                icon: 22.0,
            },
            WingsDesign::MacOs => Self {
                height: 24.0,
                padding: 12.0,
                text_padding: 4.0,
                shape: ShapeRole::Small,
                gap: 4.0,
                icon_button: 28.0,
                icon: 16.0,
            },
            WingsDesign::Fluent => Self {
                height: 32.0,
                padding: 12.0,
                text_padding: 8.0,
                shape: ShapeRole::Small,
                gap: 8.0,
                icon_button: 32.0,
                icon: 16.0,
            },
        }
    }
}

type ButtonParts<'a> = (
    &'a WingsButtonVariant,
    Option<&'a WingsButtonOverride>,
    Has<WingsIconButton>,
    Has<WingsButton>,
    &'a mut Node,
    &'a mut WingsShape,
    &'a mut WingsBackground,
    &'a mut WingsContentColor,
    &'a mut WingsBorderPaint,
    &'a mut WingsElevationLevel,
    &'a mut WingsStateLayer,
);

/// Skins buttons and icon buttons: applies the palette of their variant in the active design
/// and the design's sizes. Runs for new buttons, changed variants and design changes.
pub fn button_variant_system(
    mut commands: Commands,
    skin: Res<WingsSkin>,
    added: Query<
        Entity,
        Or<(
            Added<WingsButtonVariant>,
            Changed<WingsButtonVariant>,
            Changed<WingsButtonOverride>,
        )>,
    >,
    all: Query<Entity, With<WingsButtonVariant>>,
    mut buttons: Query<ButtonParts>,
    children: Query<&Children>,
    mut icons: Query<&mut Node, (With<WingsIconTint>, Without<WingsButtonVariant>)>,
) {
    let metrics = WingsButtonMetrics::for_design(skin.design);
    for entity in skin_targets(&skin, added, all) {
        let Ok((
            variant,
            palette_override,
            icon_button,
            push_button,
            mut node,
            mut shape,
            mut bg,
            mut content,
            mut border,
            mut elevation,
            mut layer,
        )) = buttons.get_mut(entity)
        else {
            continue;
        };
        let adjust = palette_override.copied().unwrap_or_default();
        let p = adjust
            .palette
            .unwrap_or_else(|| variant.palette_for(skin.design));
        bg.set_if_neq(WingsBackground(p.container));
        content.set_if_neq(WingsContentColor(WingsPaint::Role(p.content)));
        border.set_if_neq(WingsBorderPaint(p.border));
        elevation.set_if_neq(WingsElevationLevel(p.elevation));
        let layer_color = if skin.design == WingsDesign::MacOs {
            WingsPaint::Role(ColorRole::Shadow)
        } else {
            WingsPaint::Role(p.content)
        };
        if layer.color != layer_color {
            layer.color = layer_color;
        }
        // Apple glass buttons and their translucent fallback.
        if skin.design.is_apple()
            && *variant == WingsButtonVariant::Elevated
            && adjust.palette.is_none()
        {
            commands.entity(entity).insert(
                crate::glass::WingsGlass::new(crate::glass::WingsGlassStyle::Regular).interactive(),
            );
        } else {
            commands.entity(entity).remove::<crate::glass::WingsGlass>();
        }
        if push_button {
            let pad = if *variant == WingsButtonVariant::Text {
                metrics.text_padding
            } else {
                metrics.padding
            };
            node.min_height = px(adjust.min_height.unwrap_or(metrics.height));
            node.padding = UiRect::horizontal(px(pad));
            node.column_gap = px(metrics.gap);
            node.flex_grow = if adjust.grow { 1.0 } else { 0.0 };
            node.flex_basis = if adjust.grow { px(0) } else { Val::Auto };
            shape.set_if_neq(WingsShape(metrics.shape));
        } else if icon_button {
            let edge = px(metrics.icon_button);
            node.width = edge;
            node.height = edge;
            shape.set_if_neq(WingsShape(if skin.design.is_desktop() {
                ShapeRole::Small
            } else {
                ShapeRole::Full
            }));
            for child in children.iter_descendants(entity) {
                if let Ok(mut icon) = icons.get_mut(child) {
                    icon.width = px(metrics.icon);
                    icon.height = px(metrics.icon);
                }
            }
        }
    }
}

/// Skins floating action buttons: a Material FAB, or a tinted glass circle in the Apple
/// designs.
pub fn fab_state_layer_system(
    skin: Res<WingsSkin>,
    added: Query<Entity, Added<WingsFabMarker>>,
    all: Query<Entity, With<WingsFabMarker>>,
    mut fabs: Query<(
        &WingsFabEdge,
        &mut Node,
        &mut WingsShape,
        &mut WingsBackground,
        &mut WingsContentColor,
        &mut WingsElevationLevel,
        &mut WingsStateLayer,
    )>,
) {
    for entity in skin_targets(&skin, added, all) {
        let Ok((edge, mut node, mut shape, mut bg, mut content, mut elevation, mut layer)) =
            fabs.get_mut(entity)
        else {
            continue;
        };
        let (container, on, shape_role) = if skin.apple() {
            (ColorRole::Primary, ColorRole::OnPrimary, ShapeRole::Full)
        } else if skin.fluent() {
            (ColorRole::Primary, ColorRole::OnPrimary, ShapeRole::Large)
        } else {
            (
                ColorRole::PrimaryContainer,
                ColorRole::OnPrimaryContainer,
                ShapeRole::Large,
            )
        };
        let size = edge.0 * skin.pick(1.0, 1.0, 0.8, 0.86);
        node.min_width = px(size);
        node.height = px(size);
        shape.set_if_neq(WingsShape(shape_role));
        bg.set_if_neq(WingsBackground(WingsPaint::Role(container)));
        content.set_if_neq(WingsContentColor(WingsPaint::Role(on)));
        elevation.set_if_neq(WingsElevationLevel(if skin.apple() { 4 } else { 3 }));
        layer.color = WingsPaint::Role(on);
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

    /// Apple palettes map the variants to the platform button styles.
    #[test]
    fn apple_palettes() {
        let ios = |v: WingsButtonVariant| v.palette_for(WingsDesign::Ios);
        assert_eq!(
            ios(WingsButtonVariant::Filled).container,
            WingsPaint::Role(ColorRole::Primary)
        );
        assert_eq!(
            ios(WingsButtonVariant::Outlined).container,
            WingsPaint::Role(ColorRole::SecondaryFill)
        );
        assert_eq!(ios(WingsButtonVariant::Text).content, ColorRole::Primary);
        let mac = WingsButtonVariant::Tonal.palette_for(WingsDesign::MacOs);
        assert_eq!(mac.content, ColorRole::OnSurface);
        assert_eq!(
            WingsButtonVariant::Filled.palette_for(WingsDesign::Material),
            WingsButtonVariant::Filled.palette()
        );
        assert_eq!(
            WingsButtonMetrics::for_design(WingsDesign::Ios).height,
            34.0
        );
        assert_eq!(
            WingsButtonMetrics::for_design(WingsDesign::MacOs).height,
            24.0
        );
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
