//! Popups: [`WingsMenuButton`], [`WingsDropdown`] and [`WingsTooltip`].
//!
//! A popup is a floating panel anchored to the widget that opened it. It lives on the overlay
//! layer when there is one (so it renders as Liquid Glass in the Apple designs) and closes on
//! back (Escape, gamepad B), on a click outside it, or when an item is chosen.
//!
//! | Design | Menu | Dropdown | Tooltip |
//! | --- | --- | --- | --- |
//! | Material | surface menu, 48 tall items | outlined field with a caret | dark plain tooltip |
//! | iOS | glass menu, 44 tall items | tinted value with up and down chevrons (`Picker(.menu)`) | glass bubble |
//! | macOS | glass menu, 24 tall items | pop up button | help tag |
//! | Fluent | menu flyout, 32 tall items | combo box | tooltip with a border |
//!
//! ```ignore
//! bsn! {
//!     @WingsDropdown { @options: vec!["Small".into(), "Medium".into(), "Large".into()] }
//!     on(|ev: On<WingsMenuSelected>| info!("picked {}", ev.label))
//! }
//! ```

use bevy::input_focus::tab_navigation::TabIndex;
use bevy::picking::cursor::EntityCursor;
use bevy::picking::events::PointerPress;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::UiGlobalTransform;
use bevy::ui_widgets::{Activate, Button};
use bevy::window::SystemCursorIcon;

use bevy_wings_ui_core::back::WingsBack;
use bevy_wings_ui_responsive::WingsViewport;
use bevy_wings_ui_theme::prelude::{
    ColorRole, ShapeRole, TypeRole, WingsBackground, WingsBorderPaint, WingsContentColor,
    WingsDesign, WingsElevationLevel, WingsForeground, WingsPaint, WingsShape, WingsTypeStyle,
};

use crate::button::{WingsButton, WingsButtonVariant};
use crate::glass::{WingsGlass, WingsGlassStyle};
use crate::interaction::WingsInteractive;
use crate::overlay::WingsOverlayRoot;
use crate::scaffold::chevron_scene;
use crate::segmented::WingsSelectedIndex;
use crate::skin::{
    WingsPressFeedback, WingsSkin, WingsSkinPart, clear_skin_parts, pick, skin_targets,
};
use crate::text::WingsText;

/// Something that opens a popup with items: a [`WingsMenuButton`] or a [`WingsDropdown`].
#[derive(Component, Debug, Default, Clone, PartialEq)]
pub struct WingsPopupOwner {
    /// Item labels.
    pub items: Vec<String>,
    /// The popup is open.
    pub open: bool,
    /// The open popup.
    pub popup: Option<Entity>,
}

/// A popup panel.
#[derive(Component, Debug, Clone, Copy)]
pub struct WingsPopup {
    /// The widget that opened it.
    pub owner: Entity,
}

/// One item of a popup.
#[derive(Component, Debug, Clone, Copy)]
pub struct WingsPopupItem {
    /// Position among the items.
    pub index: usize,
}

/// The transparent layer behind an open popup that closes it when clicked.
#[derive(Component, Debug, Clone, Copy)]
pub struct WingsPopupDismiss {
    /// The widget whose popup it closes.
    pub owner: Entity,
}

/// Triggered on a menu button or dropdown when an item was chosen.
#[derive(EntityEvent, Debug, Clone)]
pub struct WingsMenuSelected {
    /// The menu button or dropdown.
    pub entity: Entity,
    /// Position of the item.
    pub index: usize,
    /// Label of the item.
    pub label: String,
}

/// A button that opens a menu.
#[derive(SceneComponent, Default, Clone)]
#[scene(WingsMenuButtonProps)]
pub struct WingsMenuButton;

/// Props of [`WingsMenuButton`].
#[derive(Clone)]
pub struct WingsMenuButtonProps {
    /// Button label.
    pub label: String,
    /// Optional icon.
    pub icon: String,
    /// Menu items.
    pub items: Vec<String>,
    /// Button emphasis.
    pub variant: WingsButtonVariant,
}

impl Default for WingsMenuButtonProps {
    fn default() -> Self {
        Self {
            label: String::new(),
            icon: String::new(),
            items: Vec::new(),
            variant: WingsButtonVariant::Tonal,
        }
    }
}

impl WingsMenuButton {
    fn scene(props: WingsMenuButtonProps) -> impl Scene {
        let owner = WingsPopupOwner {
            items: props.items,
            ..default()
        };
        let (label, icon, variant) = (props.label, props.icon, props.variant);
        bsn! {
            @WingsButton { @text: label, @icon: icon, @variant: variant }
            owner
            on(toggle_popup)
        }
    }
}

/// A dropdown: shows the selected option and opens a menu with all options.
#[derive(SceneComponent, Default, Clone)]
#[scene(WingsDropdownProps)]
pub struct WingsDropdown;

/// Props of [`WingsDropdown`].
#[derive(Clone)]
pub struct WingsDropdownProps {
    /// Options.
    pub options: Vec<String>,
    /// Initially selected option.
    pub selected: usize,
    /// Width of the field.
    pub width: Val,
}

impl Default for WingsDropdownProps {
    fn default() -> Self {
        Self {
            options: Vec::new(),
            selected: 0,
            width: Val::Auto,
        }
    }
}

/// The value text of a [`WingsDropdown`].
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct WingsDropdownValue;

impl WingsDropdown {
    fn scene(props: WingsDropdownProps) -> impl Scene {
        let value = props
            .options
            .get(props.selected)
            .cloned()
            .unwrap_or_default();
        let owner = WingsPopupOwner {
            items: props.options,
            ..default()
        };
        let selected = WingsSelectedIndex(props.selected);
        let width = props.width;
        bsn! {
            Node {
                width: width,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                column_gap: px(8),
                flex_shrink: 0.0,
            }
            owner
            selected
            Button
            WingsInteractive
            TabIndex(0)
            EntityCursor::System(SystemCursorIcon::Pointer)
            WingsBackground
            WingsBorderPaint
            WingsContentColor
            WingsShape
            WingsElevationLevel
            on(toggle_popup)
            Children [
                @WingsText { @text: value, @role: TypeRole::BodyLarge }
                WingsDropdownValue
            ]
        }
    }
}

/// Observer: opens or closes the popup of the activated owner.
pub fn toggle_popup(ev: On<Activate>, mut owners: Query<&mut WingsPopupOwner>) {
    if let Ok(mut owner) = owners.get_mut(ev.event_target()) {
        owner.open = !owner.open;
    }
}

/// Popup look of a design: `(min width, padding, shape, item height, item padding, item
/// shape, label role)`.
pub fn popup_metrics(design: WingsDesign) -> (f32, f32, ShapeRole, f32, f32, ShapeRole, TypeRole) {
    match design {
        WingsDesign::Material => (
            112.0,
            8.0,
            ShapeRole::ExtraSmall,
            48.0,
            12.0,
            ShapeRole::None,
            TypeRole::BodyLarge,
        ),
        WingsDesign::Ios => (
            250.0,
            6.0,
            ShapeRole::Large,
            44.0,
            16.0,
            ShapeRole::Medium,
            TypeRole::BodyLarge,
        ),
        WingsDesign::MacOs => (
            160.0,
            5.0,
            ShapeRole::Medium,
            24.0,
            10.0,
            ShapeRole::Small,
            TypeRole::BodyMedium,
        ),
        WingsDesign::Fluent => (
            160.0,
            4.0,
            ShapeRole::Large,
            32.0,
            12.0,
            ShapeRole::Small,
            TypeRole::BodyMedium,
        ),
    }
}

/// The overlay root above `entity`'s scaffold, if any.
fn overlay_for(
    entity: Entity,
    parents: &Query<&ChildOf>,
    roots: &Query<(Entity, &WingsOverlayRoot)>,
) -> Option<Entity> {
    let ancestors: Vec<Entity> =
        bevy_wings_ui_core::hierarchy::ancestors(entity, parents).collect();
    roots
        .iter()
        .find(|(root, r)| ancestors.contains(&r.scaffold) || ancestors.contains(root))
        .map(|(root, _)| root)
        .or_else(|| roots.iter().next().map(|(root, _)| root))
}

/// Spawns and despawns popups when owners open and close.
pub fn popup_system(
    mut commands: Commands,
    skin: Res<WingsSkin>,
    mut owners: Query<
        (Entity, &mut WingsPopupOwner, Option<&WingsSelectedIndex>),
        Changed<WingsPopupOwner>,
    >,
    dismiss: Query<(Entity, &WingsPopupDismiss)>,
    parents: Query<&ChildOf>,
    roots: Query<(Entity, &WingsOverlayRoot)>,
) {
    let (min_width, padding, shape, item_height, item_padding, item_shape, role) =
        popup_metrics(skin.design);
    for (owner_entity, mut owner, selected) in &mut owners {
        match (owner.open, owner.popup) {
            (true, None) => {
                let overlay = overlay_for(owner_entity, &parents, &roots);
                let layer = commands
                    .spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: px(0),
                            right: px(0),
                            top: px(0),
                            bottom: px(0),
                            ..default()
                        },
                        GlobalZIndex(199),
                        WingsPopupDismiss {
                            owner: owner_entity,
                        },
                    ))
                    .observe(close_on_press)
                    .id();
                let popup = commands
                    .spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            flex_direction: FlexDirection::Column,
                            min_width: px(min_width),
                            padding: UiRect::all(px(padding)),
                            border: UiRect::all(px(1)),
                            ..default()
                        },
                        GlobalZIndex(200),
                        WingsPopup {
                            owner: owner_entity,
                        },
                        WingsShape(shape),
                        WingsContentColor(WingsPaint::Role(ColorRole::OnSurface)),
                        WingsElevationLevel(pick(skin.design, 2, 4, 3, 3)),
                    ))
                    .id();
                match skin.design {
                    WingsDesign::Material => {
                        commands.entity(popup).insert((
                            WingsBackground(WingsPaint::Role(ColorRole::SurfaceContainer)),
                            WingsBorderPaint(WingsPaint::Transparent),
                        ));
                    }
                    WingsDesign::Fluent => {
                        commands.entity(popup).insert((
                            WingsBackground(WingsPaint::Role(ColorRole::SurfaceContainerLowest)),
                            WingsBorderPaint(WingsPaint::Role(ColorRole::OutlineVariant)),
                        ));
                    }
                    _ => {
                        commands
                            .entity(popup)
                            .insert(WingsGlass::new(WingsGlassStyle::Regular));
                    }
                }
                if let Some(root) = overlay {
                    commands.entity(layer).insert(ChildOf(root));
                    commands.entity(popup).insert(ChildOf(root));
                }
                let feedback = match skin.design {
                    WingsDesign::Material => WingsPressFeedback::MATERIAL,
                    WingsDesign::Fluent => WingsPressFeedback::FLUENT,
                    _ => WingsPressFeedback::HIGHLIGHT,
                };
                for (index, label) in owner.items.iter().enumerate() {
                    let item = commands
                        .spawn((
                            Node {
                                min_height: px(item_height),
                                padding: UiRect::horizontal(px(item_padding)),
                                align_items: AlignItems::Center,
                                column_gap: px(8),
                                ..default()
                            },
                            Button,
                            WingsInteractive,
                            feedback,
                            TabIndex(0),
                            WingsPopupItem { index },
                            WingsShape(item_shape),
                            WingsBackground(WingsPaint::Transparent),
                            ChildOf(popup),
                        ))
                        .observe(item_activated)
                        .id();
                    let checked = selected.is_some_and(|s| s.0 == index);
                    let text = commands
                        .spawn_scene(bsn! { @{crate::text::wings_text(label.clone(), role)} })
                        .id();
                    commands.entity(text).insert(ChildOf(item));
                    if checked && skin.design.is_apple() {
                        let mark = commands
                            .spawn_scene(bsn! {
                                @{crate::selection::static_check_mark_scene(ColorRole::Primary)}
                            })
                            .id();
                        commands.entity(mark).insert(ChildOf(item));
                    }
                }
                owner.popup = Some(popup);
            }
            (false, Some(popup)) => {
                commands.entity(popup).despawn();
                for (layer, d) in &dismiss {
                    if d.owner == owner_entity {
                        commands.entity(layer).despawn();
                    }
                }
                owner.popup = None;
            }
            _ => {}
        }
    }
}

/// Observer: a click outside the popup closes it.
pub fn close_on_press(
    ev: On<PointerPress>,
    layers: Query<&WingsPopupDismiss>,
    mut owners: Query<&mut WingsPopupOwner>,
) {
    if let Ok(layer) = layers.get(ev.event_target())
        && let Ok(mut owner) = owners.get_mut(layer.owner)
    {
        owner.open = false;
    }
}

/// Observer: an item was chosen.
pub fn item_activated(
    ev: On<Activate>,
    items: Query<(&WingsPopupItem, &ChildOf)>,
    popups: Query<&WingsPopup>,
    mut owners: Query<(&mut WingsPopupOwner, Option<&mut WingsSelectedIndex>)>,
    mut commands: Commands,
) {
    let Ok((item, parent)) = items.get(ev.event_target()) else {
        return;
    };
    let Ok(popup) = popups.get(parent.parent()) else {
        return;
    };
    let Ok((mut owner, selected)) = owners.get_mut(popup.owner) else {
        return;
    };
    let label = owner.items.get(item.index).cloned().unwrap_or_default();
    owner.open = false;
    if let Some(mut selected) = selected {
        selected.0 = item.index;
    }
    commands.trigger(WingsMenuSelected {
        entity: popup.owner,
        index: item.index,
        label,
    });
}

/// Closes every open popup on back (Escape, gamepad B) before anything else uses the press.
pub fn close_popups_on_back(mut back: ResMut<WingsBack>, mut owners: Query<&mut WingsPopupOwner>) {
    if !owners.iter().any(|o| o.open) || !back.consume() {
        return;
    }
    for mut owner in &mut owners {
        if owner.open {
            owner.open = false;
        }
    }
}

/// Logical rectangle of a laid out node.
fn rect_of(transform: &UiGlobalTransform, node: &ComputedNode) -> Rect {
    let center = transform.translation * node.inverse_scale_factor;
    let size = node.size * node.inverse_scale_factor;
    Rect::from_center_size(center, size)
}

/// Places open popups below (or, without room, above) their owner, inside the window.
pub fn position_popups(
    viewport: Res<WingsViewport>,
    owners: Query<(&UiGlobalTransform, &ComputedNode), With<WingsPopupOwner>>,
    mut popups: Query<(&WingsPopup, &mut Node, &ComputedNode)>,
) {
    for (popup, mut node, size) in &mut popups {
        let Ok((transform, computed)) = owners.get(popup.owner) else {
            continue;
        };
        let anchor = rect_of(transform, computed);
        let own = size.size * size.inverse_scale_factor;
        let mut left = anchor.min.x;
        let mut top = anchor.max.y + 4.0;
        if top + own.y > viewport.height && anchor.min.y - own.y - 4.0 > 0.0 {
            top = anchor.min.y - own.y - 4.0;
        }
        left = left.min(viewport.width - own.x - 8.0).max(8.0);
        let (l, t) = (px(left), px(top));
        if node.left != l || node.top != t {
            node.left = l;
            node.top = t;
        }
    }
}

/// Keeps dropdown value texts on the selected option.
pub fn dropdown_value_system(
    dropdowns: Query<
        (&WingsPopupOwner, &WingsSelectedIndex, &Children),
        Changed<WingsSelectedIndex>,
    >,
    mut texts: Query<&mut Text, With<WingsDropdownValue>>,
) {
    for (owner, selected, children) in &dropdowns {
        let Some(label) = owner.items.get(selected.0) else {
            continue;
        };
        for child in children.iter() {
            if let Ok(mut text) = texts.get_mut(child)
                && text.0 != *label
            {
                text.0.clone_from(label);
            }
        }
    }
}

/// Skins dropdown fields.
pub fn skin_dropdowns(
    mut commands: Commands,
    skin: Res<WingsSkin>,
    added: Query<Entity, Added<WingsDropdownValue>>,
    all: Query<Entity, (With<WingsSelectedIndex>, With<WingsPopupOwner>)>,
    value_parents: Query<&ChildOf, With<WingsDropdownValue>>,
    children: Query<&Children>,
    parts: Query<(), With<WingsSkinPart>>,
    mut fields: Query<
        (
            &mut Node,
            &mut WingsBackground,
            &mut WingsBorderPaint,
            &mut WingsContentColor,
            &mut WingsShape,
            &mut WingsElevationLevel,
        ),
        With<WingsPopupOwner>,
    >,
    mut values: Query<
        (&mut WingsTypeStyle, Option<&mut WingsForeground>),
        With<WingsDropdownValue>,
    >,
) {
    use ColorRole::*;
    let added_fields: Vec<Entity> = added
        .iter()
        .filter_map(|v| value_parents.get(v).ok().map(|p| p.parent()))
        .collect();
    for entity in skin_targets(&skin, added_fields, all) {
        let Ok((mut node, mut bg, mut border, mut content, mut shape, mut elevation)) =
            fields.get_mut(entity)
        else {
            continue;
        };
        let r = WingsPaint::Role;
        let t = WingsPaint::Transparent;
        let (height, pad, fill, line, text_color, corners, shadow, role) = match skin.design {
            WingsDesign::Material => (
                48.0,
                16.0,
                t,
                r(Outline),
                OnSurface,
                ShapeRole::ExtraSmall,
                0,
                TypeRole::BodyLarge,
            ),
            WingsDesign::Ios => (
                34.0,
                8.0,
                t,
                t,
                Primary,
                ShapeRole::Small,
                0,
                TypeRole::BodyLarge,
            ),
            WingsDesign::MacOs => (
                24.0,
                8.0,
                r(SurfaceContainerLowest),
                r(Outline),
                OnSurface,
                ShapeRole::Small,
                1,
                TypeRole::BodyMedium,
            ),
            WingsDesign::Fluent => (
                32.0,
                11.0,
                r(SurfaceContainerLowest),
                r(OutlineVariant),
                OnSurface,
                ShapeRole::Small,
                0,
                TypeRole::BodyMedium,
            ),
        };
        node.min_height = px(height);
        node.padding = UiRect::horizontal(px(pad));
        node.border = UiRect::all(px(if line == t { 0.0 } else { 1.0 }));
        bg.set_if_neq(WingsBackground(fill));
        border.set_if_neq(WingsBorderPaint(line));
        content.set_if_neq(WingsContentColor(r(text_color)));
        shape.set_if_neq(WingsShape(corners));
        elevation.set_if_neq(WingsElevationLevel(shadow));
        for child in children.iter_descendants(entity) {
            if let Ok((mut style, _)) = values.get_mut(child) {
                style.set_if_neq(WingsTypeStyle(role));
            }
        }
        // Trailing indicator: a caret (Material, Fluent) or up and down chevrons (Apple).
        clear_skin_parts(&mut commands, entity, &children, &parts);
        let indicator = commands
            .spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: px(1),
                    ..default()
                },
                Pickable::IGNORE,
                WingsSkinPart,
                ChildOf(entity),
            ))
            .id();
        let chevron_color = if skin.design == WingsDesign::Ios {
            Primary
        } else {
            OnSurfaceVariant
        };
        let angles: &[f32] = if skin.design.is_apple() {
            &[-90.0, 90.0]
        } else {
            &[90.0]
        };
        for angle in angles {
            let scale = if skin.design.is_apple() { 0.55 } else { 0.8 };
            let chevron = commands
                .spawn_scene(bsn! { @{chevron_scene(chevron_color)} })
                .id();
            commands.entity(chevron).insert((
                UiTransform {
                    rotation: Rot2::degrees(*angle),
                    scale: Vec2::splat(scale),
                    ..default()
                },
                ChildOf(indicator),
            ));
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Tooltip
// ---------------------------------------------------------------------------------------------

/// Shows `text` in a small bubble while the pointer rests on the node.
#[derive(Component, Debug, Clone, PartialEq)]
#[require(Hovered)]
pub struct WingsTooltip {
    /// Tooltip text.
    pub text: String,
    /// Seconds of hovering before it shows.
    pub delay: f32,
}

impl Default for WingsTooltip {
    fn default() -> Self {
        Self::new("")
    }
}

impl WingsTooltip {
    /// A tooltip with the default delay.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            delay: 0.6,
        }
    }
}

/// Hover timer and bubble of a tooltip.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct WingsTooltipState {
    /// Seconds hovered.
    pub hovered: f32,
    /// The visible bubble.
    pub bubble: Option<Entity>,
}

/// The bubble of a tooltip.
#[derive(Component, Debug, Clone, Copy)]
pub struct WingsTooltipBubble {
    /// The node it describes.
    pub owner: Entity,
}

/// Shows and hides tooltip bubbles.
pub fn tooltip_system(
    mut commands: Commands,
    time: Res<Time>,
    skin: Res<WingsSkin>,
    mut tips: Query<(
        Entity,
        &WingsTooltip,
        &Hovered,
        Option<&mut WingsTooltipState>,
    )>,
    parents: Query<&ChildOf>,
    roots: Query<(Entity, &WingsOverlayRoot)>,
) {
    for (entity, tip, hovered, state) in &mut tips {
        let Some(mut state) = state else {
            commands.entity(entity).insert(WingsTooltipState::default());
            continue;
        };
        if hovered.0 {
            state.hovered += time.delta_secs();
        } else {
            state.hovered = 0.0;
        }
        let show = hovered.0 && state.hovered >= tip.delay;
        match (show, state.bubble) {
            (true, None) => {
                let (fill, text_color, border, corners) = match skin.design {
                    WingsDesign::Material => (
                        WingsPaint::Role(ColorRole::InverseSurface),
                        ColorRole::InverseOnSurface,
                        WingsPaint::Transparent,
                        ShapeRole::ExtraSmall,
                    ),
                    _ => (
                        WingsPaint::Role(ColorRole::SurfaceContainerLowest),
                        ColorRole::OnSurface,
                        WingsPaint::Role(ColorRole::OutlineVariant),
                        if skin.ios() {
                            ShapeRole::Full
                        } else {
                            ShapeRole::Small
                        },
                    ),
                };
                let bubble = commands
                    .spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            padding: UiRect::axes(px(8), px(4)),
                            border: UiRect::all(px(if border == WingsPaint::Transparent {
                                0.0
                            } else {
                                1.0
                            })),
                            ..default()
                        },
                        GlobalZIndex(210),
                        Pickable::IGNORE,
                        WingsTooltipBubble { owner: entity },
                        WingsBackground(fill),
                        WingsBorderPaint(border),
                        WingsShape(corners),
                        WingsElevationLevel(if skin.design == WingsDesign::Material {
                            0
                        } else {
                            2
                        }),
                        WingsContentColor(WingsPaint::Role(text_color)),
                    ))
                    .id();
                if let Some(root) = overlay_for(entity, &parents, &roots) {
                    commands.entity(bubble).insert(ChildOf(root));
                }
                let text = commands
                    .spawn_scene(
                        bsn! { @{crate::text::wings_text(tip.text.clone(), TypeRole::BodySmall)} },
                    )
                    .id();
                commands.entity(text).insert(ChildOf(bubble));
                state.bubble = Some(bubble);
            }
            (false, Some(bubble)) => {
                commands.entity(bubble).despawn();
                state.bubble = None;
            }
            _ => {}
        }
    }
}

/// Places tooltip bubbles centered below their node.
pub fn position_tooltips(
    viewport: Res<WingsViewport>,
    owners: Query<(&UiGlobalTransform, &ComputedNode), With<WingsTooltip>>,
    mut bubbles: Query<(&WingsTooltipBubble, &mut Node, &ComputedNode)>,
) {
    for (bubble, mut node, size) in &mut bubbles {
        let Ok((transform, computed)) = owners.get(bubble.owner) else {
            continue;
        };
        let anchor = rect_of(transform, computed);
        let own = size.size * size.inverse_scale_factor;
        let left =
            (anchor.center().x - own.x * 0.5).clamp(4.0, (viewport.width - own.x - 4.0).max(4.0));
        let mut top = anchor.max.y + 6.0;
        if top + own.y > viewport.height {
            top = anchor.min.y - own.y - 6.0;
        }
        node.left = px(left);
        node.top = px(top);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Item heights follow each platform.
    #[test]
    fn popup_item_heights() {
        assert_eq!(popup_metrics(WingsDesign::Material).3, 48.0);
        assert_eq!(popup_metrics(WingsDesign::Ios).3, 44.0);
        assert_eq!(popup_metrics(WingsDesign::MacOs).3, 24.0);
        assert_eq!(popup_metrics(WingsDesign::Fluent).3, 32.0);
    }
}
