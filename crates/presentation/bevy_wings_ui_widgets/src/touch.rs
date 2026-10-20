//! Touch input: kinetic scrolling and reliable taps.
//!
//! * **Kinetic scrolling.** Bevy's [`ScrollArea`] scrolls with the mouse wheel and trackpad.
//!   On touch screens a finger drags every scroll area (scroll columns, lists, virtual lists),
//!   and letting go keeps it gliding with the finger's speed, slowing down like iOS and Android
//!   do. Touching a gliding area stops it. A drag that turns into a scroll cancels the tap on
//!   the button under the finger.
//! * **Instant taps.** A tap whose touch starts and ends within one frame (quick taps, and
//!   clicks in a simulator) would leave the widget pressed without activating it; Wings
//!   completes it as a normal tap.

use core::time::Duration;

use bevy::input::touch::Touches;
use bevy::picking::events::{PointerClick, PointerPress};
use bevy::picking::pointer::PointerId;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::ui::{Pressed, UiGlobalTransform};
use bevy::ui_widgets::ScrollArea;

/// Settings of touch scrolling.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct WingsTouchScroll {
    /// Scroll with touch drags.
    pub enabled: bool,
    /// Distance in logical pixels a finger moves before a touch becomes a scroll.
    pub slop: f32,
    /// Share of the speed kept every millisecond while gliding (iOS uses 0.998).
    pub deceleration: f32,
    /// Speed in logical pixels per second below which gliding stops.
    pub min_velocity: f32,
    /// Fastest glide in logical pixels per second.
    pub max_velocity: f32,
}

impl Default for WingsTouchScroll {
    fn default() -> Self {
        Self {
            enabled: true,
            slop: 8.0,
            deceleration: 0.998,
            min_velocity: 12.0,
            max_velocity: 9000.0,
        }
    }
}

/// A scroll area gliding after a flick.
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct WingsScrollMomentum {
    /// Speed in logical pixels per second (positive scrolls further into the content).
    pub velocity: Vec2,
}

/// Speed left after gliding for `seconds` with `deceleration` (kept per millisecond).
pub fn glide_velocity(velocity: f32, seconds: f32, deceleration: f32) -> f32 {
    velocity * deceleration.powf(seconds * 1000.0)
}

/// Total distance a glide starting at `velocity` covers before it stops.
pub fn glide_distance(velocity: f32, deceleration: f32) -> f32 {
    // The integral of v * d^(1000 t) dt from 0 to infinity.
    velocity / (-deceleration.ln() * 1000.0)
}

#[derive(Debug, Clone, Copy)]
struct Drag {
    touch: u64,
    start: Vec2,
    last: Vec2,
    last_move: f32,
    velocity: Vec2,
    area: Option<Entity>,
    axis: Vec2,
    start_scroll: Vec2,
}

/// The touch drag in progress.
#[derive(Debug, Default)]
pub struct TouchDrag {
    drag: Option<Drag>,
}

type AreaData<'a> = (
    Entity,
    &'a Node,
    &'a ComputedNode,
    &'a UiGlobalTransform,
    &'a mut ScrollPosition,
    &'a InheritedVisibility,
);

/// Logical rectangle and maximum scroll offset of a scroll area.
fn area_geometry(computed: &ComputedNode, transform: &UiGlobalTransform) -> (Rect, Vec2) {
    let scale = computed.inverse_scale_factor;
    let size = computed.size() * scale;
    let rect = Rect::from_center_size(transform.translation * scale, size);
    let max = (computed.content_size() * scale - size).max(Vec2::ZERO);
    (rect, max)
}

/// Drags scroll areas with a finger and starts the glide when it lets go.
#[allow(clippy::too_many_arguments)]
pub fn touch_scroll_system(
    mut commands: Commands,
    settings: Res<WingsTouchScroll>,
    time: Res<Time>,
    touches: Option<Res<Touches>>,
    mut areas: Query<AreaData, With<ScrollArea>>,
    pressed: Query<Entity, With<Pressed>>,
    mut state: Local<TouchDrag>,
) {
    let Some(touches) = touches.filter(|_| settings.enabled) else {
        state.drag = None;
        return;
    };
    let now = time.elapsed_secs();
    if state.drag.is_none() {
        let Some(touch) = touches.iter_just_pressed().next() else {
            return;
        };
        let position = touch.position();
        // A finger on a gliding area stops it.
        for (entity, _, computed, transform, _, visible) in &areas {
            let (rect, _) = area_geometry(computed, transform);
            if visible.get() && rect.contains(position) {
                commands.entity(entity).remove::<WingsScrollMomentum>();
            }
        }
        state.drag = Some(Drag {
            touch: touch.id(),
            start: position,
            last: position,
            last_move: now,
            velocity: Vec2::ZERO,
            area: None,
            axis: Vec2::ZERO,
            start_scroll: Vec2::ZERO,
        });
    }
    let Some(mut drag) = state.drag else {
        return;
    };
    let (position, ended) = match touches.get_pressed(drag.touch) {
        Some(t) => (t.position(), false),
        None => (
            touches
                .get_released(drag.touch)
                .map_or(drag.last, |t| t.position()),
            true,
        ),
    };
    let dt = time.delta_secs().max(1e-4);
    let step = position - drag.last;
    if step != Vec2::ZERO {
        // Content moves opposite to scrolling: dragging up scrolls down.
        drag.velocity = drag.velocity * 0.2 + (-step / dt) * 0.8;
        drag.last_move = now;
    }
    drag.last = position;

    if drag.area.is_none() {
        let moved = position - drag.start;
        if ended || moved.length() < settings.slop {
            state.drag = (!ended).then_some(drag);
            return;
        }
        // The innermost visible area under the start that scrolls in the drag's direction.
        let vertical = moved.y.abs() >= moved.x.abs();
        let mut best: Option<(Entity, f32, Vec2, Vec2)> = None;
        for (entity, node, computed, transform, scroll, visible) in &areas {
            let (rect, max) = area_geometry(computed, transform);
            if !visible.get() || !rect.contains(drag.start) {
                continue;
            }
            let axis = if vertical && node.overflow.y == OverflowAxis::Scroll && max.y > 0.0 {
                Vec2::Y
            } else if !vertical && node.overflow.x == OverflowAxis::Scroll && max.x > 0.0 {
                Vec2::X
            } else {
                continue;
            };
            let size = rect.size().x * rect.size().y;
            if best.is_none_or(|(_, s, _, _)| size < s) {
                best = Some((entity, size, axis, scroll.0));
            }
        }
        let Some((area, _, axis, start_scroll)) = best else {
            // Nothing scrolls this way (a horizontal swipe on a vertical list, for example).
            state.drag = None;
            return;
        };
        drag.area = Some(area);
        drag.axis = axis;
        // Scrolling from where the finger is now, without a jump.
        drag.start = position;
        drag.start_scroll = start_scroll;
        // The touch is a scroll now, not a tap on what is under the finger.
        for entity in &pressed {
            commands.entity(entity).remove::<Pressed>();
        }
    }

    let Some(area) = drag.area else {
        return;
    };
    let Ok((_, _, computed, transform, mut scroll, _)) = areas.get_mut(area) else {
        state.drag = None;
        return;
    };
    let (_, max) = area_geometry(computed, transform);
    let target = (drag.start_scroll - (position - drag.start)) * drag.axis
        + scroll.0 * (Vec2::ONE - drag.axis);
    let target = target.clamp(Vec2::ZERO, max);
    if scroll.0 != target {
        scroll.0 = target;
    }
    if ended {
        state.drag = None;
        // A finger that stopped before letting go does not throw the content.
        let still = now - drag.last_move > 0.06;
        let velocity = if still {
            Vec2::ZERO
        } else {
            (drag.velocity * drag.axis).clamp_length_max(settings.max_velocity)
        };
        if velocity.length() > settings.min_velocity {
            commands
                .entity(area)
                .insert(WingsScrollMomentum { velocity });
        }
    } else {
        state.drag = Some(drag);
    }
}

/// Moves gliding scroll areas and slows them down.
pub fn scroll_momentum_system(
    mut commands: Commands,
    settings: Res<WingsTouchScroll>,
    time: Res<Time>,
    mut areas: Query<(
        Entity,
        &mut WingsScrollMomentum,
        &ComputedNode,
        &mut ScrollPosition,
    )>,
) {
    let dt = time.delta_secs();
    for (entity, mut momentum, computed, mut scroll) in &mut areas {
        let scale = computed.inverse_scale_factor;
        let max = (computed.content_size() * scale - computed.size() * scale).max(Vec2::ZERO);
        let next = (scroll.0 + momentum.velocity * dt).clamp(Vec2::ZERO, max);
        // Hitting an end stops the glide on that axis.
        let mut velocity = Vec2::new(
            glide_velocity(momentum.velocity.x, dt, settings.deceleration),
            glide_velocity(momentum.velocity.y, dt, settings.deceleration),
        );
        if next.x <= 0.0 || next.x >= max.x {
            velocity.x = 0.0;
        }
        if next.y <= 0.0 || next.y >= max.y {
            velocity.y = 0.0;
        }
        scroll.0 = next;
        if velocity.length() < settings.min_velocity {
            commands.entity(entity).remove::<WingsScrollMomentum>();
        } else {
            momentum.velocity = velocity;
        }
    }
}

/// The last press of every touch, to complete taps that started and ended in one frame.
#[derive(Resource, Debug, Default)]
pub struct WingsTouchPresses(HashMap<u64, PointerPress>);

/// Observer: remembers where each touch pressed.
pub fn record_touch_press(press: On<PointerPress>, mut presses: ResMut<WingsTouchPresses>) {
    if let PointerId::Touch(id) = press.pointer.id
        && press.event_target() == press.original_event_target()
    {
        presses.0.insert(id, press.event().clone());
    }
}

/// Completes taps whose touch started and ended in the same frame: Bevy picking presses the
/// widget but never sees the release over it, so the widget would stay pressed. Wings sends the
/// click and releases the widget.
pub fn complete_instant_taps(
    mut commands: Commands,
    touches: Option<Res<Touches>>,
    mut presses: ResMut<WingsTouchPresses>,
    pressed: Query<Entity, Added<Pressed>>,
) {
    let Some(touches) = touches else {
        return;
    };
    for touch in touches.iter_just_released() {
        let id = touch.id();
        let Some(press) = presses.0.remove(&id) else {
            continue;
        };
        if !touches.just_pressed(id) {
            continue;
        }
        commands.trigger(PointerClick {
            entity: press.entity,
            pointer: press.pointer.clone(),
            button: press.button,
            hit: press.hit.clone(),
            duration: Duration::ZERO,
            count: 1,
        });
        for entity in &pressed {
            commands.entity(entity).remove::<Pressed>();
        }
    }
    for touch in touches.iter_just_canceled() {
        presses.0.remove(&touch.id());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A glide slows down like iOS and covers a finite distance.
    #[test]
    fn glide_physics() {
        let v = glide_velocity(1000.0, 1.0, 0.998);
        assert!((v - 1000.0 * 0.998f32.powf(1000.0)).abs() < 1e-3);
        assert!(
            v < 140.0 && v > 130.0,
            "about 13.5% of the speed is left after a second: {v}"
        );
        let d = glide_distance(1000.0, 0.998);
        assert!(
            d > 490.0 && d < 510.0,
            "a 1000 pt/s flick glides about 500 pt: {d}"
        );
        // Stepping frame by frame lands near the integral.
        let (mut pos, mut vel) = (0.0f32, 1000.0f32);
        for _ in 0..600 {
            pos += vel / 60.0;
            vel = glide_velocity(vel, 1.0 / 60.0, 0.998);
        }
        assert!((pos - d).abs() / d < 0.05, "{pos} vs {d}");
    }
}
