//! ECS painting: the bridge between semantic theme tokens and Bevy UI components.
//!
//! Instead of writing [`BackgroundColor`] or [`TextColor`] directly, Wings entities carry
//! *paint* components that reference theme roles:
//!
//! | Component | Writes | Purpose |
//! |---|---|---|
//! | [`WingsBackground`] | [`BackgroundColor`] | Fill color of a node |
//! | [`WingsForeground`] | [`TextColor`] | Text color of a text entity |
//! | [`WingsBorderPaint`] | [`BorderColor`] | Border color |
//! | [`WingsContentColor`] | nothing | Content color provided to descendants |
//! | [`WingsInheritContentColor`] | [`TextColor`] | Text that uses the closest [`WingsContentColor`] |
//! | [`WingsStateLayer`] | [`BackgroundColor`] | Hover and press overlay on top of the fill |
//! | [`WingsTypeStyle`] | `TextFont`, `LineHeight`, `LetterSpacing` | Type scale role |
//! | [`WingsShape`] | `Node::border_radius` | Shape role |
//! | [`WingsElevationLevel`] | [`BoxShadow`] | Elevation role |
//!
//! The [`paint_system`] resolves paints into colors. It is incremental: only entities whose
//! paint inputs changed are touched in a normal frame, and the whole UI is repainted in a
//! single pass only when the [`WingsTheme`] resource changes. That is how twenty (or two
//! thousand) white buttons all turn dark in the very frame the user flips the theme, while a
//! frame without changes costs almost nothing.
//!
//! When the theme flips, colors cross fade using [`WingsMotion::theme_transition`]
//! (powered by the Wings animation crate).
//!
//! [`WingsMotion::theme_transition`]: crate::tokens::WingsMotion::theme_transition

use bevy::color::{Alpha, Color, LinearRgba};
use bevy::ecs::entity::EntityHashSet;
use bevy::prelude::*;
use bevy::text::{FontSize, LetterSpacing, LineHeight, TextColor, TextFont};
use bevy::ui::{BackgroundColor, BorderColor, BoxShadow, Node};

use bevy_wings_ui_animation::prelude::{
    AnimationSpec, BackgroundColorProperty, BorderColorProperty, Repeat, TextColorProperty,
    WingsAnimator, WingsProperty,
};
use bevy_wings_ui_core::WingsOpacity;
use bevy_wings_ui_core::hierarchy::ancestors;

use crate::scheme::{ColorRole, WingsColorScheme};
use crate::theme::{WingsLocalTheme, WingsTheme, WingsThemeChanged};
use crate::tokens::{ShapeRole, TypeRole};

/// A color source: a theme role, a role with alpha, a fixed color, or a light and dark pair.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum WingsPaint {
    /// A theme role.
    Role(ColorRole),
    /// A theme role with an alpha multiplier.
    RoleAlpha(ColorRole, f32),
    /// A fixed color that ignores the theme.
    Fixed(Color),
    /// Explicit colors for light and dark schemes.
    Adaptive {
        /// Light scheme color.
        light: Color,
        /// Dark scheme color.
        dark: Color,
    },
    /// Fully transparent.
    #[default]
    Transparent,
}

impl From<ColorRole> for WingsPaint {
    fn from(role: ColorRole) -> Self {
        Self::Role(role)
    }
}

impl From<Color> for WingsPaint {
    fn from(color: Color) -> Self {
        Self::Fixed(color)
    }
}

impl WingsPaint {
    /// Resolves the paint against a scheme.
    pub fn resolve(&self, scheme: &WingsColorScheme, dark: bool) -> Color {
        match *self {
            WingsPaint::Role(r) => scheme.get(r),
            WingsPaint::RoleAlpha(r, a) => {
                let c = scheme.get(r);
                c.with_alpha(c.alpha() * a)
            }
            WingsPaint::Fixed(c) => c,
            WingsPaint::Adaptive { light, dark: d } => {
                if dark {
                    d
                } else {
                    light
                }
            }
            WingsPaint::Transparent => Color::NONE,
        }
    }

    /// Same paint with an extra alpha multiplier.
    pub fn with_alpha(self, alpha: f32) -> Self {
        match self {
            WingsPaint::Role(r) => WingsPaint::RoleAlpha(r, alpha),
            WingsPaint::RoleAlpha(r, a) => WingsPaint::RoleAlpha(r, a * alpha),
            WingsPaint::Fixed(c) => WingsPaint::Fixed(c.with_alpha(c.alpha() * alpha)),
            WingsPaint::Adaptive { light, dark } => WingsPaint::Adaptive {
                light: light.with_alpha(light.alpha() * alpha),
                dark: dark.with_alpha(dark.alpha() * alpha),
            },
            WingsPaint::Transparent => WingsPaint::Transparent,
        }
    }
}

/// Themed fill color.
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
#[require(BackgroundColor)]
pub struct WingsBackground(pub WingsPaint);

impl WingsBackground {
    /// Fill with a theme role.
    pub fn role(role: ColorRole) -> Self {
        Self(WingsPaint::Role(role))
    }
}

/// Themed text color.
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
#[require(TextColor)]
pub struct WingsForeground(pub WingsPaint);

impl WingsForeground {
    /// Text with a theme role.
    pub fn role(role: ColorRole) -> Self {
        Self(WingsPaint::Role(role))
    }
}

/// Themed border color.
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
#[require(BorderColor)]
pub struct WingsBorderPaint(pub WingsPaint);

/// Content color provided to descendants that carry [`WingsInheritContentColor`].
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct WingsContentColor(pub WingsPaint);

/// Marks a text entity whose color comes from the closest [`WingsContentColor`] (itself or an
/// ancestor). Falls back to [`ColorRole::OnSurface`].
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
#[require(TextColor)]
pub struct WingsInheritContentColor;

/// Interaction overlay (hover, focus, press) blended on top of [`WingsBackground`].
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct WingsStateLayer {
    /// Overlay color.
    pub color: WingsPaint,
    /// Overlay opacity, typically `0.08` hover, `0.10` focus, `0.12` pressed.
    pub opacity: f32,
}

impl Default for WingsStateLayer {
    fn default() -> Self {
        Self {
            color: WingsPaint::Role(ColorRole::OnSurface),
            opacity: 0.0,
        }
    }
}

impl WingsStateLayer {
    /// Standard hover opacity.
    pub const HOVER: f32 = 0.08;
    /// Standard focus opacity.
    pub const FOCUS: f32 = 0.10;
    /// Standard pressed opacity.
    pub const PRESSED: f32 = 0.12;
}

/// Animates color changes caused by state (hover, press, selection) with this spec.
/// Theme switches always use [`WingsMotion::theme_transition`](crate::tokens::WingsMotion).
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct WingsColorTransition(pub AnimationSpec);

impl Default for WingsColorTransition {
    fn default() -> Self {
        Self(AnimationSpec::short())
    }
}

/// Type scale role for a text entity.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[require(TextFont, LineHeight, LetterSpacing)]
pub struct WingsTypeStyle(pub TypeRole);

/// Shape role for a node's corners.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[require(Node)]
pub struct WingsShape(pub ShapeRole);

/// Elevation level (`0..=5`) for a node's shadow.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[require(BoxShadow)]
pub struct WingsElevationLevel(pub u8);

/// Marker added after an entity was painted once; colors of painted entities cross fade on
/// theme switches while fresh entities appear with their final color immediately.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct WingsPainted;

/// Blends `layer` with opacity `amount` over `base` (source over blending).
pub fn overlay(base: Color, layer: Color, amount: f32) -> Color {
    let a = (layer.alpha() * amount).clamp(0.0, 1.0);
    if a <= 0.0 {
        return base;
    }
    let b = base.to_linear();
    let l = layer.to_linear();
    let out_a = a + b.alpha * (1.0 - a);
    if out_a <= 0.0 {
        return Color::NONE;
    }
    let mix = |lc: f32, bc: f32| (lc * a + bc * b.alpha * (1.0 - a)) / out_a;
    Color::LinearRgba(LinearRgba::new(
        mix(l.red, b.red),
        mix(l.green, b.green),
        mix(l.blue, b.blue),
        out_a,
    ))
}

fn multiply_alpha(c: Color, opacity: f32) -> Color {
    if opacity >= 1.0 {
        c
    } else {
        c.with_alpha(c.alpha() * opacity)
    }
}

/// Writes a color either directly or through an animator.
fn apply<P: WingsProperty<Value = Color>>(
    entity: Entity,
    target: &mut Mut<P::Target>,
    animator: Option<Mut<WingsAnimator<P>>>,
    color: Color,
    transition: Option<AnimationSpec>,
    commands: &mut Commands,
) {
    match (transition, animator) {
        (Some(spec), Some(mut a)) => {
            if a.target() != Some(&color) {
                a.request(None, color, spec, Repeat::ONCE);
            }
        }
        (Some(spec), None) => {
            if P::get(target) != color {
                commands
                    .entity(entity)
                    .insert(WingsAnimator::<P>::to(color, spec));
            }
        }
        (None, Some(mut a)) if a.is_active() => {
            a.request(None, color, AnimationSpec::Snap, Repeat::ONCE);
        }
        _ => {
            if P::get(target) != color {
                P::set(target, color);
            }
        }
    }
}

type PaintTargets<'a> = (
    Option<&'a WingsBackground>,
    Option<&'a WingsStateLayer>,
    Option<&'a WingsForeground>,
    Option<&'a WingsBorderPaint>,
    Has<WingsInheritContentColor>,
    Option<&'a WingsColorTransition>,
    Has<WingsPainted>,
    Option<&'a mut BackgroundColor>,
    Option<&'a mut TextColor>,
    Option<&'a mut BorderColor>,
    Option<&'a mut WingsAnimator<BackgroundColorProperty>>,
    Option<&'a mut WingsAnimator<TextColorProperty>>,
    Option<&'a mut WingsAnimator<BorderColorProperty>>,
);

/// Change filters that make an entity dirty for painting.
pub type PaintChanged = Or<(
    Changed<WingsBackground>,
    Changed<WingsStateLayer>,
    Changed<WingsForeground>,
    Changed<WingsBorderPaint>,
    Added<WingsInheritContentColor>,
)>;

/// Change filters that make a whole subtree dirty.
pub type SubtreeChanged = Or<(
    Changed<WingsContentColor>,
    Changed<WingsOpacity>,
    Changed<WingsLocalTheme>,
)>;

/// Resolves paint components into Bevy UI colors. See the [module docs](self).
pub fn paint_system(
    theme: Res<WingsTheme>,
    mut last_dark: Local<Option<bool>>,
    mut changed_messages: MessageWriter<WingsThemeChanged>,
    parents: Query<&ChildOf>,
    children: Query<&Children>,
    locals: Query<&WingsLocalTheme>,
    opacities: Query<&WingsOpacity>,
    contents: Query<&WingsContentColor>,
    dirty_paint: Query<Entity, PaintChanged>,
    dirty_subtree: Query<Entity, SubtreeChanged>,
    all: Query<
        Entity,
        Or<(
            With<WingsBackground>,
            With<WingsForeground>,
            With<WingsBorderPaint>,
            With<WingsInheritContentColor>,
        )>,
    >,
    mut targets: Query<PaintTargets>,
    mut commands: Commands,
) {
    let dark_now = theme.is_dark();
    let theme_changed = theme.is_changed();
    if *last_dark != Some(dark_now) {
        if last_dark.is_some() {
            changed_messages.write(WingsThemeChanged { dark: dark_now });
        }
        *last_dark = Some(dark_now);
    }

    let mut dirty = EntityHashSet::default();
    if theme_changed {
        dirty.extend(all.iter());
    } else {
        dirty.extend(dirty_paint.iter());
        for root in &dirty_subtree {
            dirty.insert(root);
            dirty.extend(children.iter_descendants(root));
        }
    }
    if dirty.is_empty() {
        return;
    }

    let theme_transition = theme.motion.theme_transition;
    for entity in dirty {
        let Ok((
            bg,
            layer,
            fg,
            border,
            inherit,
            transition,
            painted,
            bg_c,
            text_c,
            border_c,
            bg_a,
            text_a,
            border_a,
        )) = targets.get_mut(entity)
        else {
            continue;
        };
        let dark = ancestors(entity, &parents)
            .find_map(|e| locals.get(e).ok())
            .map_or(dark_now, |l| l.dark);
        let scheme = theme.scheme(dark);
        let opacity: f32 = ancestors(entity, &parents)
            .filter_map(|e| opacities.get(e).ok())
            .map(|o| o.clamped())
            .product();

        let spec = if !painted {
            None
        } else if theme_changed {
            theme_transition
        } else {
            transition.map(|t| t.0)
        };

        if let (Some(bg), Some(mut target)) = (bg, bg_c) {
            let mut color = bg.0.resolve(scheme, dark);
            if let Some(layer) = layer {
                color = overlay(color, layer.color.resolve(scheme, dark), layer.opacity);
            }
            let color = multiply_alpha(color, opacity);
            apply::<BackgroundColorProperty>(entity, &mut target, bg_a, color, spec, &mut commands);
        }

        if let Some(mut target) = text_c {
            let paint = match fg {
                Some(fg) => Some(fg.0),
                None if inherit => Some(
                    ancestors(entity, &parents)
                        .find_map(|e| contents.get(e).ok())
                        .map_or(WingsPaint::Role(ColorRole::OnSurface), |c| c.0),
                ),
                None => None,
            };
            if let Some(paint) = paint {
                let color = multiply_alpha(paint.resolve(scheme, dark), opacity);
                apply::<TextColorProperty>(entity, &mut target, text_a, color, spec, &mut commands);
            }
        }

        if let (Some(border), Some(mut target)) = (border, border_c) {
            let color = multiply_alpha(border.0.resolve(scheme, dark), opacity);
            apply::<BorderColorProperty>(entity, &mut target, border_a, color, spec, &mut commands);
        }

        if !painted {
            commands.entity(entity).insert(WingsPainted);
        }
    }
}

/// Applies [`WingsTypeStyle`] roles to text fonts.
pub fn typography_system(
    theme: Res<WingsTheme>,
    mut query: Query<(
        Ref<WingsTypeStyle>,
        &mut TextFont,
        &mut LineHeight,
        &mut LetterSpacing,
    )>,
) {
    let full = theme.is_changed();
    for (role, mut font, mut line, mut spacing) in &mut query {
        if !full && !role.is_changed() {
            continue;
        }
        let style = theme.typography.get(role.0);
        font.font = theme.typography.font.clone();
        font.font_size = FontSize::Px(style.size);
        font.weight = style.weight;
        *line = LineHeight::RelativeToFont(style.line_height);
        *spacing = LetterSpacing::Px(style.letter_spacing);
    }
}

/// Applies [`WingsShape`] roles to node corners.
pub fn shape_system(theme: Res<WingsTheme>, mut query: Query<(Ref<WingsShape>, &mut Node)>) {
    let full = theme.is_changed();
    for (shape, mut node) in &mut query {
        if !full && !shape.is_changed() {
            continue;
        }
        let radius = theme.shapes.radius(shape.0);
        if node.border_radius != radius {
            node.border_radius = radius;
        }
    }
}

/// Applies [`WingsElevationLevel`] to node shadows.
pub fn elevation_system(
    theme: Res<WingsTheme>,
    mut query: Query<(Ref<WingsElevationLevel>, &mut BoxShadow)>,
) {
    let full = theme.is_changed();
    let dark = theme.is_dark();
    let shadow_color = theme.color(ColorRole::Shadow);
    for (level, mut shadow) in &mut query {
        if !full && !level.is_changed() {
            continue;
        }
        *shadow = theme.elevation.shadow(level.0, shadow_color, dark);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Role paints follow the scheme.
    #[test]
    fn resolve_roles() {
        let light = WingsColorScheme::light();
        assert_eq!(
            WingsPaint::Role(ColorRole::Primary).resolve(&light, false),
            light.get(ColorRole::Primary)
        );
        assert_eq!(WingsPaint::Transparent.resolve(&light, false), Color::NONE);
    }

    /// Adaptive paints pick by darkness.
    #[test]
    fn resolve_adaptive() {
        let p = WingsPaint::Adaptive {
            light: Color::WHITE,
            dark: Color::BLACK,
        };
        let s = WingsColorScheme::light();
        assert_eq!(p.resolve(&s, false), Color::WHITE);
        assert_eq!(p.resolve(&s, true), Color::BLACK);
    }

    /// Alpha multipliers multiply.
    #[test]
    fn alpha_multipliers_multiply() {
        let p = WingsPaint::Role(ColorRole::Primary)
            .with_alpha(0.5)
            .with_alpha(0.5);
        assert_eq!(p, WingsPaint::RoleAlpha(ColorRole::Primary, 0.25));
    }

    /// Overlay with zero amount returns the base; full amount returns the layer.
    #[test]
    fn overlay_extremes() {
        let base = Color::WHITE;
        assert_eq!(overlay(base, Color::BLACK, 0.0), base);
        let full = overlay(base, Color::BLACK, 1.0).to_linear();
        assert!(full.red < 1e-6 && (full.alpha - 1.0).abs() < 1e-6);
    }

    /// Overlay on a transparent base yields the layer color with the overlay opacity
    /// (needed for text buttons that have no fill).
    #[test]
    fn overlay_on_transparent() {
        let c = overlay(Color::NONE, Color::WHITE, 0.12).to_linear();
        assert!((c.alpha - 0.12).abs() < 1e-5);
        assert!((c.red - 1.0).abs() < 1e-5);
    }
}
