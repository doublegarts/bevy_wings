//! # bevy_wings_ui_modifiers
//!
//! [`WingsModifier`] is an ordered, chainable description of how a node looks and behaves:
//! size, padding, background, border, shape, elevation, transform, opacity and interaction
//! feedback. It is a plain component, so it works the same in `bsn!` scenes, in `commands.spawn`
//! and as a widget prop:
//!
//! ```ignore
//! commands.spawn((
//!     Node::default(),
//!     WingsModifier::new()
//!         .fill_max_width()
//!         .padding(16.0)
//!         .background(ColorRole::SurfaceContainer)
//!         .shape(ShapeRole::Large)
//!         .elevation(2)
//!         .hover_scale(1.02),
//! ));
//! ```
//!
//! The [`apply_modifiers`] system folds the chain into Bevy UI components only when the
//! modifier changes. Later operations win over earlier ones, exactly like reading the chain from
//! left to right. Paint operations insert theme components, so modified nodes follow theme
//! switches automatically.
//!
//! Note: a modifier *sets* fields. Removing an operation from a chain does not restore the
//! previous value of that field.

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/doublegarts/bevy_wings/main/assets/branding/bevy-wings-icon.png",
    html_favicon_url = "https://raw.githubusercontent.com/doublegarts/bevy_wings/main/assets/branding/bevy-wings-icon.png"
)]

use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::{
    AlignSelf, BorderRadius, Node, Overflow, PositionType, UiRect, UiTransform, Val, Val2, ZIndex,
};

use bevy_wings_ui_animation::prelude::{AnimationSpec, ScaleProperty, WingsAnimateExt};
use bevy_wings_ui_core::{WingsOpacity, WingsUiSystems, ensure_core};
use bevy_wings_ui_theme::prelude::{
    ShapeRole, WingsBackground, WingsBorderPaint, WingsElevationLevel, WingsPaint, WingsShape,
};

/// A single modifier operation.
#[derive(Debug, Clone, PartialEq)]
pub enum ModifierOp {
    /// Width.
    Width(Val),
    /// Height.
    Height(Val),
    /// Minimum width.
    MinWidth(Val),
    /// Minimum height.
    MinHeight(Val),
    /// Maximum width.
    MaxWidth(Val),
    /// Maximum height.
    MaxHeight(Val),
    /// Aspect ratio (width / height).
    AspectRatio(f32),
    /// Flex grow weight with a zero basis.
    Weight(f32),
    /// Padding.
    Padding(UiRect),
    /// Margin.
    Margin(UiRect),
    /// Border width with its paint.
    Border(UiRect, WingsPaint),
    /// Background paint.
    Background(WingsPaint),
    /// Theme shape.
    Shape(ShapeRole),
    /// Explicit corner radius.
    CornerRadius(BorderRadius),
    /// Elevation level.
    Elevation(u8),
    /// Visual translation in pixels (does not affect layout).
    Offset(Vec2),
    /// Absolute positioning relative to the parent.
    Absolute {
        /// Left offset.
        left: Val,
        /// Top offset.
        top: Val,
    },
    /// Uniform or per axis scale.
    Scale(Vec2),
    /// Rotation in degrees.
    Rotate(f32),
    /// Subtree opacity.
    Alpha(f32),
    /// Clip children to the node bounds.
    Clip,
    /// Self alignment in the parent.
    AlignSelf(AlignSelf),
    /// Local stacking order.
    ZIndex(i32),
    /// Debug or test name.
    Tag(&'static str),
    /// Scale to this factor while hovered.
    HoverScale(f32),
}

/// An ordered chain of [`ModifierOp`]s.
#[derive(Component, Debug, Clone, PartialEq, Default)]
#[require(Node)]
pub struct WingsModifier {
    ops: Vec<ModifierOp>,
}

/// Shorthand for [`WingsModifier::new`].
pub fn modifier() -> WingsModifier {
    WingsModifier::new()
}

macro_rules! op {
    ($(#[$doc:meta])* $name:ident($($arg:ident: $ty:ty),*) => $body:expr) => {
        $(#[$doc])*
        pub fn $name(mut self, $($arg: $ty),*) -> Self {
            self.ops.push($body);
            self
        }
    };
}

impl WingsModifier {
    /// An empty modifier.
    pub fn new() -> Self {
        Self { ops: Vec::new() }
    }

    /// The operations in order.
    pub fn ops(&self) -> &[ModifierOp] {
        &self.ops
    }

    /// Appends all operations of `other` (they win over the current ones).
    pub fn then(mut self, other: WingsModifier) -> Self {
        self.ops.extend(other.ops);
        self
    }

    op!(/// Fixed width in pixels.
        width(px: f32) => ModifierOp::Width(Val::Px(px)));
    op!(/// Fixed height in pixels.
        height(px: f32) => ModifierOp::Height(Val::Px(px)));
    op!(/// Width as any [`Val`].
        width_val(v: Val) => ModifierOp::Width(v));
    op!(/// Height as any [`Val`].
        height_val(v: Val) => ModifierOp::Height(v));
    op!(/// Minimum width in pixels.
        min_width(px: f32) => ModifierOp::MinWidth(Val::Px(px)));
    op!(/// Minimum height in pixels.
        min_height(px: f32) => ModifierOp::MinHeight(Val::Px(px)));
    op!(/// Maximum width in pixels.
        max_width(px: f32) => ModifierOp::MaxWidth(Val::Px(px)));
    op!(/// Maximum height in pixels.
        max_height(px: f32) => ModifierOp::MaxHeight(Val::Px(px)));
    op!(/// Aspect ratio (width / height).
        aspect_ratio(r: f32) => ModifierOp::AspectRatio(r));
    op!(/// Takes a share of the remaining space in a row or column.
        weight(w: f32) => ModifierOp::Weight(w));
    op!(/// Same padding on every side.
        padding(px: f32) => ModifierOp::Padding(UiRect::all(Val::Px(px))));
    op!(/// Horizontal and vertical padding.
        padding_xy(x: f32, y: f32) => ModifierOp::Padding(UiRect::axes(Val::Px(x), Val::Px(y))));
    op!(/// Padding per side.
        padding_rect(rect: UiRect) => ModifierOp::Padding(rect));
    op!(/// Same margin on every side.
        margin(px: f32) => ModifierOp::Margin(UiRect::all(Val::Px(px))));
    op!(/// Margin per side.
        margin_rect(rect: UiRect) => ModifierOp::Margin(rect));
    op!(/// Background from any paint (role, color, adaptive pair).
        background(paint: impl Into<WingsPaint>) => ModifierOp::Background(paint.into()));
    op!(/// Border with a width in pixels and a paint.
        border(px: f32, paint: impl Into<WingsPaint>) => ModifierOp::Border(UiRect::all(Val::Px(px)), paint.into()));
    op!(/// Theme shape for the corners.
        shape(role: ShapeRole) => ModifierOp::Shape(role));
    op!(/// Explicit corner radius in pixels.
        rounded(px: f32) => ModifierOp::CornerRadius(BorderRadius::all(Val::Px(px))));
    op!(/// Elevation level `0..=5`.
        elevation(level: u8) => ModifierOp::Elevation(level));
    op!(/// Visual offset in pixels.
        offset(x: f32, y: f32) => ModifierOp::Offset(Vec2::new(x, y)));
    op!(/// Absolute position relative to the parent.
        absolute(left: f32, top: f32) => ModifierOp::Absolute { left: Val::Px(left), top: Val::Px(top) });
    op!(/// Uniform scale.
        scale(s: f32) => ModifierOp::Scale(Vec2::splat(s)));
    op!(/// Rotation in degrees.
        rotate(degrees: f32) => ModifierOp::Rotate(degrees));
    op!(/// Subtree opacity.
        alpha(a: f32) => ModifierOp::Alpha(a));
    op!(/// Clips overflowing children.
        clip() => ModifierOp::Clip);
    op!(/// Aligns this node inside its parent.
        align_self(a: AlignSelf) => ModifierOp::AlignSelf(a));
    op!(/// Local z order.
        z_index(z: i32) => ModifierOp::ZIndex(z));
    op!(/// Attaches a [`Name`] for debugging and tests.
        tag(name: &'static str) => ModifierOp::Tag(name));
    op!(/// Animates to `factor` scale while the pointer hovers the node.
        hover_scale(factor: f32) => ModifierOp::HoverScale(factor));

    /// Fills the parent width.
    pub fn fill_max_width(self) -> Self {
        self.width_val(Val::Percent(100.0))
    }

    /// Fills the parent height.
    pub fn fill_max_height(self) -> Self {
        self.height_val(Val::Percent(100.0))
    }

    /// Fills the parent in both directions.
    pub fn fill_max_size(self) -> Self {
        self.fill_max_width().fill_max_height()
    }

    /// Fixed width and height.
    pub fn size(self, w: f32, h: f32) -> Self {
        self.width(w).height(h)
    }

    /// Fixed square size.
    pub fn square(self, side: f32) -> Self {
        self.size(side, side)
    }

    /// Fully rounded corners (pill or circle).
    pub fn circle(self) -> Self {
        self.shape(ShapeRole::Full)
    }

    /// Folds the chain into a [`Node`] and returns the non layout results.
    pub fn fold(&self, node: &mut Node) -> ModifierOutput {
        let mut out = ModifierOutput::default();
        for op in &self.ops {
            match op {
                ModifierOp::Width(v) => node.width = *v,
                ModifierOp::Height(v) => node.height = *v,
                ModifierOp::MinWidth(v) => node.min_width = *v,
                ModifierOp::MinHeight(v) => node.min_height = *v,
                ModifierOp::MaxWidth(v) => node.max_width = *v,
                ModifierOp::MaxHeight(v) => node.max_height = *v,
                ModifierOp::AspectRatio(r) => node.aspect_ratio = Some(*r),
                ModifierOp::Weight(w) => {
                    node.flex_grow = *w;
                    node.flex_shrink = 1.0;
                    node.flex_basis = Val::Px(0.0);
                }
                ModifierOp::Padding(r) => node.padding = *r,
                ModifierOp::Margin(r) => node.margin = *r,
                ModifierOp::Border(r, p) => {
                    node.border = *r;
                    out.border = Some(*p);
                }
                ModifierOp::Background(p) => out.background = Some(*p),
                ModifierOp::Shape(s) => {
                    out.shape = Some(*s);
                }
                ModifierOp::CornerRadius(r) => {
                    out.shape = None;
                    node.border_radius = *r;
                }
                ModifierOp::Elevation(l) => out.elevation = Some(*l),
                ModifierOp::Offset(o) => {
                    out.transform
                        .get_or_insert(UiTransform::IDENTITY)
                        .translation = Val2::px(o.x, o.y)
                }
                ModifierOp::Absolute { left, top } => {
                    node.position_type = PositionType::Absolute;
                    node.left = *left;
                    node.top = *top;
                }
                ModifierOp::Scale(s) => {
                    out.transform.get_or_insert(UiTransform::IDENTITY).scale = *s
                }
                ModifierOp::Rotate(d) => {
                    out.transform.get_or_insert(UiTransform::IDENTITY).rotation = Rot2::degrees(*d)
                }
                ModifierOp::Alpha(a) => out.alpha = Some(*a),
                ModifierOp::Clip => node.overflow = Overflow::clip(),
                ModifierOp::AlignSelf(a) => node.align_self = *a,
                ModifierOp::ZIndex(z) => out.z_index = Some(*z),
                ModifierOp::Tag(t) => out.tag = Some(t),
                ModifierOp::HoverScale(s) => out.hover_scale = Some(*s),
            }
        }
        out
    }
}

/// Non layout results of folding a modifier.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ModifierOutput {
    /// Background paint.
    pub background: Option<WingsPaint>,
    /// Border paint.
    pub border: Option<WingsPaint>,
    /// Theme shape.
    pub shape: Option<ShapeRole>,
    /// Elevation.
    pub elevation: Option<u8>,
    /// Transform.
    pub transform: Option<UiTransform>,
    /// Opacity.
    pub alpha: Option<f32>,
    /// Z index.
    pub z_index: Option<i32>,
    /// Name.
    pub tag: Option<&'static str>,
    /// Hover scale.
    pub hover_scale: Option<f32>,
}

/// Scales the node to `factor` while hovered, with a spring.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
#[require(Hovered, UiTransform)]
pub struct WingsHoverScale(pub f32);

/// Folds changed modifiers into components.
pub fn apply_modifiers(
    mut commands: Commands,
    mut query: Query<(Entity, &WingsModifier, &mut Node), Changed<WingsModifier>>,
) {
    for (entity, modifier, mut node) in &mut query {
        let out = modifier.fold(&mut node);
        let mut e = commands.entity(entity);
        if let Some(p) = out.background {
            e.insert(WingsBackground(p));
        }
        if let Some(p) = out.border {
            e.insert(WingsBorderPaint(p));
        }
        if let Some(s) = out.shape {
            e.insert(WingsShape(s));
        }
        if let Some(l) = out.elevation {
            e.insert(WingsElevationLevel(l));
        }
        if let Some(t) = out.transform {
            e.insert(t);
        }
        if let Some(a) = out.alpha {
            e.insert(WingsOpacity(a));
        }
        if let Some(z) = out.z_index {
            e.insert(ZIndex(z));
        }
        if let Some(t) = out.tag {
            e.insert(Name::new(t));
        }
        if let Some(s) = out.hover_scale {
            e.insert(WingsHoverScale(s));
        }
    }
}

/// Animates [`WingsHoverScale`] nodes when their hover state changes.
pub fn hover_scale_system(
    mut commands: Commands,
    query: Query<(Entity, &WingsHoverScale, &Hovered), Changed<Hovered>>,
) {
    for (entity, scale, hovered) in &query {
        let target = if hovered.0 {
            Vec2::splat(scale.0)
        } else {
            Vec2::ONE
        };
        commands
            .entity(entity)
            .animate::<ScaleProperty>(target, AnimationSpec::spring());
    }
}

/// Adds the modifier systems.
#[derive(Default)]
pub struct WingsModifierPlugin;

impl Plugin for WingsModifierPlugin {
    fn build(&self, app: &mut App) {
        ensure_core(app);
        app.add_systems(
            Update,
            (apply_modifiers, hover_scale_system).in_set(WingsUiSystems::Modifier),
        );
    }
}

/// Commonly used items.
pub mod prelude {
    pub use crate::{
        ModifierOp, ModifierOutput, WingsHoverScale, WingsModifier, WingsModifierPlugin, modifier,
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_wings_ui_theme::prelude::ColorRole;

    /// Layout operations land in the node.
    #[test]
    fn fold_layout() {
        let m = modifier()
            .fill_max_width()
            .padding(8.0)
            .height(40.0)
            .weight(2.0);
        let mut node = Node::default();
        m.fold(&mut node);
        assert_eq!(node.width, Val::Percent(100.0));
        assert_eq!(node.height, Val::Px(40.0));
        assert_eq!(node.padding, UiRect::all(Val::Px(8.0)));
        assert_eq!(node.flex_grow, 2.0);
        assert_eq!(node.flex_basis, Val::Px(0.0));
    }

    /// Later operations win.
    #[test]
    fn last_op_wins() {
        let m = modifier().padding(4.0).padding(12.0);
        let mut node = Node::default();
        m.fold(&mut node);
        assert_eq!(node.padding, UiRect::all(Val::Px(12.0)));
    }

    /// Paint, transform and misc operations are reported in the output.
    #[test]
    fn fold_outputs() {
        let m = modifier()
            .background(ColorRole::Primary)
            .border(2.0, ColorRole::Outline)
            .shape(ShapeRole::Small)
            .elevation(3)
            .offset(4.0, 5.0)
            .scale(1.5)
            .rotate(90.0)
            .alpha(0.5)
            .z_index(3)
            .tag("hero")
            .hover_scale(1.1);
        let mut node = Node::default();
        let out = m.fold(&mut node);
        assert_eq!(out.background, Some(WingsPaint::Role(ColorRole::Primary)));
        assert_eq!(out.border, Some(WingsPaint::Role(ColorRole::Outline)));
        assert_eq!(node.border, UiRect::all(Val::Px(2.0)));
        assert_eq!(out.shape, Some(ShapeRole::Small));
        assert_eq!(out.elevation, Some(3));
        let t = out.transform.unwrap();
        assert_eq!(t.translation, Val2::px(4.0, 5.0));
        assert_eq!(t.scale, Vec2::splat(1.5));
        assert!((t.rotation.as_degrees() - 90.0).abs() < 1e-3);
        assert_eq!(out.alpha, Some(0.5));
        assert_eq!(out.z_index, Some(3));
        assert_eq!(out.tag, Some("hero"));
        assert_eq!(out.hover_scale, Some(1.1));
    }

    /// An explicit corner radius cancels an earlier theme shape.
    #[test]
    fn rounded_overrides_shape() {
        let mut node = Node::default();
        let out = modifier()
            .shape(ShapeRole::Large)
            .rounded(3.0)
            .fold(&mut node);
        assert_eq!(out.shape, None);
        assert_eq!(node.border_radius, BorderRadius::all(Val::Px(3.0)));
    }

    /// `then` appends another chain.
    #[test]
    fn then_appends() {
        let m = modifier().padding(1.0).then(modifier().padding(2.0).clip());
        assert_eq!(m.ops().len(), 3);
    }

    /// The plugin applies the modifier and inserts theme components.
    #[test]
    fn system_applies_components() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, WingsModifierPlugin));
        let e = app
            .world_mut()
            .spawn(
                modifier()
                    .square(48.0)
                    .background(ColorRole::Primary)
                    .circle()
                    .alpha(0.4),
            )
            .id();
        app.update();
        let w = app.world();
        assert_eq!(w.get::<Node>(e).unwrap().width, Val::Px(48.0));
        assert_eq!(
            w.get::<WingsBackground>(e),
            Some(&WingsBackground(WingsPaint::Role(ColorRole::Primary)))
        );
        assert_eq!(w.get::<WingsShape>(e), Some(&WingsShape(ShapeRole::Full)));
        assert_eq!(w.get::<WingsOpacity>(e), Some(&WingsOpacity(0.4)));
    }

    /// Changing the modifier re-applies it.
    #[test]
    fn reapplies_on_change() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, WingsModifierPlugin));
        let e = app.world_mut().spawn(modifier().width(10.0)).id();
        app.update();
        *app.world_mut().get_mut::<WingsModifier>(e).unwrap() = modifier().width(20.0);
        app.update();
        assert_eq!(app.world().get::<Node>(e).unwrap().width, Val::Px(20.0));
    }
}
