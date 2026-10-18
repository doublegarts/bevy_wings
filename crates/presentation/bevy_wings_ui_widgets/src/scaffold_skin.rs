//! Design skins of the screen structure: [`WingsScaffold`], [`WingsTopAppBar`],
//! [`WingsNavigationBar`], [`WingsNavigationRail`], [`WingsNavigationItem`] and
//! [`WingsListItem`].
//!
//! | Widget | Material | iOS | macOS | Fluent |
//! | --- | --- | --- | --- | --- |
//! | Top bar | 64 tall | navigation bar, large title collapsing to inline | 52 tall unified toolbar | 48 tall command bar |
//! | Navigation bar | 80 tall bottom bar | floating tab bar capsule | hidden (sidebar) | hidden (navigation pane) |
//! | Navigation rail | 80 wide rail | sidebar on wide windows | floating sidebar | navigation pane |
//! | List item | 56 tall | 52 tall row | 36 tall row | 40 tall row |

use bevy::prelude::*;
use bevy::text::{Justify, TextLayout};
use bevy::ui_widgets::ScrollArea;

use bevy_wings_ui_responsive::{
    WidthClass, WingsEdges, WingsResponsiveVisibility, WingsSafeArea, WingsSafeAreaPadding,
    WingsViewport,
};
use bevy_wings_ui_theme::prelude::{
    ColorRole, ShapeRole, TypeRole, WingsBackground, WingsBorderPaint, WingsContentColor,
    WingsDesign, WingsElevationLevel, WingsPaint, WingsShape, WingsTypeOverride, WingsTypeStyle,
};

use crate::feedback::WingsSnackbarHost;
use crate::glass::{WingsGlass, WingsGlassStyle};
use crate::interaction::WingsInteractive;
use crate::overlay::{WingsFloating, WingsOverlayRoot};
use crate::scaffold::{
    WingsListItem, WingsListItemDetail, WingsListItemHeadline, WingsListItemIcon,
    WingsListItemSupporting, WingsNavigationBar, WingsNavigationItem, WingsNavigationLabel,
    WingsNavigationPill, WingsNavigationRail, WingsScaffold, WingsTitleDisplay, WingsTopAppBar,
    WingsTopAppBarNavigation, WingsTopAppBarTitle,
};
use crate::selection::{WingsCheckedPart, WingsCheckedRefresh};
use crate::skin::{
    WingsPressFeedback, WingsSkin, WingsSkinPart, clear_skin_parts, pick, skin_targets,
};
use crate::text::WingsIconTint;

use ColorRole::*;

const T: WingsPaint = WingsPaint::Transparent;

fn role(r: ColorRole) -> WingsPaint {
    WingsPaint::Role(r)
}

// ---------------------------------------------------------------------------------------------
// Scaffold
// ---------------------------------------------------------------------------------------------

/// Every descendant of a scaffold, including chrome that floats in its overlay root.
fn scaffold_tree(
    scaffold: Entity,
    children: &Query<&Children>,
    roots: &Query<(Entity, &WingsOverlayRoot)>,
) -> Vec<Entity> {
    let mut all: Vec<Entity> = children.iter_descendants(scaffold).collect();
    for (root, r) in roots {
        if r.scaffold == scaffold {
            all.extend(children.iter_descendants(root));
        }
    }
    all
}

/// Skins scaffolds: places the snackbar host above the design's bottom chrome.
pub fn skin_scaffolds(
    skin: Res<WingsSkin>,
    added: Query<Entity, Added<WingsScaffold>>,
    all: Query<Entity, With<WingsScaffold>>,
    children: Query<&Children>,
    mut hosts: Query<&mut Node, With<WingsSnackbarHost>>,
) {
    let bottom = pick(skin.design, 88.0, 100.0, 24.0, 24.0);
    for entity in skin_targets(&skin, added, all) {
        for child in children.iter_descendants(entity) {
            if let Ok(mut node) = hosts.get_mut(child) {
                node.bottom = px(bottom);
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Top app bar
// ---------------------------------------------------------------------------------------------

/// The resolved title display of a top bar in the active design.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WingsResolvedTitle {
    /// Shows the large title (iOS only).
    pub large: bool,
    /// The large title collapsed into the toolbar because content scrolled.
    pub collapsed: bool,
}

/// Height of the iOS toolbar row above a large title.
pub const IOS_TOOLBAR: f32 = 52.0;
/// Height the iOS large title adds below the toolbar row.
pub const IOS_LARGE_TITLE: f32 = 52.0;

/// Skins top bars.
pub fn skin_top_app_bars(
    mut commands: Commands,
    skin: Res<WingsSkin>,
    added: Query<Entity, Or<(Added<WingsTopAppBar>, Changed<WingsTitleDisplay>)>>,
    all: Query<Entity, With<WingsTopAppBar>>,
    bars: Query<(&WingsTitleDisplay, &Children, Option<&WingsResolvedTitle>), With<WingsTopAppBar>>,
    navs: Query<(), With<WingsTopAppBarNavigation>>,
) {
    for entity in skin_targets(&skin, added, all) {
        let Ok((display, children, resolved)) = bars.get(entity) else {
            continue;
        };
        let has_nav = children.iter().any(|c| navs.contains(c));
        let large = skin.ios()
            && match display {
                WingsTitleDisplay::Automatic => !has_nav,
                WingsTitleDisplay::Large => true,
                WingsTitleDisplay::Inline => false,
            };
        let collapsed = resolved.is_some_and(|r| r.collapsed);
        commands
            .entity(entity)
            .insert(WingsResolvedTitle { large, collapsed });
    }
}

/// Lays out top bars whose design or title state changed.
pub fn layout_top_app_bars(
    skin: Res<WingsSkin>,
    safe: Res<WingsSafeArea>,
    mut bars: Query<
        (
            Entity,
            Ref<WingsResolvedTitle>,
            &Children,
            &mut Node,
            &mut WingsBackground,
            &mut WingsBorderPaint,
        ),
        With<WingsTopAppBar>,
    >,
    mut titles: Query<
        (
            &mut Node,
            &mut WingsTypeStyle,
            &mut TextLayout,
            Option<&mut WingsTypeOverride>,
        ),
        (With<WingsTopAppBarTitle>, Without<WingsTopAppBar>),
    >,
    mut navs: Query<
        &mut Node,
        (
            With<WingsTopAppBarNavigation>,
            Without<WingsTopAppBar>,
            Without<WingsTopAppBarTitle>,
        ),
    >,
    mut commands: Commands,
) {
    // The bar's background extends under the status bar, notch or Dynamic Island; its content
    // starts below them.
    let inset = safe.insets.top;
    for (bar_entity, resolved, children, mut node, mut bg, mut border) in &mut bars {
        if !resolved.is_changed() && !skin.is_changed() && !safe.is_changed() {
            continue;
        }
        let large = resolved.large && !resolved.collapsed;
        let (height, pad_x, background) = match skin.design {
            WingsDesign::Material => (64.0, 8.0, role(Surface)),
            // iOS 26 navigation bars have no background: the screen shows through.
            WingsDesign::Ios if large => (IOS_TOOLBAR + IOS_LARGE_TITLE, 8.0, T),
            WingsDesign::Ios => (IOS_TOOLBAR, 8.0, T),
            WingsDesign::MacOs => (52.0, 12.0, T),
            WingsDesign::Fluent => (48.0, 12.0, role(Background)),
        };
        node.height = px(height + inset);
        node.padding = UiRect::horizontal(px(pad_x));
        node.column_gap = px(pick(skin.design, 4.0, 8.0, 8.0, 4.0));
        node.align_items = if large {
            AlignItems::FlexStart
        } else {
            AlignItems::Center
        };
        node.padding.top = px(inset + if large { 4.0 } else { 0.0 });
        bg.set_if_neq(WingsBackground(background));
        border.set_if_neq(WingsBorderPaint(T));
        // Scrolled content slides under a frosted bar (the scroll edge effect).
        if skin.apple() && resolved.collapsed {
            commands
                .entity(bar_entity)
                .insert(WingsGlass::new(WingsGlassStyle::Regular).without_rim());
        } else {
            commands.entity(bar_entity).remove::<WingsGlass>();
        }
        let ios = skin.ios();
        for child in children.iter() {
            if let Ok(mut nav) = navs.get_mut(child) {
                // With an absolute title the row is empty: the navigation button sticks to the
                // leading edge, actions to the trailing edge.
                nav.margin.right = if ios { Val::Auto } else { px(0) };
                continue;
            }
            let Ok((mut title, mut type_style, mut layout, adjust)) = titles.get_mut(child) else {
                continue;
            };
            let (role_for, centered, absolute) = match skin.design {
                WingsDesign::Ios if large => (TypeRole::DisplayLarge, false, true),
                WingsDesign::Ios => (TypeRole::TitleLarge, true, true),
                _ => (TypeRole::TitleLarge, false, false),
            };
            type_style.set_if_neq(WingsTypeStyle(role_for));
            layout.justify = if centered {
                Justify::Center
            } else {
                Justify::Left
            };
            if absolute {
                title.position_type = PositionType::Absolute;
                if large {
                    title.left = px(16);
                    title.right = Val::Auto;
                    title.top = px(inset + IOS_TOOLBAR);
                    title.bottom = Val::Auto;
                } else {
                    title.left = px(0);
                    title.right = px(0);
                    title.top = px(inset);
                    title.bottom = px(0);
                }
                title.align_self = AlignSelf::Center;
                title.margin = UiRect::ZERO;
            } else {
                title.position_type = PositionType::Relative;
                title.left = Val::Auto;
                title.right = Val::Auto;
                title.top = Val::Auto;
                title.bottom = Val::Auto;
                title.margin = UiRect::horizontal(px(8));
            }
            // A centered absolute title leaves the row empty: keep actions trailing.
            title.flex_grow = if absolute { 0.0 } else { 1.0 };
            let wanted = match skin.design {
                WingsDesign::MacOs => Some(WingsTypeOverride {
                    size: Some(15.0),
                    weight: Some(bevy::text::FontWeight::BOLD),
                }),
                _ => None,
            };
            match (adjust, wanted) {
                (Some(mut a), Some(w)) => {
                    a.set_if_neq(w);
                }
                (None, Some(w)) => {
                    commands.entity(child).insert(w);
                }
                (Some(_), None) => {
                    commands.entity(child).remove::<WingsTypeOverride>();
                }
                (None, None) => {}
            }
        }
        node.justify_content = if ios {
            JustifyContent::End
        } else {
            JustifyContent::Start
        };
    }
}

/// Collapses iOS large titles while the content below them scrolls, like a navigation stack.
pub fn collapse_large_titles(
    skin: Res<WingsSkin>,
    mut bars: Query<
        (&ChildOf, Option<&WingsFloating>, &mut WingsResolvedTitle),
        With<WingsTopAppBar>,
    >,
    children: Query<&Children>,
    parents: Query<&ChildOf>,
    scrolls: Query<(&ScrollPosition, &InheritedVisibility), With<ScrollArea>>,
) {
    if !skin.apple() {
        return;
    }
    for (parent, floating, mut resolved) in &mut bars {
        // Floating bars look for scrolling content in their scaffold. Only the screen's own
        // scroll counts, not lists or carousels nested inside it.
        let host = floating.map_or(parent.parent(), |f| f.scaffold);
        let scrolled = children
            .iter_descendants(host)
            .filter(|e| {
                scrolls.contains(*e)
                    && !parents
                        .iter_ancestors(*e)
                        .take_while(|a| *a != host)
                        .any(|a| scrolls.contains(a))
            })
            .filter_map(|e| scrolls.get(e).ok())
            .filter(|(_, visible)| visible.get())
            .any(|(position, _)| position.y > 24.0);
        if resolved.collapsed != scrolled {
            resolved.collapsed = scrolled;
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Navigation bar and rail
// ---------------------------------------------------------------------------------------------

/// Skins bottom navigation bars. Apple and Fluent desktops never show one; iOS floats a
/// capsule tab bar above the content.
pub fn skin_navigation_bars(
    mut commands: Commands,
    skin: Res<WingsSkin>,
    added: Query<Entity, Added<WingsNavigationBar>>,
    all: Query<Entity, With<WingsNavigationBar>>,
    mut bars: Query<
        (
            &mut Node,
            &mut WingsBackground,
            &mut WingsBorderPaint,
            &mut WingsShape,
            &mut WingsElevationLevel,
        ),
        With<WingsNavigationBar>,
    >,
) {
    for entity in skin_targets(&skin, added, all) {
        let Ok((mut node, mut bg, mut border, mut shape, mut elevation)) = bars.get_mut(entity)
        else {
            continue;
        };
        let visibility = if skin.design.is_desktop() {
            WingsResponsiveVisibility::only(&[])
        } else {
            WingsResponsiveVisibility::below(WidthClass::Medium)
        };
        commands.entity(entity).insert(visibility);
        if skin.ios() {
            commands
                .entity(entity)
                .insert(WingsGlass::new(WingsGlassStyle::Regular));
            node.margin = UiRect {
                left: px(21),
                right: px(21),
                top: px(4),
                bottom: px(12),
            };
            node.padding = UiRect::all(px(4));
            node.border = UiRect::all(px(1));
            node.height = px(62);
            shape.set_if_neq(WingsShape(ShapeRole::Full));
            bg.set_if_neq(WingsBackground(role(SurfaceContainerLowest)));
            border.set_if_neq(WingsBorderPaint(role(OutlineVariant)));
            elevation.set_if_neq(WingsElevationLevel(3));
        } else {
            commands.entity(entity).remove::<WingsGlass>();
            node.height = px(80);
            node.margin = UiRect::ZERO;
            node.padding = UiRect::ZERO;
            node.border = UiRect::ZERO;
            shape.set_if_neq(WingsShape(ShapeRole::None));
            bg.set_if_neq(WingsBackground(role(SurfaceContainer)));
            border.set_if_neq(WingsBorderPaint(T));
            elevation.set_if_neq(WingsElevationLevel(2));
        }
    }
}

/// Skins navigation rails: a Material rail, an iPad or Mac sidebar, or a Fluent navigation
/// pane.
pub fn skin_navigation_rails(
    mut commands: Commands,
    skin: Res<WingsSkin>,
    added: Query<Entity, Added<WingsNavigationRail>>,
    all: Query<Entity, With<WingsNavigationRail>>,
    mut rails: Query<
        (
            &mut Node,
            &mut WingsBackground,
            &mut WingsShape,
            &mut WingsElevationLevel,
        ),
        With<WingsNavigationRail>,
    >,
) {
    for entity in skin_targets(&skin, added, all) {
        let Ok((mut node, mut bg, mut shape, mut elevation)) = rails.get_mut(entity) else {
            continue;
        };
        let visibility = if skin.design.is_desktop() {
            WingsResponsiveVisibility::from(WidthClass::Compact)
        } else {
            WingsResponsiveVisibility::from(WidthClass::Medium)
        };
        commands.entity(entity).insert(visibility);
        match skin.design {
            WingsDesign::Material => {
                commands.entity(entity).remove::<WingsGlass>();
                node.width = px(80);
                node.padding = UiRect::vertical(px(12));
                node.margin = UiRect::ZERO;
                node.row_gap = px(12);
                node.align_items = AlignItems::Center;
                bg.set_if_neq(WingsBackground(role(Surface)));
                shape.set_if_neq(WingsShape(ShapeRole::None));
                elevation.set_if_neq(WingsElevationLevel(0));
            }
            WingsDesign::Ios | WingsDesign::MacOs => {
                if skin.macos() {
                    commands
                        .entity(entity)
                        .insert(WingsGlass::new(WingsGlassStyle::Regular));
                } else {
                    commands.entity(entity).remove::<WingsGlass>();
                }
                node.width = px(sidebar_width(skin.design, false));
                node.padding = UiRect::all(px(10));
                node.margin = UiRect::all(px(8));
                node.row_gap = px(2);
                node.align_items = AlignItems::Stretch;
                bg.set_if_neq(WingsBackground(role(SurfaceVariant)));
                shape.set_if_neq(WingsShape(ShapeRole::Large));
                elevation.set_if_neq(WingsElevationLevel(1));
            }
            WingsDesign::Fluent => {
                commands.entity(entity).remove::<WingsGlass>();
                node.width = px(sidebar_width(skin.design, false));
                node.padding = UiRect::axes(px(4), px(8));
                node.margin = UiRect::ZERO;
                node.row_gap = px(4);
                node.align_items = AlignItems::Stretch;
                bg.set_if_neq(WingsBackground(role(Background)));
                shape.set_if_neq(WingsShape(ShapeRole::None));
                elevation.set_if_neq(WingsElevationLevel(0));
            }
        }
    }
}

/// Width of a sidebar or navigation pane, full or collapsed to its icons.
pub fn sidebar_width(design: WingsDesign, compact: bool) -> f32 {
    match (design, compact) {
        (WingsDesign::Material, _) => 80.0,
        (WingsDesign::Ios, _) => 260.0,
        (WingsDesign::MacOs, false) => 220.0,
        (WingsDesign::MacOs, true) => 60.0,
        (WingsDesign::Fluent, false) => 240.0,
        (WingsDesign::Fluent, true) => 48.0,
    }
}

/// Narrow windows collapse the desktop sidebars (macOS, Fluent) to their icons, like Fluent's
/// compact navigation pane, so the content keeps its room.
pub fn compact_sidebars(
    skin: Res<WingsSkin>,
    viewport: Res<WingsViewport>,
    mut last: Local<Option<(WingsDesign, bool)>>,
    added: Query<(), Added<WingsNavigationRail>>,
    mut rails: Query<(Entity, &mut Node), With<WingsNavigationRail>>,
    children: Query<&Children>,
    mut labels: Query<&mut Node, (With<WingsNavigationLabel>, Without<WingsNavigationRail>)>,
) {
    let compact = skin.design.is_desktop() && viewport.width_class == WidthClass::Compact;
    let state = Some((skin.design, compact));
    if *last == state && added.is_empty() && !skin.is_changed() {
        return;
    }
    let was_compact = last.is_some_and(|(_, c)| c);
    *last = state;
    if !compact && !was_compact {
        return;
    }
    for (rail, mut node) in &mut rails {
        node.width = px(sidebar_width(skin.design, compact));
        for d in children.iter_descendants(rail) {
            if let Ok(mut label) = labels.get_mut(d) {
                let display = if compact {
                    Display::None
                } else {
                    Display::Flex
                };
                if label.display != display {
                    label.display = display;
                }
            }
        }
    }
}

/// Where a navigation item lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ItemPlace {
    Bar,
    Rail,
}

type ItemParts<'a> = (&'a mut Node, &'a mut WingsShape, &'a mut WingsCheckedPart);

/// Skins navigation items according to their container (bar or rail) and the design.
pub fn skin_navigation_items(
    mut commands: Commands,
    skin: Res<WingsSkin>,
    added: Query<Entity, Added<WingsNavigationItem>>,
    all: Query<Entity, With<WingsNavigationItem>>,
    parents: Query<&ChildOf>,
    bar_parents: Query<(), With<WingsNavigationBar>>,
    children: Query<&Children>,
    skin_parts: Query<(), With<WingsSkinPart>>,
    mut items: Query<ItemParts, (With<WingsNavigationItem>, Without<WingsNavigationPill>)>,
    mut pills: Query<
        (
            Entity,
            &mut Node,
            &mut WingsCheckedPart,
            Has<WingsInteractive>,
        ),
        (With<WingsNavigationPill>, Without<WingsNavigationItem>),
    >,
    mut labels: Query<
        &mut WingsTypeStyle,
        (With<WingsNavigationLabel>, Without<WingsNavigationItem>),
    >,
    mut icons: Query<
        &mut Node,
        (
            With<WingsIconTint>,
            Without<WingsNavigationItem>,
            Without<WingsNavigationPill>,
        ),
    >,
) {
    for entity in skin_targets(&skin, added, all) {
        let place = match parents.get(entity) {
            Ok(p) if bar_parents.contains(p.parent()) => ItemPlace::Bar,
            _ => ItemPlace::Rail,
        };
        let Ok((mut node, mut shape, mut part)) = items.get_mut(entity) else {
            continue;
        };
        clear_skin_parts(&mut commands, entity, &children, &skin_parts);
        let material = skin.design == WingsDesign::Material;
        let row = !material && place == ItemPlace::Rail;
        // Root: column (bars, Material) or row (sidebars, panes).
        node.flex_direction = if row {
            FlexDirection::Row
        } else {
            FlexDirection::Column
        };
        node.justify_content = if row {
            JustifyContent::Start
        } else {
            JustifyContent::Center
        };
        node.align_items = AlignItems::Center;
        let (height, pad, gap, item_shape) = match (skin.design, place) {
            (WingsDesign::Material, _) => (Val::Auto, UiRect::ZERO, 4.0, ShapeRole::Full),
            (_, ItemPlace::Bar) => (px(54), UiRect::axes(px(10), px(4)), 1.0, ShapeRole::Full),
            (WingsDesign::Ios, ItemPlace::Rail) => {
                (px(44), UiRect::horizontal(px(12)), 10.0, ShapeRole::Medium)
            }
            (WingsDesign::MacOs, ItemPlace::Rail) => {
                (px(32), UiRect::horizontal(px(10)), 8.0, ShapeRole::Small)
            }
            (WingsDesign::Fluent, ItemPlace::Rail) => {
                (px(36), UiRect::horizontal(px(12)), 12.0, ShapeRole::Small)
            }
        };
        node.height = height;
        node.padding = pad;
        node.row_gap = px(gap);
        node.column_gap = px(gap);
        node.min_width = px(if material {
            56.0
        } else if row {
            0.0
        } else {
            72.0
        });
        shape.set_if_neq(WingsShape(item_shape));
        let (selected_fill, selected_content, content) = match (skin.design, place) {
            (WingsDesign::Material, _) => (T, OnSurface, OnSurfaceVariant),
            (WingsDesign::Ios, ItemPlace::Rail) => {
                (role(PrimaryContainer), OnPrimaryContainer, OnSurface)
            }
            (WingsDesign::Ios, ItemPlace::Bar) => (role(SecondaryFill), Primary, OnSurface),
            _ => (role(SecondaryFill), OnSurface, OnSurface),
        };
        part.set_if_neq(WingsCheckedPart {
            background: Some((selected_fill, T)),
            content: Some((role(selected_content), role(content))),
            ..default()
        });
        for d in children.iter_descendants(entity) {
            if let Ok((pill, mut pill_node, mut pill_part, interactive)) = pills.get_mut(d) {
                if material {
                    pill_node.width = px(64);
                    pill_node.height = px(32);
                    pill_part.set_if_neq(WingsCheckedPart {
                        background: Some((role(SecondaryContainer), T)),
                        content: Some((role(OnSecondaryContainer), role(OnSurfaceVariant))),
                        ..default()
                    });
                    if !interactive {
                        commands.entity(pill).insert(WingsInteractive);
                        commands.entity(entity).remove::<WingsInteractive>();
                    }
                } else {
                    pill_node.width = Val::Auto;
                    pill_node.height = Val::Auto;
                    let icon_color = match (skin.design, place) {
                        (WingsDesign::MacOs, ItemPlace::Rail) => Primary,
                        (WingsDesign::Ios, ItemPlace::Bar) => selected_content,
                        _ => selected_content,
                    };
                    pill_part.set_if_neq(WingsCheckedPart {
                        background: Some((T, T)),
                        content: Some((
                            role(icon_color),
                            role(if skin.macos() { Primary } else { content }),
                        )),
                        ..default()
                    });
                    if interactive {
                        commands.entity(pill).remove::<WingsInteractive>();
                        commands.entity(entity).insert(WingsInteractive);
                    }
                }
            } else if let Ok(mut label) = labels.get_mut(d) {
                let (r, adjust) = match (skin.design, place) {
                    (WingsDesign::Material, _) => (TypeRole::LabelMedium, None),
                    (_, ItemPlace::Bar) => (
                        TypeRole::LabelSmall,
                        Some(WingsTypeOverride {
                            size: Some(10.0),
                            weight: Some(bevy::text::FontWeight::SEMIBOLD),
                        }),
                    ),
                    _ => (TypeRole::BodyLarge, None),
                };
                label.set_if_neq(WingsTypeStyle(r));
                match adjust {
                    Some(a) => {
                        commands.entity(d).insert(a);
                    }
                    None => {
                        commands.entity(d).remove::<WingsTypeOverride>();
                    }
                }
            } else if let Ok(mut icon) = icons.get_mut(d) {
                let size = match (skin.design, place) {
                    (WingsDesign::Material, _) => 24.0,
                    (WingsDesign::Ios, ItemPlace::Bar) => 26.0,
                    (WingsDesign::Ios, ItemPlace::Rail) => 22.0,
                    _ => 16.0,
                };
                icon.width = px(size);
                icon.height = px(size);
            }
        }
        if skin.fluent() && place == ItemPlace::Rail {
            // Fluent marks the selected pane item with a short accent bar.
            commands.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(0),
                    top: px(10),
                    width: px(3),
                    height: px(16),
                    ..default()
                },
                WingsShape(ShapeRole::Full),
                WingsBackground(role(Primary)),
                WingsCheckedPart {
                    visible_when_checked: true,
                    ..default()
                },
                Pickable::IGNORE,
                WingsSkinPart,
                ChildOf(entity),
            ));
        }
        commands.entity(entity).insert(WingsCheckedRefresh);
    }
}

// ---------------------------------------------------------------------------------------------
// List items
// ---------------------------------------------------------------------------------------------

/// List row metrics of a design.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WingsListItemMetrics {
    /// Minimum row height.
    pub min_height: f32,
    /// Horizontal and vertical padding.
    pub padding: Vec2,
    /// Gap between icon, texts and accessory.
    pub gap: f32,
    /// Leading icon size.
    pub icon: f32,
    /// Supporting text role.
    pub supporting: TypeRole,
}

impl WingsListItemMetrics {
    /// Metrics of a design.
    pub fn for_design(design: WingsDesign) -> Self {
        match design {
            WingsDesign::Material => Self {
                min_height: 56.0,
                padding: Vec2::new(16.0, 8.0),
                gap: 16.0,
                icon: 24.0,
                supporting: TypeRole::BodyMedium,
            },
            WingsDesign::Ios => Self {
                min_height: 52.0,
                padding: Vec2::new(16.0, 10.0),
                gap: 12.0,
                icon: 22.0,
                supporting: TypeRole::BodySmall,
            },
            WingsDesign::MacOs => Self {
                min_height: 36.0,
                padding: Vec2::new(10.0, 6.0),
                gap: 8.0,
                icon: 16.0,
                supporting: TypeRole::BodySmall,
            },
            WingsDesign::Fluent => Self {
                min_height: 40.0,
                padding: Vec2::new(12.0, 8.0),
                gap: 12.0,
                icon: 16.0,
                supporting: TypeRole::BodySmall,
            },
        }
    }
}

/// Skins list rows.
pub fn skin_list_items(
    mut commands: Commands,
    skin: Res<WingsSkin>,
    added: Query<Entity, Added<WingsListItem>>,
    all: Query<Entity, With<WingsListItem>>,
    children: Query<&Children>,
    mut rows: Query<&mut Node, With<WingsListItem>>,
    mut icons: Query<&mut Node, (With<WingsListItemIcon>, Without<WingsListItem>)>,
    mut texts: Query<
        (
            &mut WingsTypeStyle,
            Has<WingsListItemSupporting>,
            Has<WingsListItemDetail>,
        ),
        Or<(
            With<WingsListItemHeadline>,
            With<WingsListItemSupporting>,
            With<WingsListItemDetail>,
        )>,
    >,
    tinted: Query<(), With<WingsListItemIcon>>,
) {
    let m = WingsListItemMetrics::for_design(skin.design);
    let feedback = match skin.design {
        WingsDesign::Material => WingsPressFeedback::MATERIAL,
        WingsDesign::Fluent => WingsPressFeedback::FLUENT,
        _ => WingsPressFeedback::HIGHLIGHT,
    };
    for entity in skin_targets(&skin, added, all) {
        if let Ok(mut node) = rows.get_mut(entity) {
            node.min_height = px(m.min_height);
            node.padding = UiRect::axes(px(m.padding.x), px(m.padding.y));
            node.column_gap = px(m.gap);
        }
        commands.entity(entity).insert(feedback);
        for d in children.iter_descendants(entity) {
            if let Ok(mut icon) = icons.get_mut(d) {
                icon.width = px(m.icon);
                icon.height = px(m.icon);
                if tinted.contains(d) {
                    // Apple and Fluent tint list icons with the accent.
                    if skin.design == WingsDesign::Material {
                        commands.entity(d).remove::<WingsContentColor>();
                    } else {
                        commands.entity(d).insert(WingsContentColor(role(Primary)));
                    }
                }
            } else if let Ok((mut role_style, supporting, _detail)) = texts.get_mut(d) {
                // Detail text uses the headline size, like the headline.
                let r = if supporting {
                    m.supporting
                } else {
                    TypeRole::BodyLarge
                };
                role_style.set_if_neq(WingsTypeStyle(r));
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Screen background
// ---------------------------------------------------------------------------------------------

/// Gives a scaffold the grouped background while a grouped list is on screen in the Apple and
/// Fluent designs, so the gray reaches behind the transparent bars like a SwiftUI `List`.
pub fn scaffold_background_system(
    skin: Res<WingsSkin>,
    mut scaffolds: Query<(Entity, &mut WingsBackground), With<WingsScaffold>>,
    children: Query<&Children>,
    lists: Query<(&crate::list::WingsListStyle, &InheritedVisibility, &Node)>,
) {
    for (scaffold, mut bg) in &mut scaffolds {
        let grouped = !matches!(skin.design, WingsDesign::Material)
            && children.iter_descendants(scaffold).any(|d| {
                lists.get(d).is_ok_and(|(style, visible, node)| {
                    visible.get()
                        && node.display != Display::None
                        && style.resolve(skin.design) == crate::list::WingsListStyle::Grouped
                })
            });
        let wanted = WingsBackground(role(if grouped {
            GroupedBackground
        } else {
            Background
        }));
        bg.set_if_neq(wanted);
    }
}

// ---------------------------------------------------------------------------------------------
// Safe area
// ---------------------------------------------------------------------------------------------

/// Fits scaffolds to the safe area, like SwiftUI: bars extend under system UI while their
/// content, and everything between them, stays clear of it.
///
/// * The top bar grows under the status bar (see [`layout_top_app_bars`]).
/// * A bottom navigation bar sits above the home indicator or Android navigation bar; the
///   iOS tab bar floats just above it. Bottom bars hide while the on-screen keyboard is up.
/// * The scaffold pads its content for the left and right insets (landscape notches), for the
///   top when it has no top bar and for the bottom (or the keyboard) when no bottom bar shows.
/// * Snackbars stay above the bottom chrome and the keyboard.
pub fn scaffold_safe_area_system(
    mut commands: Commands,
    skin: Res<WingsSkin>,
    safe: Res<WingsSafeArea>,
    roots: Query<(Entity, &WingsOverlayRoot)>,
    scaffolds: Query<(Entity, Option<&WingsSafeAreaPadding>), With<WingsScaffold>>,
    children: Query<&Children>,
    top_bars: Query<(), With<WingsTopAppBar>>,
    mut bottom_bars: Query<
        (&mut Node, &mut WingsResponsiveVisibility),
        (With<WingsNavigationBar>, Without<WingsSnackbarHost>),
    >,
    mut hosts: Query<&mut Node, (With<WingsSnackbarHost>, Without<WingsNavigationBar>)>,
    added: Query<(), Added<WingsScaffold>>,
    mut last_bar_visible: Local<bevy::platform::collections::HashMap<Entity, bool>>,
) {
    let keyboard = safe.keyboard_visible();
    for (scaffold, padding) in &scaffolds {
        let mut has_top_bar = false;
        let mut bottom_visible = false;
        let tree = scaffold_tree(scaffold, &children, &roots);
        for d in tree.iter().copied() {
            if top_bars.contains(d) {
                has_top_bar = true;
            }
            if let Ok((mut node, mut visibility)) = bottom_bars.get_mut(d) {
                let wants = visibility.visible_on.iter().any(|v| *v);
                if keyboard && node.display != Display::None {
                    node.display = Display::None;
                } else if !keyboard && node.display == Display::None && wants {
                    // Let the responsive rules decide again.
                    visibility.set_changed();
                }
                bottom_visible |= node.display != Display::None && !keyboard;
                // Bottom inset: Material bars grow under the system bar, the iOS tab bar
                // floats just above the home indicator.
                let inset = safe.insets.bottom;
                if skin.ios() {
                    let bottom = px((inset - 12.0).max(12.0));
                    if node.margin.bottom != bottom {
                        node.margin.bottom = bottom;
                    }
                } else if !skin.design.is_desktop() {
                    let height = px(80.0 + inset);
                    if node.height != height {
                        node.height = height;
                        node.padding.bottom = px(inset);
                    }
                }
            }
        }
        let previous = last_bar_visible.insert(scaffold, bottom_visible);
        let edges = WingsEdges {
            top: !has_top_bar,
            bottom: !bottom_visible,
            left: true,
            right: true,
        };
        match padding {
            Some(p) if p.edges == edges => {}
            Some(p) => {
                let mut p = *p;
                p.edges = edges;
                commands.entity(scaffold).insert(p);
            }
            None => {
                commands
                    .entity(scaffold)
                    .insert(WingsSafeAreaPadding::edges(edges));
            }
        }
        if safe.is_changed()
            || skin.is_changed()
            || previous != Some(bottom_visible)
            || added.contains(scaffold)
        {
            let base = pick(skin.design, 88.0, 100.0, 24.0, 24.0);
            let bottom = if bottom_visible {
                base + safe.insets.bottom
            } else {
                24.0 + safe.content().bottom
            };
            for d in tree.iter().copied() {
                if let Ok(mut host) = hosts.get_mut(d) {
                    host.bottom = px(bottom);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Window chrome
// ---------------------------------------------------------------------------------------------

/// Width the macOS window buttons (close, minimize, zoom) need at the leading edge of a
/// transparent title bar.
pub const MAC_TRAFFIC_LIGHTS: f32 = 80.0;

/// Fits scaffolds to the window chrome.
///
/// * With a transparent, full size content title bar on macOS the toolbar leaves room for the
///   window buttons.
/// * In the macOS design a visible sidebar reaches up under the title bar and the toolbar
///   starts after it, like a SwiftUI `NavigationSplitView`.
pub fn window_chrome_system(
    skin: Res<WingsSkin>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    scaffolds: Query<Entity, With<WingsScaffold>>,
    children: Query<&Children>,
    roots: Query<(Entity, &WingsOverlayRoot)>,
    floating: Query<(), With<WingsFloating>>,
    mut bars: Query<&mut Node, (With<WingsTopAppBar>, Without<WingsNavigationRail>)>,
    mut rails: Query<&mut Node, (With<WingsNavigationRail>, Without<WingsTopAppBar>)>,
) {
    let transparent_titlebar = cfg!(target_os = "macos")
        && windows
            .single()
            .is_ok_and(|w| w.titlebar_transparent && w.fullsize_content_view);
    let traffic = if transparent_titlebar {
        MAC_TRAFFIC_LIGHTS
    } else {
        0.0
    };
    for scaffold in &scaffolds {
        let mut bar = None;
        let mut rail = None;
        for d in scaffold_tree(scaffold, &children, &roots) {
            if bar.is_none() && bars.contains(d) {
                bar = Some(d);
            }
            if rail.is_none() && rails.contains(d) {
                rail = Some(d);
            }
        }
        let Some(bar) = bar else { continue };
        let Ok(mut bar_node) = bars.get_mut(bar) else {
            continue;
        };
        let bar_height = match bar_node.height {
            Val::Px(h) => h,
            _ => 52.0,
        };
        let design_pad = pick(skin.design, 8.0, 8.0, 12.0, 12.0);
        let rail_floats = rail.is_some_and(|r| floating.contains(r));
        let rail_node = rail.and_then(|r| rails.get_mut(r).ok());
        let sidebar = match rail_node {
            Some(mut rail_node)
                if skin.macos() && rail_floats && rail_node.display != Display::None =>
            {
                // A floating sidebar reaches the top of the window, inset like Tahoe's.
                let width = match rail_node.width {
                    Val::Px(w) => w,
                    _ => 220.0,
                };
                if rail_node.margin.top != px(8)
                    || rail_node.padding.top != px(bar_height - 8.0 + 2.0)
                {
                    rail_node.margin = UiRect::all(px(8));
                    rail_node.padding.top = px(bar_height - 8.0 + 2.0);
                }
                Some(width + 16.0)
            }
            Some(mut rail_node) if skin.macos() && rail_node.display != Display::None => {
                let width = match rail_node.width {
                    Val::Px(w) => w,
                    _ => 220.0,
                };
                let top = px(8.0 - bar_height);
                if rail_node.margin.top != top {
                    rail_node.margin.top = top;
                    // Items start below the window buttons.
                    rail_node.padding.top = px(bar_height - 8.0 + 2.0);
                }
                Some(width + 16.0)
            }
            Some(mut rail_node) if skin.macos() => {
                if rail_node.margin.top != px(8) {
                    rail_node.margin.top = px(8);
                    rail_node.padding.top = px(10);
                }
                None
            }
            _ => None,
        };
        let left = px(match sidebar {
            Some(w) => w + design_pad,
            None => design_pad.max(traffic),
        });
        if bar_node.padding.left != left {
            bar_node.padding.left = left;
        }
    }
}
