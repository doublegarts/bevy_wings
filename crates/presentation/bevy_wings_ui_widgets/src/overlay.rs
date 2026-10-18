//! The overlay layer: floating chrome above the content, like SwiftUI's navigation layer.
//!
//! In the Liquid Glass designs bars float over the content, which scrolls underneath them:
//!
//! * iOS: the navigation bar, the tab bar and toasts,
//! * macOS: the toolbar, the sidebar and toasts.
//!
//! Wings moves this chrome out of the scaffold into an overlay root drawn by a second camera
//! (the overlay camera). Before that camera draws its UI, the shaders crate captures the
//! window into a blurred backdrop, so [`WingsGlass`](crate::glass::WingsGlass) on floating
//! chrome shows the real content behind it. Each moved entity remembers where it came from
//! ([`WingsFloating`]) and goes back when the design no longer floats it.
//!
//! The scaffold then pads its content by the floating chrome ([`WingsChromeInsets`]); scroll
//! columns that fill the content area reach back under the chrome so their content can scroll
//! beneath it while starting below it.

use bevy::camera::visibility::RenderLayers;
use bevy::camera::{Camera, Camera2d, ClearColorConfig, RenderTarget};
use bevy::prelude::*;
use bevy::ui::{IsDefaultUiCamera, UiTargetCamera};

use bevy_wings_ui_theme::prelude::WingsDesign;

use crate::feedback::WingsSnackbarHost;
use crate::layout::WingsScrollColumn;
use crate::scaffold::{WingsNavigationBar, WingsNavigationRail, WingsScaffold, WingsTopAppBar};
use crate::skin::WingsSkin;

/// Render layer the overlay camera uses so it draws no world content, only its UI.
pub const OVERLAY_RENDER_LAYER: usize = 31;

/// Marks the Wings overlay camera.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct WingsOverlayCamera;

/// The overlay root of a scaffold.
#[derive(Component, Debug, Clone, Copy)]
pub struct WingsOverlayRoot {
    /// The scaffold whose chrome floats here.
    pub scaffold: Entity,
}

/// Which edge a floating element sits on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WingsFloatEdge {
    /// A top bar.
    Top,
    /// A bottom bar.
    Bottom,
    /// A leading sidebar.
    Leading,
    /// Positioned by itself (toast hosts).
    Free,
}

/// A chrome element moved into the overlay root.
#[derive(Component, Debug, Clone, Copy)]
pub struct WingsFloating {
    /// Original parent.
    pub home: Entity,
    /// Original index among the parent's children.
    pub index: usize,
    /// Edge it floats on.
    pub edge: WingsFloatEdge,
    /// The scaffold it belongs to.
    pub scaffold: Entity,
}

/// Space the floating chrome of a scaffold covers, in logical pixels.
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct WingsChromeInsets {
    /// Height of the floating top bar.
    pub top: f32,
    /// Height of the floating bottom bar, including its margins.
    pub bottom: f32,
    /// Width of the floating sidebar, including its margins.
    pub leading: f32,
    /// Insets already applied to the scaffold padding.
    applied: Vec4,
}

/// Turns the floating layer on or off. On by default; apps that never want chrome to float
/// (or tests) can switch it off.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct WingsFloatingChrome {
    /// Float chrome in designs that do.
    pub enabled: bool,
}

impl Default for WingsFloatingChrome {
    fn default() -> Self {
        Self { enabled: true }
    }
}

/// True when `design` floats `edge` chrome.
pub fn design_floats(design: WingsDesign, edge: WingsFloatEdge) -> bool {
    match design {
        WingsDesign::Ios => matches!(
            edge,
            WingsFloatEdge::Top | WingsFloatEdge::Bottom | WingsFloatEdge::Free
        ),
        WingsDesign::MacOs => matches!(
            edge,
            WingsFloatEdge::Top | WingsFloatEdge::Leading | WingsFloatEdge::Free
        ),
        _ => false,
    }
}

/// Creates the overlay camera next to the app's UI camera when chrome should float.
pub fn ensure_overlay_camera(
    mut commands: Commands,
    skin: Res<WingsSkin>,
    settings: Res<WingsFloatingChrome>,
    overlays: Query<Entity, With<WingsOverlayCamera>>,
    cameras: Query<
        (
            Entity,
            &Camera,
            Option<&RenderTarget>,
            Has<IsDefaultUiCamera>,
        ),
        Without<WingsOverlayCamera>,
    >,
    scaffolds: Query<(), With<WingsScaffold>>,
) {
    let wanted = settings.enabled
        && !scaffolds.is_empty()
        && matches!(skin.design, WingsDesign::Ios | WingsDesign::MacOs);
    if !wanted || !overlays.is_empty() {
        return;
    }
    let Some((main, camera, target, is_default)) = cameras
        .iter()
        .filter(|(_, c, _, _)| c.is_active)
        .max_by_key(|(_, c, _, _)| c.order)
    else {
        return;
    };
    if !is_default && !cameras.iter().any(|(_, _, _, d)| d) {
        // Content UI stays on the app camera, only overlay roots use the overlay camera.
        commands.entity(main).insert(IsDefaultUiCamera);
    }
    let mut overlay = commands.spawn((
        Camera2d,
        Camera {
            order: camera.order + 1,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        RenderLayers::layer(OVERLAY_RENDER_LAYER),
        WingsOverlayCamera,
        Name::new("Wings overlay camera"),
    ));
    // Draw where the app draws: its window, another window or an image.
    if let Some(target) = target {
        overlay.insert(target.clone());
    }
    #[cfg(feature = "shaders")]
    overlay.insert(bevy_wings_ui_shaders::backdrop::WingsBackdropCamera);
}

/// Creates one overlay root per scaffold once the overlay camera exists.
pub fn ensure_overlay_roots(
    mut commands: Commands,
    cameras: Query<Entity, With<WingsOverlayCamera>>,
    scaffolds: Query<Entity, With<WingsScaffold>>,
    roots: Query<(Entity, &WingsOverlayRoot)>,
) {
    let Ok(camera) = cameras.single() else {
        return;
    };
    for scaffold in &scaffolds {
        if roots.iter().any(|(_, r)| r.scaffold == scaffold) {
            continue;
        }
        commands.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                right: px(0),
                top: px(0),
                bottom: px(0),
                ..default()
            },
            UiTargetCamera(camera),
            Pickable::IGNORE,
            WingsOverlayRoot { scaffold },
            Name::new("Wings overlay root"),
        ));
    }
    // Roots of despawned scaffolds go away.
    for (root, r) in &roots {
        if scaffolds.get(r.scaffold).is_err() {
            commands.entity(root).despawn();
        }
    }
}

type ChromeKinds<'a> = (
    Has<WingsTopAppBar>,
    Has<WingsNavigationBar>,
    Has<WingsNavigationRail>,
    Has<WingsSnackbarHost>,
);

fn edge_of(kind: (bool, bool, bool, bool)) -> Option<WingsFloatEdge> {
    match kind {
        (true, _, _, _) => Some(WingsFloatEdge::Top),
        (_, true, _, _) => Some(WingsFloatEdge::Bottom),
        (_, _, true, _) => Some(WingsFloatEdge::Leading),
        (_, _, _, true) => Some(WingsFloatEdge::Free),
        _ => None,
    }
}

/// Moves chrome into the overlay root when the design floats it, and back when it does not.
pub fn float_chrome(
    mut commands: Commands,
    skin: Res<WingsSkin>,
    settings: Res<WingsFloatingChrome>,
    roots: Query<(Entity, &WingsOverlayRoot)>,
    scaffolds: Query<Entity, With<WingsScaffold>>,
    children: Query<&Children>,
    parents: Query<&ChildOf>,
    chrome: Query<ChromeKinds>,
    floating: Query<(Entity, &WingsFloating)>,
    mut nodes: Query<&mut Node>,
) {
    // Bring back what should no longer float.
    for (entity, f) in &floating {
        if settings.enabled && design_floats(skin.design, f.edge) && scaffolds.contains(f.scaffold)
        {
            continue;
        }
        if let Ok(mut node) = nodes.get_mut(entity)
            && f.edge != WingsFloatEdge::Free
        {
            node.position_type = PositionType::Relative;
            node.left = Val::Auto;
            node.right = Val::Auto;
            node.top = Val::Auto;
            node.bottom = Val::Auto;
        }
        commands.entity(entity).remove::<WingsFloating>();
        if scaffolds.contains(f.scaffold)
            || parents.get(f.home).is_ok()
            || children.get(f.home).is_ok()
        {
            let len = children.get(f.home).map_or(0, |c| c.len());
            commands
                .entity(f.home)
                .insert_children(f.index.min(len), &[entity]);
        }
    }
    if !settings.enabled {
        return;
    }
    for (root, r) in &roots {
        let Ok(scaffold) = scaffolds.get(r.scaffold) else {
            continue;
        };
        let candidates: Vec<Entity> = children
            .iter_descendants(scaffold)
            .filter(|e| chrome.get(*e).is_ok_and(|k| edge_of(k).is_some()))
            .collect();
        for entity in candidates {
            let Ok(kind) = chrome.get(entity) else {
                continue;
            };
            let Some(edge) = edge_of(kind) else { continue };
            if !design_floats(skin.design, edge) || floating.contains(entity) {
                continue;
            }
            let Ok(parent) = parents.get(entity) else {
                continue;
            };
            let home = parent.parent();
            let index = children
                .get(home)
                .ok()
                .and_then(|c| c.iter().position(|e| e == entity))
                .unwrap_or(0);
            if let Ok(mut node) = nodes.get_mut(entity) {
                match edge {
                    WingsFloatEdge::Top => {
                        node.position_type = PositionType::Absolute;
                        node.left = px(0);
                        node.right = px(0);
                        node.top = px(0);
                    }
                    WingsFloatEdge::Bottom => {
                        node.position_type = PositionType::Absolute;
                        node.left = px(0);
                        node.right = px(0);
                        node.bottom = px(0);
                    }
                    WingsFloatEdge::Leading => {
                        node.position_type = PositionType::Absolute;
                        node.left = px(0);
                        node.top = px(0);
                        node.bottom = px(0);
                    }
                    WingsFloatEdge::Free => {}
                }
            }
            commands.entity(entity).insert((
                WingsFloating {
                    home,
                    index,
                    edge,
                    scaffold,
                },
                ChildOf(root),
            ));
        }
    }
}

fn px_of(v: Val) -> f32 {
    match v {
        Val::Px(p) => p,
        _ => 0.0,
    }
}

/// Measures the floating chrome of each scaffold and pads the scaffold content by it.
pub fn chrome_insets_system(
    mut commands: Commands,
    floating: Query<
        (
            &WingsFloating,
            &Node,
            Option<&crate::scaffold_skin::WingsResolvedTitle>,
        ),
        Without<WingsScaffold>,
    >,
    mut scaffolds: Query<(Entity, &mut Node, Option<&mut WingsChromeInsets>), With<WingsScaffold>>,
) {
    for (scaffold, mut node, insets) in &mut scaffolds {
        let mut top: f32 = 0.0;
        let mut bottom: f32 = 0.0;
        let mut leading: f32 = 0.0;
        for (f, n, title) in &floating {
            if f.scaffold != scaffold || n.display == Display::None {
                continue;
            }
            match f.edge {
                WingsFloatEdge::Top => {
                    // A collapsed large title keeps the expanded inset so content does not jump:
                    // the title scrolls away with it instead.
                    let collapsed = title.is_some_and(|t| t.large && t.collapsed);
                    let extra = if collapsed {
                        crate::scaffold_skin::IOS_LARGE_TITLE
                    } else {
                        0.0
                    };
                    top = top.max(px_of(n.height) + extra);
                }
                WingsFloatEdge::Bottom => {
                    bottom =
                        bottom.max(px_of(n.height) + px_of(n.margin.bottom) + px_of(n.margin.top));
                }
                WingsFloatEdge::Leading => {
                    leading =
                        leading.max(px_of(n.width) + px_of(n.margin.left) + px_of(n.margin.right));
                }
                WingsFloatEdge::Free => {}
            }
        }
        let wanted = Vec4::new(top, bottom, leading, 0.0);
        let Some(mut insets) = insets else {
            commands
                .entity(scaffold)
                .insert(WingsChromeInsets::default());
            continue;
        };
        if insets.applied == wanted {
            continue;
        }
        let applied = insets.applied;
        node.padding.top = px(px_of(node.padding.top) - applied.x + wanted.x);
        node.padding.bottom = px(px_of(node.padding.bottom) - applied.y + wanted.y);
        node.padding.left = px(px_of(node.padding.left) - applied.z + wanted.z);
        insets.top = top;
        insets.bottom = bottom;
        insets.leading = leading;
        insets.applied = wanted;
    }
}

/// Lets screen filling scroll columns reach back under the floating chrome: they stretch from
/// the top to the bottom of the window while their content starts below the chrome, so it
/// scrolls beneath it.
pub fn scroll_under_chrome(
    scaffolds: Query<(Entity, Ref<WingsChromeInsets>)>,
    added: Query<(), Added<WingsScrollColumn>>,
    children: Query<&Children>,
    mut columns: Query<(&mut Node, Option<&mut WingsUnderChrome>), With<WingsScrollColumn>>,
    parents: Query<&ChildOf>,
    scroll_parents: Query<(), With<WingsScrollColumn>>,
    mut commands: Commands,
) {
    let any_added = !added.is_empty();
    for (scaffold, insets) in &scaffolds {
        if !insets.is_changed() && !any_added {
            continue;
        }
        for e in children.iter_descendants(scaffold) {
            let Ok((mut node, under)) = columns.get_mut(e) else {
                continue;
            };
            // Only outermost scroll columns (screens), not nested ones.
            let nested = std::iter::successors(parents.get(e).ok().map(|p| p.parent()), |p| {
                parents.get(*p).ok().map(|pp| pp.parent())
            })
            .take_while(|p| *p != scaffold)
            .any(|p| scroll_parents.contains(p));
            if nested {
                continue;
            }
            let wanted = Vec2::new(insets.top, insets.bottom);
            let current = under.as_ref().map_or(Vec2::ZERO, |u| u.insets);
            if current == wanted {
                continue;
            }
            let mut state = under.map(|u| *u).unwrap_or(WingsUnderChrome {
                insets: Vec2::ZERO,
                original: UnderChromeOriginal::of(&node),
            });
            // Undo the previous adjustment.
            node.padding.top = px(px_of(node.padding.top) - state.insets.x);
            node.padding.bottom = px(px_of(node.padding.bottom) - state.insets.y);
            if wanted == Vec2::ZERO {
                state.original.restore(&mut node);
            } else {
                node.position_type = PositionType::Absolute;
                node.left = px(0);
                node.right = px(0);
                node.top = px(-wanted.x);
                node.bottom = px(-wanted.y);
                node.height = Val::Auto;
                node.padding.top = px(px_of(node.padding.top) + wanted.x);
                node.padding.bottom = px(px_of(node.padding.bottom) + wanted.y);
            }
            state.insets = wanted;
            commands.entity(e).insert(state);
        }
    }
}

/// Layout a scroll column had before reaching under the chrome.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UnderChromeOriginal {
    position_type: PositionType,
    left: Val,
    right: Val,
    top: Val,
    bottom: Val,
    height: Val,
}

impl UnderChromeOriginal {
    fn of(node: &Node) -> Self {
        Self {
            position_type: node.position_type,
            left: node.left,
            right: node.right,
            top: node.top,
            bottom: node.bottom,
            height: node.height,
        }
    }

    fn restore(&self, node: &mut Node) {
        node.position_type = self.position_type;
        node.left = self.left;
        node.right = self.right;
        node.top = self.top;
        node.bottom = self.bottom;
        node.height = self.height;
    }
}

/// How a scroll column reaches under floating chrome.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct WingsUnderChrome {
    /// Top and bottom chrome insets applied.
    pub insets: Vec2,
    original: UnderChromeOriginal,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// iOS floats bars and toasts, macOS toolbar, sidebar and toasts, the others nothing.
    #[test]
    fn floating_per_design() {
        use WingsFloatEdge::*;
        assert!(design_floats(WingsDesign::Ios, Bottom));
        assert!(!design_floats(WingsDesign::Ios, Leading));
        assert!(design_floats(WingsDesign::MacOs, Leading));
        assert!(!design_floats(WingsDesign::MacOs, Bottom));
        for edge in [Top, Bottom, Leading, Free] {
            assert!(!design_floats(WingsDesign::Material, edge));
            assert!(!design_floats(WingsDesign::Fluent, edge));
        }
    }
}
