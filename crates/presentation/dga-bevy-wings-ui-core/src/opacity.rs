//! Subtree opacity.
//!
//! Bevy UI nodes do not have an opacity property. Wings adds [`WingsOpacity`]: a multiplier in
//! the `0.0..=1.0` range that applies to the entity and all of its descendants. The theme
//! painter multiplies the alpha of every painted color by the product of all
//! [`WingsOpacity`] values found on the entity and its ancestors, and the animation crate can
//! tween it to build fade transitions.

use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::hierarchy::ChildOf;
use bevy::ecs::system::Query;
use bevy::reflect::Reflect;

/// Opacity multiplier for an entity and its whole subtree.
#[derive(Component, Debug, Clone, Copy, PartialEq, Reflect)]
pub struct WingsOpacity(pub f32);

impl Default for WingsOpacity {
    fn default() -> Self {
        Self(1.0)
    }
}

impl WingsOpacity {
    /// Fully opaque.
    pub const OPAQUE: Self = Self(1.0);
    /// Fully transparent.
    pub const TRANSPARENT: Self = Self(0.0);

    /// Returns the value clamped to `0.0..=1.0`.
    pub fn clamped(self) -> f32 {
        self.0.clamp(0.0, 1.0)
    }
}

/// Computes the effective opacity of `entity`: the product of every [`WingsOpacity`] on the
/// entity and its ancestors. Entities without any opacity component are fully opaque.
pub fn effective_opacity(
    entity: Entity,
    parents: &Query<&ChildOf>,
    opacities: &Query<&WingsOpacity>,
) -> f32 {
    crate::hierarchy::ancestors(entity, parents)
        .filter_map(|e| opacities.get(e).ok())
        .map(|o| o.clamped())
        .product()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    use bevy::prelude::*;

    /// Opacity multiplies down the hierarchy.
    #[test]
    fn opacity_multiplies_through_ancestors() {
        let mut world = World::new();
        let root = world.spawn(WingsOpacity(0.5)).id();
        let mid = world.spawn((ChildOf(root), WingsOpacity(0.5))).id();
        let leaf = world.spawn(ChildOf(mid)).id();
        let v = world
            .run_system_once(move |p: Query<&ChildOf>, o: Query<&WingsOpacity>| {
                effective_opacity(leaf, &p, &o)
            })
            .unwrap();
        assert!((v - 0.25).abs() < 1e-6);
    }

    /// Out of range values are clamped.
    #[test]
    fn clamps_out_of_range() {
        assert_eq!(WingsOpacity(2.0).clamped(), 1.0);
        assert_eq!(WingsOpacity(-1.0).clamped(), 0.0);
        assert_eq!(WingsOpacity::default(), WingsOpacity::OPAQUE);
    }
}
