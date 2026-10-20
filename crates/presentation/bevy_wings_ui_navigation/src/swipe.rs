//! Interactive edge swipe back, as on iOS.
//!
//! A finger that starts at the leading edge of the screen (the left edge, or the right edge in
//! right to left layouts) and moves across drags the current screen with it and uncovers the
//! previous one, which follows with a parallax offset. Letting go past a third of the width,
//! or with a quick flick, goes back; otherwise both screens spring back into place.
//!
//! The gesture is on by default on iOS, where the system has no back button. Android's back
//! button and back gesture arrive as system back presses instead (see
//! [`WingsBack`](bevy_wings_ui_core::back::WingsBack)). For previews on a desktop, enable it
//! with the mouse:
//!
//! ```ignore
//! app.insert_resource(WingsEdgeSwipeBack { enabled: true, mouse: true, ..default() });
//! ```

use bevy::input::ButtonInput;
use bevy::input::mouse::MouseButton;
use bevy::input::touch::Touches;
use bevy::prelude::*;
use bevy::ui::{Display, Node, Pressed, UiTransform, Val2};
use bevy::window::PrimaryWindow;

use bevy_wings_ui_animation::prelude::{
    AnimationSpec, OpacityProperty, SpringSpec, TranslationProperty, WingsAnimateExt,
};
use bevy_wings_ui_core::WingsOpacity;
use bevy_wings_ui_responsive::{WingsLayoutDirection, WingsViewport};

use crate::back_stack::{NavKey, WingsBackStack};
use crate::{WingsNavDisplayState, WingsNavEntry, WingsNavExiting};

/// Settings of the edge swipe back gesture.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct WingsEdgeSwipeBack {
    /// Recognize the gesture (on by default on iOS).
    pub enabled: bool,
    /// Width of the edge area where the gesture starts, in logical pixels.
    pub edge: f32,
    /// Also follow mouse drags (for previews on a desktop).
    pub mouse: bool,
    /// Share of the width past which letting go goes back.
    pub commit_fraction: f32,
    /// Speed (logical pixels per second) of a flick that goes back however short it is.
    pub commit_velocity: f32,
}

impl Default for WingsEdgeSwipeBack {
    fn default() -> Self {
        Self {
            enabled: cfg!(target_os = "ios"),
            edge: 24.0,
            mouse: false,
            commit_fraction: 0.35,
            commit_velocity: 500.0,
        }
    }
}

/// An entry uncovered by a swipe: going back continues its motion from where the finger left
/// it instead of playing the pop transition from the start.
#[derive(Component, Debug, Clone, Copy)]
pub struct WingsNavRevealed;

/// An entry swiped away: going back slides it this far (the screen width) and despawns it.
#[derive(Component, Debug, Clone, Copy)]
pub struct WingsNavSwipedAway(pub f32);

/// How far the uncovered screen sits behind, as a share of the width (the iOS parallax).
pub const PARALLAX: f32 = 0.3;

/// The motion that finishes or cancels a swipe.
pub fn settle_spec() -> AnimationSpec {
    AnimationSpec::spring_with(SpringSpec::from_response(0.35, 1.0))
}

/// Progress `0..1` of a swipe that moved `distance` across a screen `width` wide.
pub fn swipe_progress(distance: f32, width: f32) -> f32 {
    (distance / width.max(1.0)).clamp(0.0, 1.0)
}

/// True when letting go at `progress` with `velocity` goes back.
pub fn swipe_commits(progress: f32, velocity: f32, settings: &WingsEdgeSwipeBack) -> bool {
    progress >= settings.commit_fraction
        || (velocity >= settings.commit_velocity && progress > 0.02)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SwipePointer {
    Touch(u64),
    Mouse,
}

#[derive(Debug, Clone, Copy)]
struct Swipe {
    pointer: SwipePointer,
    start: Vec2,
    last_x: f32,
    velocity: f32,
    engaged: bool,
    top: Entity,
    below: Entity,
    width: f32,
    dir: f32,
}

/// The swipe in progress.
#[derive(Debug, Default)]
pub struct SwipeTracker {
    active: Option<Swipe>,
}

/// Follows edge swipes and goes back when one completes.
#[allow(clippy::too_many_arguments)]
pub fn edge_swipe_back<K: NavKey>(
    mut commands: Commands,
    settings: Res<WingsEdgeSwipeBack>,
    time: Res<Time>,
    viewport: Res<WingsViewport>,
    direction: Option<Res<WingsLayoutDirection>>,
    touches: Option<Res<Touches>>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut stack: ResMut<WingsBackStack<K>>,
    displays: Query<&WingsNavDisplayState<K>>,
    mut entries: Query<(&mut Node, &mut UiTransform, &mut WingsOpacity), With<WingsNavEntry<K>>>,
    pressed: Query<Entity, With<Pressed>>,
    mut tracker: Local<SwipeTracker>,
) {
    if !settings.enabled {
        tracker.active = None;
        return;
    }
    let cursor = windows.single().ok().and_then(Window::cursor_position);
    let mouse_on = settings.mouse && mouse.is_some();
    let Some(mut swipe) = tracker.active else {
        // A new swipe starts at the leading edge while there is somewhere to go back to.
        let start = touches
            .as_ref()
            .and_then(|t| t.iter_just_pressed().next())
            .map(|t| (SwipePointer::Touch(t.id()), t.position()))
            .or_else(|| {
                let down = mouse_on
                    && mouse
                        .as_ref()
                        .is_some_and(|m| m.just_pressed(MouseButton::Left));
                cursor.filter(|_| down).map(|c| (SwipePointer::Mouse, c))
            });
        let Some((pointer, position)) = start else {
            return;
        };
        let width = viewport.width;
        let dir = if direction.is_some_and(|d| *d == WingsLayoutDirection::RightToLeft) {
            -1.0
        } else {
            1.0
        };
        let at_edge = if dir > 0.0 {
            position.x <= settings.edge
        } else {
            position.x >= width - settings.edge
        };
        if !at_edge || !stack.can_pop() {
            return;
        }
        // Only a single pane showing the top destination slides; the one below must exist.
        let n = stack.len();
        let pair = displays.iter().find_map(|d| {
            (d.visible == [n - 1]).then_some(())?;
            Some((
                d.entities.get(n - 1).copied()??,
                d.entities.get(n - 2).copied()??,
            ))
        });
        let Some((top, below)) = pair else {
            return;
        };
        tracker.active = Some(Swipe {
            pointer,
            start: position,
            last_x: position.x,
            velocity: 0.0,
            engaged: false,
            top,
            below,
            width,
            dir,
        });
        return;
    };

    // Where the pointer is now, and whether it was let go or the system took it away.
    let (position, ended, canceled) = match swipe.pointer {
        SwipePointer::Touch(id) => {
            let Some(t) = touches.as_ref() else {
                tracker.active = None;
                return;
            };
            if let Some(touch) = t.get_pressed(id) {
                (touch.position(), false, false)
            } else if let Some(touch) = t.get_released(id) {
                (touch.position(), true, false)
            } else {
                (Vec2::new(swipe.last_x, swipe.start.y), true, true)
            }
        }
        SwipePointer::Mouse => {
            let position = cursor.unwrap_or(Vec2::new(swipe.last_x, swipe.start.y));
            let held = mouse.as_ref().is_some_and(|m| m.pressed(MouseButton::Left));
            (position, !held, false)
        }
    };
    let dx = (position.x - swipe.start.x) * swipe.dir;
    let dy = (position.y - swipe.start.y).abs();
    let step = (position.x - swipe.last_x) * swipe.dir / time.delta_secs().max(1e-4);
    swipe.velocity = swipe.velocity * 0.4 + step * 0.6;
    swipe.last_x = position.x;

    if !swipe.engaged {
        if ended || (dy > 12.0 && dy > dx) {
            // A tap at the edge or a vertical scroll, not a swipe.
            tracker.active = None;
            return;
        }
        if dx <= 10.0 || dx <= dy {
            tracker.active = Some(swipe);
            return;
        }
        swipe.engaged = true;
        // The press under the finger is no longer a tap.
        for entity in &pressed {
            commands.entity(entity).remove::<Pressed>();
        }
        for entity in [swipe.top, swipe.below] {
            commands
                .entity(entity)
                .remove::<WingsNavExiting>()
                .stop_animation::<TranslationProperty>()
                .stop_animation::<OpacityProperty>();
        }
        if let Ok((mut node, _, mut opacity)) = entries.get_mut(swipe.below) {
            node.display = Display::Flex;
            *opacity = WingsOpacity::OPAQUE;
        }
    }

    let progress = swipe_progress(dx, swipe.width);
    if ended || canceled {
        tracker.active = None;
        if !canceled && swipe_commits(progress, swipe.velocity, &settings) && stack.can_pop() {
            commands.entity(swipe.below).insert(WingsNavRevealed);
            commands
                .entity(swipe.top)
                .insert(WingsNavSwipedAway(swipe.width * swipe.dir));
            stack.pop();
        } else {
            commands
                .entity(swipe.top)
                .animate::<TranslationProperty>(Vec2::ZERO, settle_spec());
            commands
                .entity(swipe.below)
                .insert(WingsNavExiting { despawn: false })
                .animate::<TranslationProperty>(
                    Vec2::new(-PARALLAX * swipe.width * swipe.dir, 0.0),
                    settle_spec(),
                );
        }
        return;
    }

    if let Ok((_, mut transform, _)) = entries.get_mut(swipe.top) {
        transform.translation = Val2::px(dx.clamp(0.0, swipe.width) * swipe.dir, 0.0);
    }
    if let Ok((_, mut transform, _)) = entries.get_mut(swipe.below) {
        let behind = -PARALLAX * swipe.width * (1.0 - progress) * swipe.dir;
        transform.translation = Val2::px(behind, 0.0);
    }
    tracker.active = Some(swipe);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Long drags and quick flicks go back; short slow drags spring back.
    #[test]
    fn commit_rules() {
        let s = WingsEdgeSwipeBack::default();
        assert!(swipe_commits(swipe_progress(200.0, 400.0), 0.0, &s));
        assert!(swipe_commits(swipe_progress(40.0, 400.0), 900.0, &s));
        assert!(!swipe_commits(swipe_progress(60.0, 400.0), 100.0, &s));
        assert_eq!(swipe_progress(-30.0, 400.0), 0.0);
        assert_eq!(swipe_progress(900.0, 400.0), 1.0);
    }
}
