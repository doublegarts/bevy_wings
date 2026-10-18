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
    ColorRole, ShapeRole, TypeRole, WingsBackground, WingsBorderPaint, WingsContentColor,
    WingsElevationLevel, WingsPaint, WingsShape,
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
    /// How the title is shown (large or inline).
    pub title_display: WingsTitleDisplay,
}

/// How a [`WingsTopAppBar`] shows its title, like SwiftUI's `navigationBarTitleDisplayMode`.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WingsTitleDisplay {
    /// Large on root screens in iOS (no navigation icon), inline everywhere else.
    #[default]
    Automatic,
    /// A large title below the toolbar that collapses to inline while content scrolls (iOS).
    Large,
    /// A small title inside the toolbar.
    Inline,
}

/// Marks the leading navigation icon button of a top app bar.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct WingsTopAppBarNavigation;

/// Marks the title of a top app bar.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct WingsTopAppBarTitle;

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
        let display = props.title_display;
        bsn! {
            Node {
                height: px(64),
                flex_shrink: 0.0,
                padding: UiRect::horizontal(px(8)),
                column_gap: px(4),
                align_items: AlignItems::Center,
            }
            display
            WingsBackground({WingsPaint::Role(ColorRole::Surface)})
            WingsBorderPaint
            WingsContentColor({WingsPaint::Role(ColorRole::OnSurface)})
            @{nav}
            Children [
                @WingsText { @text: title, @role: TypeRole::TitleLarge }
                Node { flex_grow: 1.0, margin: UiRect::horizontal(px(8)) }
                TextLayout
                WingsTopAppBarTitle
                Pickable::IGNORE
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
            WingsShape(ShapeRole::None)
            WingsBorderPaint
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
            WingsShape(ShapeRole::None)
            WingsBorderPaint
            WingsElevationLevel
            WingsBackground({WingsPaint::Role(ColorRole::Surface)})
        }
    }
}

/// The icon container ("pill") of a [`WingsNavigationItem`].
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct WingsNavigationPill;

/// The label of a [`WingsNavigationItem`].
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct WingsNavigationLabel;

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
        let icon_scene: Box<dyn Scene> = if props.icon.is_empty() {
            Box::new(bsn! { Node })
        } else {
            let image = props.icon;
            Box::new(bsn! { @WingsIcon { @image: image, @size: 24.0 } })
        };
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
            WingsBackground
            WingsShape(ShapeRole::Full)
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
                WingsNavigationPill
                pill
                Children [ @{icon_scene} ]
                --
                @WingsText { @text: label, @role: TypeRole::LabelMedium }
                WingsNavigationLabel
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

/// What a [`WingsListItem`] shows at its trailing edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WingsListAccessory {
    /// Nothing.
    #[default]
    None,
    /// A disclosure chevron: the row opens another screen.
    Chevron,
    /// A checkmark: the row is the selected option.
    Checkmark,
}

/// The headline of a [`WingsListItem`].
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct WingsListItemHeadline;

/// The supporting text of a [`WingsListItem`].
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct WingsListItemSupporting;

/// The trailing detail text of a [`WingsListItem`].
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct WingsListItemDetail;

/// The leading icon of a [`WingsListItem`].
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct WingsListItemIcon;

/// A clickable row with a headline, optional supporting text, an optional leading icon and an
/// optional trailing detail and accessory.
///
/// ```ignore
/// bsn! {
///     @WingsListItem {
///         @headline: "Wi-Fi",
///         @detail: "Home",
///         @accessory: WingsListAccessory::Chevron,
///     }
/// }
/// ```
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
    /// Trailing value text, like the current setting. Empty for none.
    pub detail: String,
    /// Trailing accessory.
    pub accessory: WingsListAccessory,
}

/// A small disclosure chevron (`>`), 7 by 12 pixels.
pub(crate) fn chevron_scene(color: ColorRole) -> impl Scene {
    let stroke = |top: f32, angle: f32| {
        Node {
            position_type: PositionType::Absolute,
            left: px(-0.5),
            top: px(top),
            width: px(8.5),
            height: px(2),
            border_radius: BorderRadius::all(px(1)),
            ..default()
        }
        .clone()
        .with_rotation(angle)
    };
    let (upper, upper_rot) = stroke(2.4, 45.0_f32.to_radians());
    let (lower, lower_rot) = stroke(7.6, -45.0_f32.to_radians());
    bsn! {
        Node { width: px(7), height: px(12), flex_shrink: 0.0, margin: UiRect::left(px(4)) }
        Pickable::IGNORE
        Children [
            upper
            upper_rot
            WingsBackground({WingsPaint::Role(color)})
            --
            lower
            lower_rot
            WingsBackground({WingsPaint::Role(color)})
        ]
    }
}

/// Pairs a node with a rotation transform.
trait WithRotation {
    fn with_rotation(self, angle: f32) -> (Node, UiTransform);
}

impl WithRotation for Node {
    fn with_rotation(self, angle: f32) -> (Node, UiTransform) {
        (self, UiTransform::from_rotation(Rot2::radians(angle)))
    }
}

impl WingsListItem {
    fn scene(props: WingsListItemProps) -> impl Scene {
        let icon: Box<dyn Scene> = if props.icon.is_empty() {
            Box::new(bsn! {})
        } else {
            let image = props.icon;
            Box::new(bsn! {
                Children [ @WingsIcon { @image: image, @size: 24.0 } WingsListItemIcon ]
            })
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
                    WingsListItemSupporting
                ]
            })
        };
        let detail: Box<dyn Scene> = if props.detail.is_empty() {
            Box::new(bsn! {})
        } else {
            let text = props.detail;
            Box::new(bsn! {
                Children [
                    @WingsText {
                        @text: text,
                        @role: TypeRole::BodyLarge,
                        @color: ColorRole::OnSurfaceVariant,
                    }
                    WingsListItemDetail
                ]
            })
        };
        let accessory: Box<dyn Scene> = match props.accessory {
            WingsListAccessory::None => Box::new(bsn! {}),
            WingsListAccessory::Chevron => Box::new(bsn! {
                Children [ @{chevron_scene(ColorRole::TertiaryLabel)} ]
            }),
            WingsListAccessory::Checkmark => Box::new(bsn! {
                Children [ @{crate::selection::static_check_mark_scene(ColorRole::Primary)} ]
            }),
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
            WingsBorderPaint
            @{icon}
            Children [
                Node { flex_direction: FlexDirection::Column, flex_grow: 1.0 }
                Children [
                    @WingsText { @text: headline, @role: TypeRole::BodyLarge }
                    WingsListItemHeadline
                ]
                @{supporting}
            ]
            @{detail}
            @{accessory}
        }
    }
}
