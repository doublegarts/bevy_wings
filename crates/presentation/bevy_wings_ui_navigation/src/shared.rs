//! Shared element transitions.
//!
//! Tag an element on two destinations with the same [`WingsSharedElement`] and it moves and
//! resizes from one to the other when navigation swaps them, like SwiftUI's
//! `matchedGeometryEffect` or Material's container transform:
//!
//! ```ignore
//! // On the list screen:
//! bsn! { @WingsCard WingsSharedElement::new("note-42") Children [ ... ] }
//! // On the detail screen:
//! bsn! { @WingsCard WingsSharedElement::new("note-42") Children [ ... ] }
//! ```
//!
//! It works with every navigation operation (push, pop, replace, reset) because it only
//! watches which tagged element is visible: when a tag shows up on a new entity, that entity
//! starts where the previous one was and springs into place. Fade transitions between the
//! destinations look best with shared elements.

use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::*;
use bevy::ui::UiGlobalTransform;

use bevy_wings_ui_animation::prelude::{
    AnimationSpec, Repeat, ScaleProperty, SpringSpec, TranslationProperty, WingsAnimateExt,
};

/// Pairs elements across destinations by tag.
#[derive(Component, Debug, Clone, PartialEq, Eq, Hash, Default, Reflect)]
pub struct WingsSharedElement(pub String);

impl WingsSharedElement {
    /// A tag.
    pub fn new(tag: impl Into<String>) -> Self {
        Self(tag.into())
    }
}

/// Where every visible shared element was last frame.
#[derive(Resource, Debug, Default)]
pub struct WingsSharedElementRects {
    last: HashMap<String, (Entity, Rect)>,
    visible: HashSet<Entity>,
}

/// Motion of shared element transitions.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct WingsSharedElementMotion(pub AnimationSpec);

impl Default for WingsSharedElementMotion {
    fn default() -> Self {
        Self(AnimationSpec::spring_with(SpringSpec::from_response(
            0.45, 1.0,
        )))
    }
}

/// The transform that makes a node laid out at `to` appear at `from`: a translation in
/// logical pixels and a scale.
pub fn shared_offset(from: Rect, to: Rect) -> (Vec2, Vec2) {
    let size = to.size().max(Vec2::splat(1.0));
    (from.center() - to.center(), from.size() / size)
}

/// Starts shared element transitions for tags that moved to another entity.
pub fn shared_element_system(
    mut commands: Commands,
    motion: Res<WingsSharedElementMotion>,
    mut rects: ResMut<WingsSharedElementRects>,
    elements: Query<(
        Entity,
        &WingsSharedElement,
        &ComputedNode,
        &UiGlobalTransform,
    )>,
) {
    let mut now: HashMap<String, (Entity, Rect)> = HashMap::default();
    let mut visible = HashSet::default();
    for (entity, tag, node, transform) in &elements {
        if node.size.x <= 0.0 || node.size.y <= 0.0 {
            continue;
        }
        let scale = node.inverse_scale_factor;
        let rect = Rect::from_center_size(transform.translation * scale, node.size * scale);
        visible.insert(entity);
        // Prefer the element that just appeared when two share a tag during a transition.
        let fresh = !rects.visible.contains(&entity);
        match now.get(&tag.0) {
            Some((other, _)) if !fresh || !rects.visible.contains(other) => {}
            _ => {
                now.insert(tag.0.clone(), (entity, rect));
            }
        }
    }
    for (tag, (entity, rect)) in &now {
        if rects.visible.contains(entity) {
            continue;
        }
        let Some((previous, from)) = rects.last.get(tag) else {
            continue;
        };
        if previous == entity {
            continue;
        }
        let (offset, scale) = shared_offset(*from, *rect);
        commands
            .entity(*entity)
            .animate_from_to::<TranslationProperty>(offset, Vec2::ZERO, motion.0, Repeat::ONCE)
            .animate_from_to::<ScaleProperty>(scale, Vec2::ONE, motion.0, Repeat::ONCE);
    }
    // Remember positions, keeping tags that are hidden for a moment (between destinations).
    for (tag, value) in now {
        rects.last.insert(tag, value);
    }
    rects.visible = visible;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The offset brings the new rect onto the old one.
    #[test]
    fn offset_math() {
        let from = Rect::new(0.0, 0.0, 100.0, 50.0);
        let to = Rect::new(100.0, 100.0, 300.0, 200.0);
        let (offset, scale) = shared_offset(from, to);
        assert_eq!(offset, Vec2::new(-150.0, -125.0));
        assert_eq!(scale, Vec2::new(0.5, 0.5));
    }
}
