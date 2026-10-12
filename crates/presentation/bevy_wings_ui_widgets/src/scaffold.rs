//! Screen structure: [`WingsScaffold`], [`WingsTopAppBar`], [`WingsNavigationBar`],
//! [`WingsNavigationRail`], [`WingsNavigationItem`] and [`WingsListItem`].
//!
//! The navigation bar and the navigation rail are responsive out of the box: the bar shows on
//! compact windows and the rail on medium and wider windows, so one scene adapts from phones
//! to desktops:
//!
//! ```ignore
//! bsn! {
//!     @WingsScaffold
//!     Children [
//!         @WingsTopAppBar { @title: "Inbox" }
//!         --
//!         @WingsRow { @align: AlignItems::Stretch } Node { flex_grow: 1.0 }
//!         Children [
//!             @WingsNavigationRail Children [ /* items */ ]
//!             --
//!             @WingsScrollColumn Node { flex_grow: 1.0 } Children [ /* content */ ]
//!         ]
//!         --
//!         @WingsNavigationBar Children [ /* items */ ]
//!     ]
//! }
//! ```

use bevy::input_focus::tab_navigation::TabIndex;
use bevy::picking::cursor::EntityCursor;
use bevy::prelude::*;
use bevy::ui::Checked;
use bevy::ui_widgets::{Activate, Button};
use bevy::window::SystemCursorIcon;

use bevy_wings_ui_responsive::{WidthClass, WingsResponsiveVisibility};
use bevy_wings_ui_theme::prelude::{
    ColorRole, ShapeRole, TypeRole, WingsBackground, WingsContentColor, WingsElevationLevel,
    WingsPaint, WingsShape,
};

use crate::button::WingsIconButton;
use crate::feedback::WingsSnackbarHost;
use crate::interaction::WingsInteractive;
use crate::selection::{WingsCheckedOwner, WingsCheckedPart};
use crate::text::{WingsIcon, WingsText};

/// Full window screen layout with a themed background and a snackbar host.
#[derive(SceneComponent, Default, Clone)]
pub struct WingsScaffold;

impl WingsScaffold {
    fn scene() -> impl Scene {
        bsn! {
            Node {
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
            }
            WingsBackground({WingsPaint::Role(ColorRole::Background)})
            WingsContentColor({WingsPaint::Role(ColorRole::OnBackground)})
            Children [
                Node {
                    position_type: PositionType::Absolute,
                    left: px(0),
                    right: px(0),
                    bottom: px(88),
                    justify_content: JustifyContent::Center,
                }
                WingsSnackbarHost
                GlobalZIndex(50)
            ]
        }
    }
}

/// Top app bar with an optional navigation icon and a title. Children (action buttons) are
/// placed at the trailing edge.
#[derive(SceneComponent, Default, Clone)]
#[scene(WingsTopAppBarProps)]
pub struct WingsTopAppBar;

/// Props of [`WingsTopAppBar`].
#[derive(Default, Clone)]
pub struct WingsTopAppBarProps {
    /// Title text.
    pub title: String,
    /// Optional leading navigation icon (menu, back arrow).
    pub navigation_icon: String,
}

/// Marks the leading navigation icon button of a top app bar.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct WingsTopAppBarNavigation;

impl WingsTopAppBar {
    fn scene(props: WingsTopAppBarProps) -> impl Scene {
        let nav: Box<dyn Scene> = if props.navigation_icon.is_empty() {
            Box::new(bsn! {})
        } else {
            let icon = props.navigation_icon;
            Box::new(
                bsn! { Children [ @WingsIconButton { @icon: icon } WingsTopAppBarNavigation ] },
            )
        };
        let title = props.title;
        bsn! {
            Node {
                height: px(64),
                flex_shrink: 0.0,
                padding: UiRect::horizontal(px(8)),
                column_gap: px(4),
                align_items: AlignItems::Center,
            }
            WingsBackground({WingsPaint::Role(ColorRole::Surface)})
            WingsContentColor({WingsPaint::Role(ColorRole::OnSurface)})
            @{nav}
            Children [
                @WingsText { @text: title, @role: TypeRole::TitleLarge }
                Node { flex_grow: 1.0, margin: UiRect::horizontal(px(8)) }
            ]
        }
    }
}

/// Triggered on a navigation bar or rail when an item was selected.
#[derive(EntityEvent, Debug, Clone, Copy)]
pub struct WingsNavigationSelected {
    /// The bar or rail.
    pub entity: Entity,
    /// The selected item.
    pub item: Entity,
    /// Index of the item among the bar's items.
    pub index: usize,
}

/// Marks a container whose [`WingsNavigationItem`] children are mutually exclusive.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct WingsNavigationContainer;

/// Bottom navigation bar, visible on compact windows.
#[derive(SceneComponent, Default, Clone)]
pub struct WingsNavigationBar;

impl WingsNavigationBar {
    fn scene() -> impl Scene {
        let responsive = WingsResponsiveVisibility::below(WidthClass::Medium);
        bsn! {
            Node {
                height: px(80),
                flex_shrink: 0.0,
                justify_content: JustifyContent::SpaceAround,
                align_items: AlignItems::Center,
            }
            responsive
            WingsNavigationContainer
            WingsElevationLevel(2)
            WingsBackground({WingsPaint::Role(ColorRole::SurfaceContainer)})
        }
    }
}

/// Side navigation rail, visible on medium and wider windows.
#[derive(SceneComponent, Default, Clone)]
pub struct WingsNavigationRail;

impl WingsNavigationRail {
    fn scene() -> impl Scene {
        let responsive = WingsResponsiveVisibility::from(WidthClass::Medium);
        bsn! {
            Node {
                width: px(80),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                padding: UiRect::vertical(px(12)),
                row_gap: px(12),
                align_items: AlignItems::Center,
            }
            responsive
            WingsNavigationContainer
            WingsBackground({WingsPaint::Role(ColorRole::Surface)})
        }
    }
}

/// A destination in a [`WingsNavigationBar`] or [`WingsNavigationRail`].
#[derive(SceneComponent, Default, Clone)]
#[scene(WingsNavigationItemProps)]
pub struct WingsNavigationItem;

/// Props of [`WingsNavigationItem`].
#[derive(Default, Clone)]
pub struct WingsNavigationItemProps {
    /// Icon asset path.
    pub icon: String,
    /// Label below the icon.
    pub label: String,
    /// Initially selected.
    pub selected: bool,
}

impl WingsNavigationItem {
    fn scene(props: WingsNavigationItemProps) -> impl Scene {
        let selected: Box<dyn Scene> = if props.selected {
            Box::new(bsn! { Checked })
        } else {
            Box::new(bsn! {})
        };
        let pill = WingsCheckedPart {
            background: Some((
                WingsPaint::Role(ColorRole::SecondaryContainer),
                WingsPaint::Transparent,
            )),
            content: Some((
                WingsPaint::Role(ColorRole::OnSecondaryContainer),
                WingsPaint::Role(ColorRole::OnSurfaceVariant),
            )),
            ..default()
        };
        let label_part = WingsCheckedPart {
            content: Some((
                WingsPaint::Role(ColorRole::OnSurface),
                WingsPaint::Role(ColorRole::OnSurfaceVariant),
            )),
            ..default()
        };
        let icon = props.icon;
        let label = props.label;
        bsn! {
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: px(4),
                min_width: px(56),
            }
            Button
            WingsCheckedOwner
            TabIndex(0)
            EntityCursor::System(SystemCursorIcon::Pointer)
            WingsContentColor
            label_part
            on(navigation_item_activated)
            @{selected}
            Children [
                Node {
                    width: px(64),
                    height: px(32),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                }
                WingsShape(ShapeRole::Full)
                WingsInteractive
                WingsBackground
                WingsContentColor
                pill
                Children [ @WingsIcon { @image: icon, @size: 24.0 } ]
                --
                @WingsText { @text: label, @role: TypeRole::LabelMedium }
            ]
        }
    }
}

/// Observer: selects the activated item and deselects its siblings.
pub fn navigation_item_activated(
    ev: On<Activate>,
    mut commands: Commands,
    parents: Query<&ChildOf>,
    containers: Query<&Children, With<WingsNavigationContainer>>,
    items: Query<Has<Checked>, With<WingsNavigationItem>>,
) {
    let item = ev.event_target();
    let Ok(parent) = parents.get(item) else {
        return;
    };
    let container = parent.parent();
    let Ok(children) = containers.get(container) else {
        return;
    };
    let mut index = 0;
    let mut position = None;
    for child in children.iter() {
        let Ok(is_checked) = items.get(child) else {
            continue;
        };
        if child == item {
            position = Some(index);
            if !is_checked {
                commands.entity(child).insert(Checked);
            }
        } else if is_checked {
            commands.entity(child).remove::<Checked>();
        }
        index += 1;
    }
    if let Some(index) = position {
        commands.trigger(WingsNavigationSelected {
            entity: container,
            item,
            index,
        });
    }
}

/// A clickable row with a headline, optional supporting text and optional leading icon.
#[derive(SceneComponent, Default, Clone)]
#[scene(WingsListItemProps)]
pub struct WingsListItem;

/// Props of [`WingsListItem`].
#[derive(Default, Clone)]
pub struct WingsListItemProps {
    /// Main text.
    pub headline: String,
    /// Secondary text. Empty for a one line item.
    pub supporting: String,
    /// Leading icon asset path. Empty for none.
    pub icon: String,
}

impl WingsListItem {
    fn scene(props: WingsListItemProps) -> impl Scene {
        let icon: Box<dyn Scene> = if props.icon.is_empty() {
            Box::new(bsn! {})
        } else {
            let image = props.icon;
            Box::new(bsn! { Children [ @WingsIcon { @image: image, @size: 24.0 } ] })
        };
        let supporting: Box<dyn Scene> = if props.supporting.is_empty() {
            Box::new(bsn! {})
        } else {
            let text = props.supporting;
            Box::new(bsn! {
                Children [
                    @WingsText {
                        @text: text,
                        @role: TypeRole::BodyMedium,
                        @color: ColorRole::OnSurfaceVariant,
                    }
                ]
            })
        };
        let headline = props.headline;
        bsn! {
            Node {
                min_height: px(56),
                padding: UiRect::axes(px(16), px(8)),
                column_gap: px(16),
                align_items: AlignItems::Center,
            }
            Button
            WingsInteractive
            TabIndex(0)
            EntityCursor::System(SystemCursorIcon::Pointer)
            WingsBackground
            @{icon}
            Children [
                Node { flex_direction: FlexDirection::Column, flex_grow: 1.0 }
                Children [ @WingsText { @text: headline, @role: TypeRole::BodyLarge } ]
                @{supporting}
            ]
        }
    }
}
