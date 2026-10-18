//! # bevy_wings_ui_responsive
//!
//! Responsive UI driven by ECS.
//!
//! * [`WingsViewport`] is a resource describing the primary window: logical size, width and
//!   height [`WidthClass`] / [`HeightClass`], and [`Orientation`]. It is updated only when the
//!   window actually changes, so systems can use `resource_changed::<WingsViewport>` as a cheap
//!   run condition.
//! * [`WingsBreakpoints`] defines the class boundaries (600, 840, 1200 and 1600 logical
//!   pixels by default).
//! * [`WingsResponsive<T>`] stores one value per width class with fallback to the closest
//!   smaller class.
//! * Components react automatically when the width class changes:
//!   [`WingsResponsiveNode`] patches the [`Node`], [`WingsResponsiveVisibility`] shows or hides
//!   a subtree, [`WingsResponsiveColumns`] sets grid columns, and
//!   [`WingsResponsiveComponent<C>`] inserts any component.
//! * [`WingsAutoScale`] adjusts [`UiScale`] per class.
//!
//! ```ignore
//! commands.spawn((
//!     Node { display: Display::Grid, ..default() },
//!     WingsResponsiveColumns(WingsResponsive::new(1).medium(2).expanded(3).large(4)),
//! ));
//! ```

use std::sync::Arc;

use bevy::prelude::*;
use bevy::ui::{Display, GridTrack, Node, RepeatedGridTrack, UiScale};
use bevy::window::{PrimaryWindow, Window};

use bevy_wings_ui_core::{WingsUiSystems, ensure_core};

pub mod direction;
pub mod safe_area;

pub use direction::{WingsLayoutDirection, WingsNoMirror};
pub use safe_area::{
    WingsDevicePreview, WingsDeviceProfile, WingsEdges, WingsInsets, WingsSafeArea,
    WingsSafeAreaPadding, WingsScreenKind,
};

/// Width based size class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Reflect)]
pub enum WidthClass {
    /// Phones in portrait (below 600 px by default).
    #[default]
    Compact,
    /// Tablets in portrait, foldables (600 to 839 px).
    Medium,
    /// Tablets in landscape, small desktops (840 to 1199 px).
    Expanded,
    /// Desktops (1200 to 1599 px).
    Large,
    /// Wide desktops (1600 px and above).
    ExtraLarge,
}

impl WidthClass {
    /// All classes from smallest to largest.
    pub const ALL: [WidthClass; 5] = [
        WidthClass::Compact,
        WidthClass::Medium,
        WidthClass::Expanded,
        WidthClass::Large,
        WidthClass::ExtraLarge,
    ];

    /// Index in [`WidthClass::ALL`].
    pub fn index(self) -> usize {
        self as usize
    }

    /// True for this class or any larger one.
    pub fn at_least(self, other: WidthClass) -> bool {
        self >= other
    }
}

/// Height based size class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Reflect)]
pub enum HeightClass {
    /// Landscape phones (below 480 px by default).
    Compact,
    /// Most devices (480 to 899 px).
    #[default]
    Medium,
    /// Tall screens (900 px and above).
    Expanded,
}

/// Screen orientation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Reflect)]
pub enum Orientation {
    /// Taller than wide.
    #[default]
    Portrait,
    /// Wider than tall (or square).
    Landscape,
}

/// Class boundaries in logical pixels.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct WingsBreakpoints {
    /// Minimum width of [`WidthClass::Medium`].
    pub medium: f32,
    /// Minimum width of [`WidthClass::Expanded`].
    pub expanded: f32,
    /// Minimum width of [`WidthClass::Large`].
    pub large: f32,
    /// Minimum width of [`WidthClass::ExtraLarge`].
    pub extra_large: f32,
    /// Minimum height of [`HeightClass::Medium`].
    pub height_medium: f32,
    /// Minimum height of [`HeightClass::Expanded`].
    pub height_expanded: f32,
}

impl Default for WingsBreakpoints {
    fn default() -> Self {
        Self {
            medium: 600.0,
            expanded: 840.0,
            large: 1200.0,
            extra_large: 1600.0,
            height_medium: 480.0,
            height_expanded: 900.0,
        }
    }
}

impl WingsBreakpoints {
    /// Classifies a width.
    pub fn width_class(&self, width: f32) -> WidthClass {
        if width >= self.extra_large {
            WidthClass::ExtraLarge
        } else if width >= self.large {
            WidthClass::Large
        } else if width >= self.expanded {
            WidthClass::Expanded
        } else if width >= self.medium {
            WidthClass::Medium
        } else {
            WidthClass::Compact
        }
    }

    /// Classifies a height.
    pub fn height_class(&self, height: f32) -> HeightClass {
        if height >= self.height_expanded {
            HeightClass::Expanded
        } else if height >= self.height_medium {
            HeightClass::Medium
        } else {
            HeightClass::Compact
        }
    }
}

/// Description of the current viewport.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Default)]
pub struct WingsViewport {
    /// Logical width.
    pub width: f32,
    /// Logical height.
    pub height: f32,
    /// Window scale factor.
    pub scale_factor: f32,
    /// Width class.
    pub width_class: WidthClass,
    /// Height class.
    pub height_class: HeightClass,
    /// Orientation.
    pub orientation: Orientation,
}

impl WingsViewport {
    /// Builds a viewport description from a logical size.
    pub fn from_size(width: f32, height: f32, scale_factor: f32, bp: &WingsBreakpoints) -> Self {
        Self {
            width,
            height,
            scale_factor,
            width_class: bp.width_class(width),
            height_class: bp.height_class(height),
            orientation: if width >= height {
                Orientation::Landscape
            } else {
                Orientation::Portrait
            },
        }
    }
}

/// When present, this resource replaces the primary window as the viewport source. Useful for
/// tests, previews of other device sizes, and split screen tools.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct WingsViewportOverride {
    /// Logical width.
    pub width: f32,
    /// Logical height.
    pub height: f32,
}

/// Sent when the width class changes.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct WidthClassChanged {
    /// Previous class.
    pub old: WidthClass,
    /// New class.
    pub new: WidthClass,
}

/// One value per width class. Missing entries fall back to the closest smaller class.
#[derive(Debug, Clone, PartialEq)]
pub struct WingsResponsive<T> {
    values: [Option<T>; 5],
}

impl<T: Clone> WingsResponsive<T> {
    /// Value used for compact and every larger class without its own value.
    pub fn new(compact: T) -> Self {
        Self {
            values: [Some(compact), None, None, None, None],
        }
    }

    fn with(mut self, class: WidthClass, value: T) -> Self {
        self.values[class.index()] = Some(value);
        self
    }

    /// Value from [`WidthClass::Medium`] upwards.
    pub fn medium(self, v: T) -> Self {
        self.with(WidthClass::Medium, v)
    }

    /// Value from [`WidthClass::Expanded`] upwards.
    pub fn expanded(self, v: T) -> Self {
        self.with(WidthClass::Expanded, v)
    }

    /// Value from [`WidthClass::Large`] upwards.
    pub fn large(self, v: T) -> Self {
        self.with(WidthClass::Large, v)
    }

    /// Value for [`WidthClass::ExtraLarge`].
    pub fn extra_large(self, v: T) -> Self {
        self.with(WidthClass::ExtraLarge, v)
    }

    /// Resolves the value for a class.
    pub fn get(&self, class: WidthClass) -> &T {
        self.values[..=class.index()]
            .iter()
            .rev()
            .find_map(Option::as_ref)
            .expect("compact value is always set")
    }
}

/// A function patching a [`Node`].
pub type NodePatch = Arc<dyn Fn(&mut Node) + Send + Sync>;

/// Creates a [`NodePatch`] from a closure.
pub fn node_patch(f: impl Fn(&mut Node) + Send + Sync + 'static) -> NodePatch {
    Arc::new(f)
}

/// Patches the entity's [`Node`] when the width class changes.
#[derive(Component, Clone)]
#[require(Node)]
pub struct WingsResponsiveNode(pub WingsResponsive<NodePatch>);

/// Shows the subtree only on the listed width classes.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
#[require(Node)]
pub struct WingsResponsiveVisibility {
    /// Visibility per class, indexed by [`WidthClass::index`].
    pub visible_on: [bool; 5],
    /// Display used when visible.
    pub display: Display,
}

impl WingsResponsiveVisibility {
    /// Visible only on the given classes.
    pub fn only(classes: &[WidthClass]) -> Self {
        let mut visible_on = [false; 5];
        for c in classes {
            visible_on[c.index()] = true;
        }
        Self {
            visible_on,
            display: Display::Flex,
        }
    }

    /// Visible on `class` and every larger class.
    pub fn from(class: WidthClass) -> Self {
        let mut visible_on = [false; 5];
        for c in WidthClass::ALL {
            visible_on[c.index()] = c >= class;
        }
        Self {
            visible_on,
            display: Display::Flex,
        }
    }

    /// Visible on classes strictly smaller than `class`.
    pub fn below(class: WidthClass) -> Self {
        let mut visible_on = [false; 5];
        for c in WidthClass::ALL {
            visible_on[c.index()] = c < class;
        }
        Self {
            visible_on,
            display: Display::Flex,
        }
    }

    /// Uses another display mode when visible.
    pub fn with_display(mut self, display: Display) -> Self {
        self.display = display;
        self
    }
}

/// Sets equal width grid columns per width class.
#[derive(Component, Debug, Clone, PartialEq)]
#[require(Node)]
pub struct WingsResponsiveColumns(pub WingsResponsive<u16>);

/// Inserts a component chosen per width class.
#[derive(Component, Clone)]
pub struct WingsResponsiveComponent<C: Component + Clone>(pub WingsResponsive<C>);

/// Adjusts [`UiScale`] per width class when present.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct WingsAutoScale(pub WingsResponsive<f32>);

/// Updates [`WingsViewport`] from the primary window or [`WingsViewportOverride`].
pub fn update_viewport(
    windows: Query<&Window, With<PrimaryWindow>>,
    override_size: Option<Res<WingsViewportOverride>>,
    breakpoints: Res<WingsBreakpoints>,
    mut viewport: ResMut<WingsViewport>,
    mut messages: MessageWriter<WidthClassChanged>,
) {
    let next = if let Some(o) = override_size {
        WingsViewport::from_size(o.width, o.height, 1.0, &breakpoints)
    } else if let Ok(window) = windows.single() {
        WingsViewport::from_size(
            window.width(),
            window.height(),
            window.scale_factor(),
            &breakpoints,
        )
    } else {
        return;
    };
    if *viewport != next {
        if viewport.width_class != next.width_class {
            messages.write(WidthClassChanged {
                old: viewport.width_class,
                new: next.width_class,
            });
        }
        *viewport = next;
    }
}

fn should_apply<T: Component>(viewport: &Res<WingsViewport>, item: &Ref<T>) -> bool {
    viewport.is_changed() || item.is_added() || item.is_changed()
}

/// Applies [`WingsResponsiveNode`] patches.
pub fn apply_responsive_nodes(
    viewport: Res<WingsViewport>,
    mut query: Query<(Ref<WingsResponsiveNode>, &mut Node)>,
) {
    for (r, mut node) in &mut query {
        if should_apply(&viewport, &r) {
            (r.0.get(viewport.width_class))(&mut node);
        }
    }
}

/// Applies [`WingsResponsiveVisibility`].
pub fn apply_responsive_visibility(
    viewport: Res<WingsViewport>,
    mut query: Query<(Ref<WingsResponsiveVisibility>, &mut Node)>,
) {
    for (r, mut node) in &mut query {
        if !should_apply(&viewport, &r) {
            continue;
        }
        let display = if r.visible_on[viewport.width_class.index()] {
            r.display
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
    }
}

/// Applies [`WingsResponsiveColumns`].
pub fn apply_responsive_columns(
    viewport: Res<WingsViewport>,
    mut query: Query<(Ref<WingsResponsiveColumns>, &mut Node)>,
) {
    for (r, mut node) in &mut query {
        if !should_apply(&viewport, &r) {
            continue;
        }
        let n = (*r.0.get(viewport.width_class)).max(1);
        node.display = Display::Grid;
        node.grid_template_columns = vec![RepeatedGridTrack::flex(n, 1.0)];
        node.grid_auto_rows = vec![GridTrack::auto()];
    }
}

/// Applies [`WingsResponsiveComponent<C>`].
pub fn apply_responsive_component<C: Component + Clone>(
    viewport: Res<WingsViewport>,
    query: Query<(Entity, Ref<WingsResponsiveComponent<C>>)>,
    mut commands: Commands,
) {
    for (e, r) in &query {
        if should_apply(&viewport, &r) {
            commands
                .entity(e)
                .insert(r.0.get(viewport.width_class).clone());
        }
    }
}

/// Applies [`WingsAutoScale`].
pub fn apply_auto_scale(
    viewport: Res<WingsViewport>,
    auto: Option<Res<WingsAutoScale>>,
    mut scale: ResMut<UiScale>,
) {
    let Some(auto) = auto else { return };
    if !(viewport.is_changed() || auto.is_changed()) {
        return;
    }
    let s = *auto.0.get(viewport.width_class);
    if scale.0 != s {
        scale.0 = s;
    }
}

/// App extension to register responsive components.
pub trait WingsResponsiveAppExt {
    /// Enables [`WingsResponsiveComponent<C>`] for component `C`.
    fn register_responsive_component<C: Component + Clone>(&mut self) -> &mut Self;
}

impl WingsResponsiveAppExt for App {
    fn register_responsive_component<C: Component + Clone>(&mut self) -> &mut Self {
        self.add_systems(
            Update,
            apply_responsive_component::<C>
                .in_set(WingsUiSystems::Responsive)
                .after(update_viewport),
        )
    }
}

/// Adds the viewport tracking and responsive component systems.
#[derive(Default)]
pub struct WingsResponsivePlugin {
    /// Custom breakpoints (default when `None`).
    pub breakpoints: Option<WingsBreakpoints>,
}

impl Plugin for WingsResponsivePlugin {
    fn build(&self, app: &mut App) {
        ensure_core(app);
        app.insert_resource(self.breakpoints.unwrap_or_default())
            .init_resource::<WingsViewport>()
            .init_resource::<UiScale>()
            .init_resource::<WingsSafeArea>()
            .init_resource::<WingsLayoutDirection>()
            .add_message::<WidthClassChanged>()
            .add_systems(
                Update,
                (
                    safe_area::apply_device_preview,
                    #[cfg(feature = "native")]
                    safe_area::read_native_safe_area,
                    update_viewport,
                    (
                        apply_responsive_nodes,
                        apply_responsive_visibility,
                        apply_responsive_columns,
                        apply_auto_scale,
                        safe_area::apply_safe_area_padding,
                        safe_area::draw_device_preview,
                        direction::apply_layout_direction,
                    ),
                )
                    .chain()
                    .in_set(WingsUiSystems::Responsive),
            );
    }
}

/// Run condition: true when the viewport width class is at least `class`.
pub fn width_at_least(class: WidthClass) -> impl Fn(Res<WingsViewport>) -> bool + Clone {
    move |v: Res<WingsViewport>| v.width_class >= class
}

/// Commonly used items.
pub mod prelude {
    pub use crate::direction::{WingsLayoutDirection, WingsNoMirror};
    pub use crate::safe_area::{
        WingsDevicePreview, WingsDeviceProfile, WingsEdges, WingsInsets, WingsSafeArea,
        WingsSafeAreaPadding, WingsScreenKind,
    };
    pub use crate::{
        HeightClass, NodePatch, Orientation, WidthClass, WidthClassChanged, WingsAutoScale,
        WingsBreakpoints, WingsResponsive, WingsResponsiveAppExt, WingsResponsiveColumns,
        WingsResponsiveComponent, WingsResponsiveNode, WingsResponsivePlugin,
        WingsResponsiveVisibility, WingsViewport, WingsViewportOverride, node_patch,
        width_at_least,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(width: f32, height: f32) -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, WingsResponsivePlugin::default()));
        app.insert_resource(WingsViewportOverride { width, height });
        app
    }

    fn resize(app: &mut App, width: f32, height: f32) {
        app.insert_resource(WingsViewportOverride { width, height });
        app.update();
    }

    /// Breakpoint boundaries are inclusive on the lower bound.
    #[test]
    fn classify_widths() {
        let bp = WingsBreakpoints::default();
        assert_eq!(bp.width_class(0.0), WidthClass::Compact);
        assert_eq!(bp.width_class(599.9), WidthClass::Compact);
        assert_eq!(bp.width_class(600.0), WidthClass::Medium);
        assert_eq!(bp.width_class(840.0), WidthClass::Expanded);
        assert_eq!(bp.width_class(1200.0), WidthClass::Large);
        assert_eq!(bp.width_class(4000.0), WidthClass::ExtraLarge);
    }

    /// Heights and orientation.
    #[test]
    fn classify_heights_and_orientation() {
        let bp = WingsBreakpoints::default();
        assert_eq!(bp.height_class(400.0), HeightClass::Compact);
        assert_eq!(bp.height_class(700.0), HeightClass::Medium);
        assert_eq!(bp.height_class(1000.0), HeightClass::Expanded);
        let v = WingsViewport::from_size(800.0, 400.0, 1.0, &bp);
        assert_eq!(v.orientation, Orientation::Landscape);
        let v = WingsViewport::from_size(400.0, 800.0, 1.0, &bp);
        assert_eq!(v.orientation, Orientation::Portrait);
    }

    /// Responsive values fall back to the closest smaller class.
    #[test]
    fn responsive_fallback() {
        let r = WingsResponsive::new(1).expanded(3);
        assert_eq!(*r.get(WidthClass::Compact), 1);
        assert_eq!(*r.get(WidthClass::Medium), 1);
        assert_eq!(*r.get(WidthClass::Expanded), 3);
        assert_eq!(*r.get(WidthClass::ExtraLarge), 3);
    }

    /// The viewport resource follows the override and reports class changes.
    #[test]
    fn viewport_tracks_override() {
        let mut app = app(400.0, 800.0);
        app.update();
        assert_eq!(
            app.world().resource::<WingsViewport>().width_class,
            WidthClass::Compact
        );
        resize(&mut app, 1300.0, 800.0);
        assert_eq!(
            app.world().resource::<WingsViewport>().width_class,
            WidthClass::Large
        );
        let msgs = app.world().resource::<Messages<WidthClassChanged>>();
        let mut cursor = msgs.get_cursor();
        let all: Vec<_> = cursor.read(msgs).copied().collect();
        assert!(all.contains(&WidthClassChanged {
            old: WidthClass::Compact,
            new: WidthClass::Large
        }));
    }

    /// An unchanged window does not mark the viewport as changed.
    #[test]
    fn viewport_not_touched_when_same() {
        let mut app = app(500.0, 500.0);
        app.update();
        let t1 = app.world().resource_ref::<WingsViewport>().last_changed();
        app.update();
        let t2 = app.world().resource_ref::<WingsViewport>().last_changed();
        assert_eq!(t1, t2);
    }

    /// Visibility switches with the width class.
    #[test]
    fn visibility_by_class() {
        let mut app = app(400.0, 800.0);
        let rail = app
            .world_mut()
            .spawn(WingsResponsiveVisibility::from(WidthClass::Medium))
            .id();
        let bar = app
            .world_mut()
            .spawn(WingsResponsiveVisibility::below(WidthClass::Medium))
            .id();
        app.update();
        assert_eq!(
            app.world().get::<Node>(rail).unwrap().display,
            Display::None
        );
        assert_eq!(app.world().get::<Node>(bar).unwrap().display, Display::Flex);
        resize(&mut app, 900.0, 800.0);
        assert_eq!(
            app.world().get::<Node>(rail).unwrap().display,
            Display::Flex
        );
        assert_eq!(app.world().get::<Node>(bar).unwrap().display, Display::None);
    }

    /// Columns follow the width class.
    #[test]
    fn columns_by_class() {
        let mut app = app(400.0, 800.0);
        let grid = app
            .world_mut()
            .spawn(WingsResponsiveColumns(WingsResponsive::new(1).expanded(3)))
            .id();
        app.update();
        let cols = |app: &App| {
            app.world()
                .get::<Node>(grid)
                .unwrap()
                .grid_template_columns
                .clone()
        };
        assert_eq!(cols(&app), vec![RepeatedGridTrack::flex(1, 1.0)]);
        resize(&mut app, 1000.0, 800.0);
        assert_eq!(cols(&app), vec![RepeatedGridTrack::flex(3, 1.0)]);
    }

    /// Node patches run on class changes.
    #[test]
    fn node_patches() {
        let mut app = app(400.0, 800.0);
        let e = app
            .world_mut()
            .spawn(WingsResponsiveNode(
                WingsResponsive::new(node_patch(|n| n.flex_direction = FlexDirection::Column))
                    .expanded(node_patch(|n| n.flex_direction = FlexDirection::Row)),
            ))
            .id();
        app.update();
        assert_eq!(
            app.world().get::<Node>(e).unwrap().flex_direction,
            FlexDirection::Column
        );
        resize(&mut app, 1000.0, 800.0);
        assert_eq!(
            app.world().get::<Node>(e).unwrap().flex_direction,
            FlexDirection::Row
        );
    }

    #[derive(Component, Clone, Debug, PartialEq)]
    struct Density(u8);

    /// Generic responsive components are inserted per class.
    #[test]
    fn generic_component() {
        let mut app = app(400.0, 800.0);
        app.register_responsive_component::<Density>();
        let e = app
            .world_mut()
            .spawn(WingsResponsiveComponent(
                WingsResponsive::new(Density(1)).large(Density(3)),
            ))
            .id();
        app.update();
        assert_eq!(app.world().get::<Density>(e), Some(&Density(1)));
        resize(&mut app, 1300.0, 800.0);
        assert_eq!(app.world().get::<Density>(e), Some(&Density(3)));
    }

    /// Auto scale sets `UiScale` per class.
    #[test]
    fn auto_scale() {
        let mut app = app(400.0, 800.0);
        app.insert_resource(WingsAutoScale(WingsResponsive::new(1.0).large(1.25)));
        app.update();
        assert_eq!(app.world().resource::<UiScale>().0, 1.0);
        resize(&mut app, 1300.0, 800.0);
        assert_eq!(app.world().resource::<UiScale>().0, 1.25);
    }
}
