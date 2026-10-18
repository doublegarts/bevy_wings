//! Animation specifications.
//!
//! An [`AnimationSpec`] describes *how* a value moves from its current value to a target:
//! a timed tween with an easing curve, a physics based spring, or an instant snap.
//! [`Repeat`] describes how many times and in which direction an animation repeats.

use crate::easing::WingsEasing;

/// Spring parameters (unit mass).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpringSpec {
    /// Spring stiffness. Higher values reach the target faster.
    pub stiffness: f32,
    /// Damping ratio. `1.0` is critically damped (no bounce), values below `1.0` bounce.
    pub damping_ratio: f32,
    /// Distance (in progress units, `0.0..=1.0`) under which the spring is considered settled.
    pub threshold: f32,
}

impl SpringSpec {
    /// Wings motion token: a gentle, non bouncy spring for general UI movement.
    pub const DEFAULT: Self = Self {
        stiffness: 380.0,
        damping_ratio: 1.0,
        threshold: 0.001,
    };
    /// A soft spring with a little bounce.
    pub const BOUNCY: Self = Self {
        stiffness: 300.0,
        damping_ratio: 0.5,
        threshold: 0.001,
    };
    /// A stiff, quick spring for small elements like switches and ripples.
    pub const STIFF: Self = Self {
        stiffness: 1400.0,
        damping_ratio: 1.0,
        threshold: 0.001,
    };
    /// A slow, lazy spring for large surfaces.
    pub const LOW: Self = Self {
        stiffness: 200.0,
        damping_ratio: 1.0,
        threshold: 0.001,
    };

    /// A spring described like SwiftUI's `spring(response:dampingFraction:)`: `response` is
    /// the period of the undamped oscillation in seconds.
    pub fn from_response(response: f32, damping_ratio: f32) -> Self {
        let omega = core::f32::consts::TAU / response.max(1e-3);
        Self {
            stiffness: omega * omega,
            damping_ratio,
            threshold: 0.001,
        }
    }

    /// Damping coefficient derived from stiffness and damping ratio.
    pub fn damping(&self) -> f32 {
        2.0 * self.damping_ratio * self.stiffness.max(0.0).sqrt()
    }
}

impl Default for SpringSpec {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// How a value animates towards its target.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnimationSpec {
    /// A timed tween.
    Tween {
        /// Duration in seconds.
        duration: f32,
        /// Easing curve.
        easing: WingsEasing,
        /// Delay before the tween starts, in seconds.
        delay: f32,
    },
    /// A physics based spring. Springs keep their velocity when retargeted, which makes
    /// interrupted animations feel natural.
    Spring {
        /// Spring parameters.
        spring: SpringSpec,
        /// Delay before the spring starts, in seconds.
        delay: f32,
    },
    /// Jump to the target immediately.
    Snap,
}

impl Default for AnimationSpec {
    fn default() -> Self {
        Self::medium()
    }
}

impl AnimationSpec {
    /// A tween with the given duration in seconds and the default [`WingsEasing::Standard`] curve.
    pub fn tween(duration: f32) -> Self {
        Self::Tween {
            duration,
            easing: WingsEasing::Standard,
            delay: 0.0,
        }
    }

    /// A tween with the given duration in milliseconds.
    pub fn millis(ms: u32) -> Self {
        Self::tween(ms as f32 / 1000.0)
    }

    /// A spring with [`SpringSpec::DEFAULT`].
    pub fn spring() -> Self {
        Self::Spring {
            spring: SpringSpec::DEFAULT,
            delay: 0.0,
        }
    }

    /// A spring with custom parameters.
    pub fn spring_with(spring: SpringSpec) -> Self {
        Self::Spring { spring, delay: 0.0 }
    }

    /// Wings motion token: short (100 ms) for small state changes like hover.
    pub fn short() -> Self {
        Self::millis(100)
    }

    /// Wings motion token: medium (250 ms) for most transitions.
    pub fn medium() -> Self {
        Self::millis(250)
    }

    /// Wings motion token: long (450 ms) for large surfaces and screen transitions.
    pub fn long() -> Self {
        Self::Tween {
            duration: 0.45,
            easing: WingsEasing::Emphasized,
            delay: 0.0,
        }
    }

    /// Returns a copy using `easing` (tweens only, other specs are returned unchanged).
    pub fn with_easing(self, easing: WingsEasing) -> Self {
        match self {
            Self::Tween {
                duration, delay, ..
            } => Self::Tween {
                duration,
                easing,
                delay,
            },
            other => other,
        }
    }

    /// Returns a copy with a start delay in seconds.
    pub fn with_delay(self, delay: f32) -> Self {
        match self {
            Self::Tween {
                duration, easing, ..
            } => Self::Tween {
                duration,
                easing,
                delay,
            },
            Self::Spring { spring, .. } => Self::Spring { spring, delay },
            Self::Snap => Self::Snap,
        }
    }

    /// Start delay in seconds.
    pub fn delay(&self) -> f32 {
        match self {
            Self::Tween { delay, .. } | Self::Spring { delay, .. } => *delay,
            Self::Snap => 0.0,
        }
    }

    /// Staggers this spec for the `index`th item of a list, adding `step` seconds per item.
    pub fn stagger(self, index: usize, step: f32) -> Self {
        let d = self.delay() + index as f32 * step;
        self.with_delay(d)
    }
}

/// How many times an animation runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RepeatCount {
    /// Run once (the default).
    #[default]
    Once,
    /// Run the given number of iterations in total.
    Times(u32),
    /// Repeat forever (loading indicators, pulses, shimmer).
    Infinite,
}

/// What happens at the end of an iteration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RepeatMode {
    /// Jump back to the start value.
    #[default]
    Restart,
    /// Play backwards (ping pong).
    Reverse,
}

/// Repetition settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Repeat {
    /// Iteration count.
    pub count: RepeatCount,
    /// Behaviour between iterations.
    pub mode: RepeatMode,
}

impl Repeat {
    /// Play once.
    pub const ONCE: Self = Self {
        count: RepeatCount::Once,
        mode: RepeatMode::Restart,
    };

    /// Repeat forever, restarting each time.
    pub fn infinite() -> Self {
        Self {
            count: RepeatCount::Infinite,
            mode: RepeatMode::Restart,
        }
    }

    /// Repeat forever, alternating direction.
    pub fn ping_pong() -> Self {
        Self {
            count: RepeatCount::Infinite,
            mode: RepeatMode::Reverse,
        }
    }

    /// Repeat `n` iterations in total.
    pub fn times(n: u32, mode: RepeatMode) -> Self {
        Self {
            count: RepeatCount::Times(n),
            mode,
        }
    }

    /// True when another iteration should run after `completed` iterations finished.
    pub fn should_continue(&self, completed: u32) -> bool {
        match self.count {
            RepeatCount::Once => false,
            RepeatCount::Times(n) => completed < n,
            RepeatCount::Infinite => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `millis` converts to seconds.
    #[test]
    fn millis_converts_to_seconds() {
        match AnimationSpec::millis(300) {
            AnimationSpec::Tween { duration, .. } => assert!((duration - 0.3).abs() < 1e-6),
            _ => panic!("expected tween"),
        }
    }

    /// Stagger adds a per index delay on top of the existing one.
    #[test]
    fn stagger_adds_delay() {
        let s = AnimationSpec::short().with_delay(0.1).stagger(3, 0.05);
        assert!((s.delay() - 0.25).abs() < 1e-6);
    }

    /// Easing replacement leaves springs untouched.
    #[test]
    fn with_easing_only_affects_tweens() {
        let s = AnimationSpec::spring().with_easing(WingsEasing::Linear);
        assert_eq!(s, AnimationSpec::spring());
    }

    /// Critically damped spring coefficient is `2 * sqrt(k)`.
    #[test]
    fn damping_coefficient() {
        let s = SpringSpec {
            stiffness: 100.0,
            damping_ratio: 1.0,
            threshold: 0.01,
        };
        assert!((s.damping() - 20.0).abs() < 1e-5);
    }

    /// Repeat counting.
    #[test]
    fn repeat_counting() {
        assert!(!Repeat::ONCE.should_continue(1));
        let r = Repeat::times(3, RepeatMode::Restart);
        assert!(r.should_continue(1));
        assert!(r.should_continue(2));
        assert!(!r.should_continue(3));
        assert!(Repeat::infinite().should_continue(10_000));
    }
}
