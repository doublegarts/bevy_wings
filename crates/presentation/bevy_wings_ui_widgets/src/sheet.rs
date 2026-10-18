//! Sheets: [`WingsBottomSheet`] and [`WingsSideSheet`].
//!
//! A sheet is a panel that slides in over the content with a dimming scrim behind it. Show or
//! hide it by setting [`WingsAnimatedVisibility::visible`]; clicking the scrim or pressing
//! Escape hides it and triggers [`WingsSheetDismissed`]. Children of the sheet are its
//! content.
//!
//! | Design | Bottom sheet | Side sheet |
//! | --- | --- | --- |
//! | Material | 28 corners, drag handle | 16 corners on the leading edge |
//! | iOS | 38 corners, grabber, medium or large detent | inspector panel |
//! | macOS | centered sheet panel | floating inspector |
//! | Fluent | 8 corners with a border | pane with a border |
//!
//! ```ignore
//! bsn! {
//!     @WingsBottomSheet { @title: "Share" }
//!     Children [ @WingsText { @text: "Pick an app" } ]
//! }
//! ```

use bevy::prelude::*;

use bevy_wings_ui_animation::prelude::{WingsAnimatedVisibility, WingsTransition};
use bevy_wings_ui_responsive::WingsViewport;
use bevy_wings_ui_theme::prelude::{
    ColorRole, ShapeRole, TypeRole, WingsBackground, WingsBorderPaint, WingsContentColor,
    WingsDesign, WingsElevationLevel, WingsPaint, WingsShape,
};

use crate::skin::{WingsSkin, WingsSkinPart, clear_skin_parts, skin_targets};
use crate::text::WingsText;

/// How tall a bottom sheet is.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WingsSheetDetent {
    /// Half the screen.
    #[default]
    Medium,
    /// Nearly the whole screen.
    Large,
    /// As tall as its content.
    Fit,
}

/// Which edge a sheet comes from.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum WingsSheetEdge {
    /// From the bottom.
    Bottom,
    /// From the trailing side.
    Side,
}

/// A sheet sliding up from the bottom.
#[derive(SceneComponent, Default, Clone)]
#[scene(WingsSheetProps)]
pub struct WingsBottomSheet;

/// A sheet sliding in from the trailing side.
#[derive(SceneComponent, Default, Clone)]
#[scene(WingsSheetProps)]
pub struct WingsSideSheet;

/// Props of the sheets.
#[derive(Clone, Default)]
pub struct WingsSheetProps {
    /// Optional title.
    pub title: String,
    /// Start visible.
    pub open: bool,
    /// Height of a bottom sheet.
    pub detent: WingsSheetDetent,
}

/// Triggered on a sheet when the user dismissed it.
#[derive(EntityEvent, Debug, Clone, Copy)]
pub struct WingsSheetDismissed {
    /// The sheet.
    pub entity: Entity,
}

/// The scrim behind an open sheet.
#[derive(Component, Debug, Clone, Copy)]
pub struct WingsSheetScrim {
    /// The sheet it belongs to.
    pub sheet: Entity,
}

fn sheet_scene(props: WingsSheetProps, edge: WingsSheetEdge) -> impl Scene {
    let visibility = WingsAnimatedVisibility {
        visible: props.open,
        enter: match edge {
            WingsSheetEdge::Bottom => WingsTransition::slide_vertically(420.0),
            WingsSheetEdge::Side => WingsTransition::slide_horizontally(380.0),
        },
        exit: match edge {
            WingsSheetEdge::Bottom => WingsTransition::slide_vertically(420.0),
            WingsSheetEdge::Side => WingsTransition::slide_horizontally(380.0),
        },
    };
    let detent = props.detent;
    let title: Box<dyn Scene> = if props.title.is_empty() {
        Box::new(bsn! {})
    } else {
        let text = props.title;
        Box::new(bsn! {
            Children [ @WingsText { @text: text, @role: TypeRole::TitleLarge } ]
        })
    };
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            flex_direction: FlexDirection::Column,
            row_gap: px(12),
        }
        edge
        detent
        visibility
        GlobalZIndex(150)
        WingsBackground
        WingsBorderPaint
        WingsShape
        WingsElevationLevel(3)
        WingsContentColor({WingsPaint::Role(ColorRole::OnSurface)})
        @{title}
    }
}

impl WingsBottomSheet {
    fn scene(props: WingsSheetProps) -> impl Scene {
        sheet_scene(props, WingsSheetEdge::Bottom)
    }
}

impl WingsSideSheet {
    fn scene(props: WingsSheetProps) -> impl Scene {
        sheet_scene(props, WingsSheetEdge::Side)
    }
}

/// Skins sheets: geometry, corners, fill and the grabber per design.
pub fn skin_sheets(
    mut commands: Commands,
    skin: Res<WingsSkin>,
    viewport: Res<WingsViewport>,
    added: Query<Entity, Added<WingsSheetEdge>>,
    all: Query<Entity, With<WingsSheetEdge>>,
    children: Query<&Children>,
    parts: Query<(), With<WingsSkinPart>>,
    mut sheets: Query<(
        &WingsSheetEdge,
        &WingsSheetDetent,
        &mut Node,
        &mut WingsBackground,
        &mut WingsBorderPaint,
        &mut WingsShape,
    )>,
    theme: Res<bevy_wings_ui_theme::prelude::WingsTheme>,
) {
    use ColorRole::*;
    let resized = viewport.is_changed();
    let targets = if resized {
        all.iter().collect()
    } else {
        skin_targets(&skin, added, all)
    };
    for entity in targets {
        let Ok((edge, detent, mut node, mut bg, mut border, mut shape)) = sheets.get_mut(entity)
        else {
            continue;
        };
        let r = WingsPaint::Role;
        let (fill, line, corners) = match skin.design {
            WingsDesign::Material => (
                r(SurfaceContainerLow),
                WingsPaint::Transparent,
                ShapeRole::ExtraLarge,
            ),
            WingsDesign::Ios => (
                r(SurfaceContainerLowest),
                WingsPaint::Transparent,
                ShapeRole::ExtraLarge,
            ),
            WingsDesign::MacOs => (
                r(SurfaceContainerLowest),
                r(OutlineVariant),
                ShapeRole::Large,
            ),
            WingsDesign::Fluent => (
                r(SurfaceContainerLowest),
                r(OutlineVariant),
                ShapeRole::Large,
            ),
        };
        bg.set_if_neq(WingsBackground(fill));
        border.set_if_neq(WingsBorderPaint(line));
        shape.set_if_neq(WingsShape(ShapeRole::None));
        let radius = theme.shapes.px(corners).min(40.0);
        node.border = UiRect::all(px(if line == WingsPaint::Transparent {
            0.0
        } else {
            1.0
        }));
        node.padding = UiRect::all(px(if skin.design == WingsDesign::MacOs {
            20.0
        } else {
            24.0
        }));
        match (edge, skin.design) {
            (WingsSheetEdge::Bottom, WingsDesign::MacOs) => {
                // macOS sheets are centered panels.
                let width = 480.0_f32.min(viewport.width - 32.0);
                node.left = px((viewport.width - width) * 0.5);
                node.right = Val::Auto;
                node.width = px(width);
                node.top = px(60);
                node.bottom = Val::Auto;
                node.max_height = percent(80);
                node.border_radius = BorderRadius::all(px(radius));
            }
            (WingsSheetEdge::Bottom, _) => {
                let max = 640.0_f32;
                let width = viewport.width.min(max);
                node.left = px((viewport.width - width) * 0.5);
                node.right = Val::Auto;
                node.width = px(width);
                node.bottom = px(0);
                node.top = Val::Auto;
                node.height = match detent {
                    WingsSheetDetent::Medium => percent(50),
                    WingsSheetDetent::Large => percent(92),
                    WingsSheetDetent::Fit => Val::Auto,
                };
                node.border_radius = BorderRadius::top(px(radius));
            }
            (WingsSheetEdge::Side, _) => {
                let inset = if skin.design.is_apple() { 8.0 } else { 0.0 };
                node.right = px(inset);
                node.left = Val::Auto;
                node.top = px(inset);
                node.bottom = px(inset);
                node.width = px(360.0_f32.min(viewport.width));
                node.height = Val::Auto;
                node.border_radius = if skin.design.is_apple() {
                    BorderRadius::all(px(radius))
                } else {
                    BorderRadius::left(px(radius))
                };
            }
        }
        // Grabber on Material and iOS bottom sheets.
        clear_skin_parts(&mut commands, entity, &children, &parts);
        if *edge == WingsSheetEdge::Bottom
            && matches!(skin.design, WingsDesign::Material | WingsDesign::Ios)
        {
            let (w, h) = if skin.ios() { (36.0, 5.0) } else { (32.0, 4.0) };
            let grabber = commands
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        top: px(if skin.ios() { 5.0 } else { 16.0 }),
                        left: percent(50),
                        margin: UiRect::left(px(-w * 0.5)),
                        width: px(w),
                        height: px(h),
                        ..default()
                    },
                    WingsShape(ShapeRole::Full),
                    WingsBackground(WingsPaint::RoleAlpha(OnSurfaceVariant, 0.4)),
                    Pickable::IGNORE,
                    WingsSkinPart,
                ))
                .id();
            commands.entity(entity).insert_children(0, &[grabber]);
        }
    }
}

/// Shows a scrim behind open sheets and removes it when they close.
pub fn sheet_scrims(
    mut commands: Commands,
    skin: Res<WingsSkin>,
    sheets: Query<
        (Entity, &WingsAnimatedVisibility, Option<&ChildOf>),
        (With<WingsSheetEdge>, Changed<WingsAnimatedVisibility>),
    >,
    scrims: Query<(Entity, &WingsSheetScrim)>,
) {
    for (sheet, visibility, parent) in &sheets {
        let existing = scrims
            .iter()
            .find(|(_, s)| s.sheet == sheet)
            .map(|(e, _)| e);
        match (visibility.visible, existing) {
            (true, None) => {
                let alpha = match skin.design {
                    WingsDesign::Material => 0.32,
                    WingsDesign::Fluent => 0.3,
                    WingsDesign::Ios => 0.2,
                    WingsDesign::MacOs => 0.1,
                };
                let scrim = commands
                    .spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: px(0),
                            right: px(0),
                            top: px(0),
                            bottom: px(0),
                            ..default()
                        },
                        GlobalZIndex(149),
                        WingsBackground(WingsPaint::Role(ColorRole::Scrim).with_alpha(alpha)),
                        WingsAnimatedVisibility {
                            visible: true,
                            enter: WingsTransition::fade(),
                            exit: WingsTransition::fade(),
                        },
                        WingsSheetScrim { sheet },
                    ))
                    .observe(dismiss_sheet_on_scrim)
                    .id();
                if let Some(parent) = parent {
                    commands.entity(scrim).insert(ChildOf(parent.parent()));
                }
            }
            (false, Some(scrim)) => {
                commands.entity(scrim).despawn();
            }
            _ => {}
        }
    }
}

/// Observer: clicking the scrim dismisses its sheet.
pub fn dismiss_sheet_on_scrim(
    ev: On<bevy::picking::events::PointerPress>,
    scrims: Query<&WingsSheetScrim>,
    mut sheets: Query<&mut WingsAnimatedVisibility, With<WingsSheetEdge>>,
    mut commands: Commands,
) {
    if let Ok(scrim) = scrims.get(ev.event_target())
        && let Ok(mut visibility) = sheets.get_mut(scrim.sheet)
    {
        visibility.visible = false;
        commands.trigger(WingsSheetDismissed {
            entity: scrim.sheet,
        });
    }
}

/// Escape dismisses open sheets.
pub fn dismiss_sheets_on_escape(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    mut sheets: Query<(Entity, &mut WingsAnimatedVisibility), With<WingsSheetEdge>>,
    mut commands: Commands,
) {
    if !keys.is_some_and(|k| k.just_pressed(KeyCode::Escape)) {
        return;
    }
    for (entity, mut visibility) in &mut sheets {
        if visibility.visible {
            visibility.visible = false;
            commands.trigger(WingsSheetDismissed { entity });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The default bottom sheet is medium height.
    #[test]
    fn default_detent() {
        assert_eq!(WingsSheetDetent::default(), WingsSheetDetent::Medium);
    }
}
