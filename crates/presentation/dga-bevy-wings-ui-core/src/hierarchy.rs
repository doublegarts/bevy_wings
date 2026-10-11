//! Hierarchy helpers.
//!
//! Many Wings features are "contextual": a text inherits the content color of its button, a
//! binding talks to the view model owned by the closest screen, a widget reads the theme
//! override of its container. These helpers walk up the [`ChildOf`] relationship.

use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::hierarchy::ChildOf;
use bevy::ecs::query::{QueryData, QueryFilter};
use bevy::ecs::system::Query;

/// Iterates over `entity` and then all of its ancestors, closest first.
pub fn ancestors<'a>(
    entity: Entity,
    parents: &'a Query<&ChildOf>,
) -> impl Iterator<Item = Entity> + 'a {
    let mut next = Some(entity);
    core::iter::from_fn(move || {
        let current = next?;
        next = parents.get(current).ok().map(ChildOf::parent);
        Some(current)
    })
}

/// Returns the closest entity (starting with `entity` itself) that matches `query`.
///
/// The walk is bounded by the hierarchy depth, which keeps it cheap for typical UI trees.
pub fn find_ancestor_with<D: QueryData, F: QueryFilter>(
    entity: Entity,
    parents: &Query<&ChildOf>,
    query: &Query<D, F>,
) -> Option<Entity> {
    ancestors(entity, parents).find(|e| query.contains(*e))
}

/// Marker trait implemented by every component; handy in generic helper signatures.
pub trait AnyComponent: Component {}
impl<C: Component> AnyComponent for C {}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    use bevy::prelude::*;

    #[derive(Component)]
    struct Marker;

    /// `ancestors` yields the entity first and then its parents up to the root.
    #[test]
    fn ancestors_walks_to_root() {
        let mut world = World::new();
        let root = world.spawn_empty().id();
        let mid = world.spawn(ChildOf(root)).id();
        let leaf = world.spawn(ChildOf(mid)).id();
        let chain = world
            .run_system_once(move |q: Query<&ChildOf>| ancestors(leaf, &q).collect::<Vec<_>>())
            .unwrap();
        assert_eq!(chain, vec![leaf, mid, root]);
    }

    /// `find_ancestor_with` finds the closest marked ancestor.
    #[test]
    fn finds_closest_marked_ancestor() {
        let mut world = World::new();
        let root = world.spawn(Marker).id();
        let mid = world.spawn((ChildOf(root), Marker)).id();
        let leaf = world.spawn(ChildOf(mid)).id();
        let found = world
            .run_system_once(move |p: Query<&ChildOf>, m: Query<(), With<Marker>>| {
                find_ancestor_with(leaf, &p, &m)
            })
            .unwrap();
        assert_eq!(found, Some(mid));
    }

    /// Without any marked ancestor the helper returns `None`.
    #[test]
    fn returns_none_without_match() {
        let mut world = World::new();
        let leaf = world.spawn_empty().id();
        let found = world
            .run_system_once(move |p: Query<&ChildOf>, m: Query<(), With<Marker>>| {
                find_ancestor_with(leaf, &p, &m)
            })
            .unwrap();
        assert_eq!(found, None);
    }
}
