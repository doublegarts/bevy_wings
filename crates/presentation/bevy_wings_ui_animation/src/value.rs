//! The animation state machine: [`WingsAnimatedValue`].
//!
//! [`WingsAnimatedValue`] is a plain struct (not a component) that owns a value and moves it
//! towards a target according to an [`AnimationSpec`]. It is used internally by the
//! component based animators, and it is also handy inside your own systems or view models
//! whenever you need "animate this number to that number" semantics.

use crate::lerp::WingsLerp;
use crate::spec::{AnimationSpec, Repeat, RepeatMode};

/// Result of a single [`WingsAnimatedValue::tick`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TickResult {
    /// The value changed during this tick.
    pub changed: bool,
    /// The animation completed during this tick.
    pub finished: bool,
}

/// Maximum integration step for springs, in seconds. Large frame times are split into
/// sub steps so springs stay stable even when the app hitches.
const SPRING_MAX_STEP: f32 = 1.0 / 240.0;

/// A value that animates towards a target.
///
/// ```
/// use bevy_wings_ui_animation::prelude::*;
///
/// let mut v = WingsAnimatedValue::new(0.0f32);
/// v.animate_to(10.0, AnimationSpec::tween(1.0).with_easing(WingsEasing::Linear));
/// v.tick(0.5);
/// assert!((v.value() - 5.0).abs() < 1e-4);
/// v.tick(0.5);
/// assert!(!v.is_running());
/// assert_eq!(*v.value(), 10.0);
/// ```
#[derive(Debug, Clone)]
pub struct WingsAnimatedValue<T: WingsLerp> {
    from: T,
    to: T,
    current: T,
    spec: AnimationSpec,
    repeat: Repeat,
    elapsed: f32,
    progress: f32,
    velocity: f32,
    iteration: u32,
    running: bool,
}

impl<T: WingsLerp> WingsAnimatedValue<T> {
    /// Creates an idle animated value.
    pub fn new(value: T) -> Self {
        Self {
            from: value.clone(),
            to: value.clone(),
            current: value,
            spec: AnimationSpec::Snap,
            repeat: Repeat::ONCE,
            elapsed: 0.0,
            progress: 1.0,
            velocity: 0.0,
            iteration: 0,
            running: false,
        }
    }

    /// The current value.
    pub fn value(&self) -> &T {
        &self.current
    }

    /// The value the animation is heading to.
    pub fn target(&self) -> &T {
        &self.to
    }

    /// True while the animation is in progress (including the start delay).
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Spring velocity in progress units per second (zero for tweens).
    pub fn velocity(&self) -> f32 {
        self.velocity
    }

    /// Number of completed iterations of the current animation.
    pub fn iteration(&self) -> u32 {
        self.iteration
    }

    /// Jumps to `value` and stops any running animation.
    pub fn snap_to(&mut self, value: T) {
        self.from = value.clone();
        self.to = value.clone();
        self.current = value;
        self.running = false;
        self.velocity = 0.0;
        self.progress = 1.0;
    }

    /// Starts animating from the current value to `target`, playing once.
    pub fn animate_to(&mut self, target: T, spec: AnimationSpec) {
        self.animate_to_with_repeat(target, spec, Repeat::ONCE);
    }

    /// Starts animating from the current value to `target` with repetition.
    ///
    /// When a spring is retargeted mid flight its velocity is preserved (rescaled to the new
    /// distance) so the motion stays continuous.
    pub fn animate_to_with_repeat(&mut self, target: T, spec: AnimationSpec, repeat: Repeat) {
        let old_distance = self.current.wings_distance(&self.to);
        let new_distance = self.current.wings_distance(&target);
        let keep_velocity = matches!(spec, AnimationSpec::Spring { .. }) && self.running;
        self.velocity = if keep_velocity && new_distance > 1e-6 {
            self.velocity * (old_distance / new_distance)
        } else {
            0.0
        };
        self.from = self.current.clone();
        self.to = target;
        self.spec = spec;
        self.repeat = repeat;
        self.elapsed = 0.0;
        self.progress = 0.0;
        self.iteration = 0;
        self.running = true;
        if matches!(spec, AnimationSpec::Snap) {
            self.current = self.to.clone();
            self.running = false;
            self.progress = 1.0;
        }
    }

    /// Animates from an explicit start value to `target`.
    pub fn animate_from_to(&mut self, from: T, target: T, spec: AnimationSpec, repeat: Repeat) {
        self.current = from;
        self.running = false;
        self.velocity = 0.0;
        self.animate_to_with_repeat(target, spec, repeat);
    }

    /// Advances the animation by `dt` seconds.
    pub fn tick(&mut self, dt: f32) -> TickResult {
        if !self.running {
            return TickResult::default();
        }
        self.elapsed += dt.max(0.0);
        let delay = self.spec.delay();
        if self.elapsed < delay {
            return TickResult::default();
        }
        match self.spec {
            AnimationSpec::Snap => {
                self.current = self.to.clone();
                self.running = false;
                TickResult {
                    changed: true,
                    finished: true,
                }
            }
            AnimationSpec::Tween {
                duration, easing, ..
            } => {
                let duration = duration.max(1e-6);
                let mut active = self.elapsed - delay;
                loop {
                    if active < duration {
                        let p = easing.ease(active / duration);
                        self.current = self.from.wings_lerp(&self.to, p);
                        return TickResult {
                            changed: true,
                            finished: false,
                        };
                    }
                    active -= duration;
                    if self.complete_iteration() {
                        return TickResult {
                            changed: true,
                            finished: true,
                        };
                    }
                    self.elapsed = delay + active;
                }
            }
            AnimationSpec::Spring { spring, .. } => {
                let mut remaining = dt.min(self.elapsed - delay).max(0.0);
                let k = spring.stiffness;
                let c = spring.damping();
                while remaining > 0.0 {
                    let h = remaining.min(SPRING_MAX_STEP);
                    let accel = -k * (self.progress - 1.0) - c * self.velocity;
                    self.velocity += accel * h;
                    self.progress += self.velocity * h;
                    remaining -= h;
                    let settled = (1.0 - self.progress).abs() < spring.threshold
                        && self.velocity.abs() < spring.threshold * 10.0;
                    if settled {
                        self.progress = 1.0;
                        self.velocity = 0.0;
                        if self.complete_iteration() {
                            return TickResult {
                                changed: true,
                                finished: true,
                            };
                        }
                    }
                }
                self.current = self.from.wings_lerp(&self.to, self.progress);
                TickResult {
                    changed: true,
                    finished: false,
                }
            }
        }
    }

    /// Handles the end of an iteration. Returns `true` when the whole animation finished.
    fn complete_iteration(&mut self) -> bool {
        self.iteration += 1;
        if self.repeat.should_continue(self.iteration) {
            if self.repeat.mode == RepeatMode::Reverse {
                core::mem::swap(&mut self.from, &mut self.to);
            }
            self.current = self.from.clone();
            self.progress = 0.0;
            self.velocity = 0.0;
            false
        } else {
            self.current = self.to.clone();
            self.running = false;
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::easing::WingsEasing;
    use crate::spec::{RepeatCount, SpringSpec};

    fn linear(d: f32) -> AnimationSpec {
        AnimationSpec::tween(d).with_easing(WingsEasing::Linear)
    }

    /// A new value is idle and reports no change on tick.
    #[test]
    fn idle_value_does_nothing() {
        let mut v = WingsAnimatedValue::new(3.0f32);
        assert_eq!(v.tick(1.0), TickResult::default());
        assert_eq!(*v.value(), 3.0);
    }

    /// The tween honours its start delay.
    #[test]
    fn tween_respects_delay() {
        let mut v = WingsAnimatedValue::new(0.0f32);
        v.animate_to(1.0, linear(1.0).with_delay(0.5));
        v.tick(0.25);
        assert_eq!(*v.value(), 0.0);
        v.tick(0.75);
        assert!((v.value() - 0.5).abs() < 1e-4);
    }

    /// Snap jumps immediately without running.
    #[test]
    fn snap_is_instant() {
        let mut v = WingsAnimatedValue::new(0.0f32);
        v.animate_to(9.0, AnimationSpec::Snap);
        assert_eq!(*v.value(), 9.0);
        assert!(!v.is_running());
    }

    /// Retargeting a tween starts from the current (interrupted) value.
    #[test]
    fn retarget_starts_from_current() {
        let mut v = WingsAnimatedValue::new(0.0f32);
        v.animate_to(10.0, linear(1.0));
        v.tick(0.5);
        v.animate_to(0.0, linear(1.0));
        v.tick(0.5);
        assert!((v.value() - 2.5).abs() < 1e-4);
    }

    /// A critically damped spring converges to the target and stops.
    #[test]
    fn spring_converges() {
        let mut v = WingsAnimatedValue::new(0.0f32);
        v.animate_to(100.0, AnimationSpec::spring());
        let mut finished = false;
        for _ in 0..600 {
            if v.tick(1.0 / 60.0).finished {
                finished = true;
                break;
            }
        }
        assert!(finished);
        assert_eq!(*v.value(), 100.0);
    }

    /// An under damped spring overshoots its target.
    #[test]
    fn bouncy_spring_overshoots() {
        let mut v = WingsAnimatedValue::new(0.0f32);
        v.animate_to(1.0, AnimationSpec::spring_with(SpringSpec::BOUNCY));
        let mut max = 0.0f32;
        for _ in 0..240 {
            v.tick(1.0 / 60.0);
            max = max.max(*v.value());
        }
        assert!(max > 1.0);
    }

    /// Retargeting a running spring keeps its momentum.
    #[test]
    fn spring_keeps_velocity_on_retarget() {
        let mut v = WingsAnimatedValue::new(0.0f32);
        v.animate_to(1.0, AnimationSpec::spring());
        v.tick(0.05);
        assert!(v.velocity() > 0.0);
        v.animate_to(2.0, AnimationSpec::spring());
        assert!(v.velocity() > 0.0);
    }

    /// Ping pong repetition swaps direction each iteration.
    #[test]
    fn ping_pong_reverses() {
        let mut v = WingsAnimatedValue::new(0.0f32);
        v.animate_to_with_repeat(1.0, linear(1.0), Repeat::times(2, RepeatMode::Reverse));
        v.tick(1.25);
        assert!((v.value() - 0.75).abs() < 1e-4);
        let r = v.tick(1.0);
        assert!(r.finished);
        assert_eq!(*v.value(), 0.0);
    }

    /// Infinite repetition never finishes.
    #[test]
    fn infinite_never_finishes() {
        let mut v = WingsAnimatedValue::new(0.0f32);
        v.animate_to_with_repeat(
            1.0,
            linear(0.1),
            Repeat {
                count: RepeatCount::Infinite,
                mode: RepeatMode::Restart,
            },
        );
        for _ in 0..100 {
            assert!(!v.tick(0.07).finished);
        }
        assert!(v.is_running());
        assert!(v.iteration() > 10);
    }

    /// `animate_from_to` forces the start value.
    #[test]
    fn from_to_forces_start() {
        let mut v = WingsAnimatedValue::new(5.0f32);
        v.animate_from_to(0.0, 1.0, linear(1.0), Repeat::ONCE);
        v.tick(0.5);
        assert!((v.value() - 0.5).abs() < 1e-4);
    }
}
