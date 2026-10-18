//! [`WingsGlass`]: Liquid Glass on any node.
//!
//! On the overlay layer (floating chrome, see [`overlay`](crate::overlay)) and with the
//! `shaders` feature, glass shows the blurred, lensed content behind it through
//! `WingsLiquidGlassMaterial`. Everywhere else (content layer, headless apps, `shaders` off,
//! [`WingsGlassSettings::reduce_transparency`]) it falls back to a translucent fill with a
//! hairline border, which is what the platforms do when transparency is reduced.

use bevy::prelude::*;
use bevy::ui::Pressed;

use bevy_wings_ui_theme::prelude::{
    ColorRole, WingsBackground, WingsBorderPaint, WingsPaint, WingsTheme,
};

use crate::overlay::WingsOverlayRoot;

/// Kind of glass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WingsGlassStyle {
    /// Frosted glass for bars and panels.
    #[default]
    Regular,
    /// Barely blurred glass over rich media.
    Clear,
    /// Glass tinted with a role (prominent glass buttons).
    Tinted(ColorRole),
}

/// Makes a node Liquid Glass. Owns the node's background and border paint while present.
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
#[require(WingsBackground, WingsBorderPaint)]
pub struct WingsGlass {
    /// Kind of glass.
    pub style: WingsGlassStyle,
    /// Glows while pressed.
    pub interactive: bool,
    /// Draws the rim light (off for full width bars).
    pub rim: bool,
}

impl WingsGlass {
    /// Regular glass with a rim.
    pub const REGULAR: Self = Self {
        style: WingsGlassStyle::Regular,
        interactive: false,
        rim: true,
    };

    /// Glass of a style.
    pub fn new(style: WingsGlassStyle) -> Self {
        Self {
            style,
            ..Self::REGULAR
        }
    }

    /// Glows while pressed.
    pub fn interactive(mut self) -> Self {
        self.interactive = true;
        self
    }

    /// Without the rim light.
    pub fn without_rim(mut self) -> Self {
        self.rim = false;
        self
    }

    /// The translucent fill used when real glass is not available.
    pub fn fallback_fill(&self) -> WingsPaint {
        match self.style {
            WingsGlassStyle::Regular => {
                WingsPaint::RoleAlpha(ColorRole::SurfaceContainerLowest, 0.86)
            }
            WingsGlassStyle::Clear => {
                WingsPaint::RoleAlpha(ColorRole::SurfaceContainerLowest, 0.45)
            }
            WingsGlassStyle::Tinted(role) => WingsPaint::Role(role),
        }
    }
}

/// Global glass settings.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct WingsGlassSettings {
    /// Real glass where possible. When off every glass node uses its fallback fill.
    pub enabled: bool,
    /// Prefer opaque fills (the accessibility setting of Apple platforms).
    pub reduce_transparency: bool,
}

impl Default for WingsGlassSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            reduce_transparency: false,
        }
    }
}

/// Glass rendered with the real material.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct WingsGlassRendered;

/// True when `entity` sits under an overlay root.
#[cfg(feature = "shaders")]
fn on_overlay(
    entity: Entity,
    parents: &Query<&ChildOf>,
    roots: &Query<(), With<WingsOverlayRoot>>,
) -> bool {
    bevy_wings_ui_core::hierarchy::ancestors(entity, parents).any(|a| roots.contains(a))
}

/// Applies glass: the material on the overlay layer, the fallback elsewhere.
#[cfg_attr(not(feature = "shaders"), allow(unused_variables))]
pub fn glass_system(
    mut commands: Commands,
    theme: Res<WingsTheme>,
    settings: Res<WingsGlassSettings>,
    parents: Query<&ChildOf>,
    roots: Query<(), With<WingsOverlayRoot>>,
    mut glass: Query<(
        Entity,
        Ref<WingsGlass>,
        Option<Ref<ChildOf>>,
        Has<WingsGlassRendered>,
        Has<Pressed>,
        &mut WingsBackground,
        &mut WingsBorderPaint,
    )>,
    #[cfg(feature = "shaders")] backdrop: Option<
        Res<bevy_wings_ui_shaders::backdrop::WingsBackdrop>,
    >,
    #[cfg(feature = "shaders")] materials: Option<
        ResMut<Assets<bevy_wings_ui_shaders::WingsLiquidGlassMaterial>>,
    >,
    #[cfg(feature = "shaders")] handles: Query<
        &MaterialNode<bevy_wings_ui_shaders::WingsLiquidGlassMaterial>,
    >,
    mut removed_pressed: RemovedComponents<Pressed>,
    pressed_now: Query<(), (With<WingsGlass>, Added<Pressed>)>,
) {
    #[cfg(feature = "shaders")]
    let mut materials = materials;
    #[cfg(feature = "shaders")]
    let backdrop_changed = backdrop.as_ref().is_some_and(|b| b.is_changed());
    #[cfg(not(feature = "shaders"))]
    let backdrop_changed = false;
    let released: Vec<Entity> = removed_pressed.read().collect();
    let force = theme.is_changed() || settings.is_changed() || backdrop_changed;
    let dark = theme.is_dark();
    for (entity, g, parent, rendered, pressed, mut bg, mut border) in &mut glass {
        let touched = force
            || g.is_changed()
            || parent.is_some_and(|p| p.is_changed())
            || pressed_now.contains(entity)
            || released.contains(&entity);
        if !touched {
            continue;
        }
        #[allow(unused_mut)]
        let mut real = false;
        #[cfg(feature = "shaders")]
        if let (Some(backdrop), Some(materials)) = (backdrop.as_ref(), materials.as_mut())
            && settings.enabled
            && !settings.reduce_transparency
            && on_overlay(entity, &parents, &roots)
        {
            use bevy_wings_ui_shaders::WingsLiquidGlassMaterial as M;
            let mut m = match g.style {
                WingsGlassStyle::Regular => M::regular(backdrop, dark),
                WingsGlassStyle::Clear => M::clear(backdrop, dark),
                WingsGlassStyle::Tinted(role) => M::tinted(backdrop, theme.color(role), dark),
            };
            if !g.rim {
                m.highlight.alpha = 0.0;
                m.refraction = 0.0;
            }
            m.press = if g.interactive && pressed { 1.0 } else { 0.0 };
            let updated = match handles.get(entity) {
                Ok(handle) => match materials.get_mut(&handle.0) {
                    Some(mut existing) => {
                        *existing = m.clone();
                        true
                    }
                    None => false,
                },
                Err(_) => false,
            };
            if !updated {
                commands
                    .entity(entity)
                    .insert((MaterialNode(materials.add(m)), WingsGlassRendered));
            }
            bg.set_if_neq(WingsBackground(WingsPaint::Transparent));
            border.set_if_neq(WingsBorderPaint(WingsPaint::Transparent));
            real = true;
        }
        if !real {
            if rendered {
                #[cfg(feature = "shaders")]
                commands
                    .entity(entity)
                    .remove::<MaterialNode<bevy_wings_ui_shaders::WingsLiquidGlassMaterial>>();
                commands.entity(entity).remove::<WingsGlassRendered>();
            }
            bg.set_if_neq(WingsBackground(g.fallback_fill()));
            border.set_if_neq(WingsBorderPaint(if g.rim {
                WingsPaint::Role(ColorRole::OutlineVariant)
            } else {
                WingsPaint::Transparent
            }));
        }
    }
    let _ = (&parents, &roots, dark);
}

/// Removes the material when glass is taken off a node.
pub fn glass_removed(
    mut commands: Commands,
    mut removed: RemovedComponents<WingsGlass>,
    rendered: Query<(), With<WingsGlassRendered>>,
) {
    for entity in removed.read() {
        if rendered.contains(entity) {
            #[cfg(feature = "shaders")]
            commands
                .entity(entity)
                .remove::<MaterialNode<bevy_wings_ui_shaders::WingsLiquidGlassMaterial>>();
            commands.entity(entity).remove::<WingsGlassRendered>();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fallback fills stay translucent except tinted glass.
    #[test]
    fn fallbacks() {
        assert!(matches!(
            WingsGlass::REGULAR.fallback_fill(),
            WingsPaint::RoleAlpha(_, a) if a < 1.0
        ));
        assert_eq!(
            WingsGlass::new(WingsGlassStyle::Tinted(ColorRole::Primary)).fallback_fill(),
            WingsPaint::Role(ColorRole::Primary)
        );
    }
}
