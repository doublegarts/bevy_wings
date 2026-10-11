//! Keyframe animation.
//!
//! [`WingsKeyframes`] is a list of `(time, value, easing)` stops sampled over a duration.
//! [`WingsKeyframeAnimator<P>`] plays keyframes on a [`WingsProperty`], with optional
//! repetition: perfect for attention grabbing effects such as a "shake" or a "heartbeat".

use core::marker::PhantomData;

use bevy::prelude::*;

use crate::easing::WingsEasing;
use crate::lerp::WingsLerp;
use crate::property::{WingsAnimationFinished, WingsProperty};
use crate::spec::{Repeat, RepeatMode};

/// One keyframe stop.
#[derive(Debug, Clone, PartialEq)]
pub struct Keyframe<T> {
    /// Normalized time of the stop in `0.0..=1.0`.
    pub at: f32,
    /// Value at that time.
    pub value: T,
    /// Easing used for the segment that *ends* at this stop.
    pub easing: WingsEasing,
}

/// A keyframe track.
#[derive(Debug, Clone, PartialEq)]
pub struct WingsKeyframes<T> {
    frames: Vec<Keyframe<T>>,
    /// Total duration of one iteration in seconds.
    pub duration: f32,
}

impl<T: WingsLerp> WingsKeyframes<T> {
    /// Creates an empty track with the given duration in seconds.
    pub fn new(duration: f32) -> Self {
        Self {
            frames: Vec::new(),
            duration,
        }
    }

    /// Adds a stop with linear easing. Stops are kept sorted by time.
    pub fn at(self, at: f32, value: T) -> Self {
        self.at_with(at, value, WingsEasing::Linear)
    }

    /// Adds a stop with a custom easing for the segment ending at it.
    pub fn at_with(mut self, at: f32, value: T, easing: WingsEasing) -> Self {
        let at = at.clamp(0.0, 1.0);
        let index = self.frames.partition_point(|f| f.at <= at);
        self.frames.insert(index, Keyframe { at, value, easing });
        self
    }

    /// Number of stops.
    pub fn len(&self) -> usize {
        self.frames.len()
    }

    /// True when there are no stops.
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// Samples the track at normalized time `t`. Returns `None` for an empty track.
    pub fn sample(&self, t: f32) -> Option<T> {
        let first = self.frames.first()?;
        let t = t.clamp(0.0, 1.0);
        if t <= first.at {
            return Some(first.value.clone());
        }
        for pair in self.frames.windows(2) {
            let (a, b) = (&pair[0], &pair[1]);
            if t <= b.at {
                let span = (b.at - a.at).max(1e-6);
                let local = b.easing.ease((t - a.at) / span);
                return Some(a.value.wings_lerp(&b.value, local));
            }
        }
        self.frames.last().map(|f| f.value.clone())
    }
}

/// Plays a [`WingsKeyframes`] track on property `P`.
#[derive(Component)]
pub struct WingsKeyframeAnimator<P: WingsProperty> {
    /// The track.
    pub keyframes: WingsKeyframes<P::Value>,
    /// Repetition.
    pub repeat: Repeat,
    elapsed: f32,
    iteration: u32,
    finished: bool,
    _marker: PhantomData<P>,
}

impl<P: WingsProperty> WingsKeyframeAnimator<P> {
    /// Creates a keyframe animator that starts on the next frame.
    pub fn new(keyframes: WingsKeyframes<P::Value>, repeat: Repeat) -> Self {
        Self {
            keyframes,
            repeat,
            elapsed: 0.0,
            iteration: 0,
            finished: false,
            _marker: PhantomData,
        }
    }

    /// True once all iterations played.
    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// Restarts from the beginning.
    pub fn restart(&mut self) {
        self.elapsed = 0.0;
        self.iteration = 0;
        self.finished = false;
    }

    /// Advances by `dt` and returns the sampled value and whether the track just finished.
    pub fn advance(&mut self, dt: f32) -> (Option<P::Value>, bool) {
        if self.finished {
            return (None, false);
        }
        let duration = self.keyframes.duration.max(1e-6);
        self.elapsed += dt.max(0.0);
        while self.elapsed >= duration {
            self.elapsed -= duration;
            self.iteration += 1;
            if !self.repeat.should_continue(self.iteration) {
                self.finished = true;
                let end = if self.repeat.mode == RepeatMode::Reverse
                    && self.iteration.is_multiple_of(2)
                {
                    0.0
                } else {
                    1.0
                };
                return (self.keyframes.sample(end), true);
            }
        }
        let mut t = self.elapsed / duration;
        if self.repeat.mode == RepeatMode::Reverse && self.iteration % 2 == 1 {
            t = 1.0 - t;
        }
        (self.keyframes.sample(t), false)
    }
}

/// Drives keyframe animators of property `P`.
pub fn drive_keyframes<P: WingsProperty>(
    time: Res<Time>,
    mut query: Query<(Entity, &mut WingsKeyframeAnimator<P>, &mut P::Target)>,
    mut commands: Commands,
) {
    let dt = time.delta_secs();
    for (entity, mut animator, mut target) in &mut query {
        if animator.finished {
            continue;
        }
        let (value, finished) = animator.advance(dt);
        if let Some(v) = value
            && P::get(&target) != v
        {
            P::set(&mut target, v);
        }
        if finished {
            commands.trigger(WingsAnimationFinished {
                entity,
                property: core::any::type_name::<P>(),
            });
        }
    }
}

/// Ready made keyframe tracks.
pub mod presets {
    use super::WingsKeyframes;
    use bevy::math::Vec2;

    /// Horizontal shake (use with [`crate::property::TranslationProperty`]), for invalid input.
    pub fn shake(amplitude: f32) -> WingsKeyframes<Vec2> {
        WingsKeyframes::new(0.4)
            .at(0.0, Vec2::ZERO)
            .at(0.15, Vec2::new(-amplitude, 0.0))
            .at(0.35, Vec2::new(amplitude, 0.0))
            .at(0.55, Vec2::new(-amplitude * 0.6, 0.0))
            .at(0.75, Vec2::new(amplitude * 0.3, 0.0))
            .at(1.0, Vec2::ZERO)
    }

    /// Heartbeat scale pulse (use with [`crate::property::ScaleProperty`]).
    pub fn heartbeat(peak: f32) -> WingsKeyframes<Vec2> {
        WingsKeyframes::new(0.8)
            .at(0.0, Vec2::ONE)
            .at(0.15, Vec2::splat(peak))
            .at(0.3, Vec2::ONE)
            .at(0.45, Vec2::splat(1.0 + (peak - 1.0) * 0.6))
            .at(1.0, Vec2::ONE)
    }

    /// Opacity blink (use with [`crate::property::OpacityProperty`]).
    pub fn blink() -> WingsKeyframes<f32> {
        WingsKeyframes::new(1.0)
            .at(0.0, 1.0)
            .at(0.5, 0.2)
            .at(1.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::property::OpacityProperty;

    /// Sampling interpolates between neighbouring stops.
    #[test]
    fn sample_between_stops() {
        let k = WingsKeyframes::new(1.0)
            .at(0.0, 0.0f32)
            .at(0.5, 10.0)
            .at(1.0, 0.0);
        assert_eq!(k.sample(0.25), Some(5.0));
        assert_eq!(k.sample(0.5), Some(10.0));
        assert_eq!(k.sample(0.75), Some(5.0));
    }

    /// Stops are kept sorted even when added out of order.
    #[test]
    fn stops_are_sorted() {
        let k = WingsKeyframes::new(1.0).at(1.0, 2.0f32).at(0.0, 0.0);
        assert_eq!(k.sample(0.5), Some(1.0));
        assert_eq!(k.len(), 2);
    }

    /// An empty track samples to `None`.
    #[test]
    fn empty_track() {
        let k: WingsKeyframes<f32> = WingsKeyframes::new(1.0);
        assert!(k.is_empty());
        assert_eq!(k.sample(0.5), None);
    }

    /// The animator finishes after its iterations and reports the final value.
    #[test]
    fn animator_finishes() {
        let mut a = WingsKeyframeAnimator::<OpacityProperty>::new(
            WingsKeyframes::new(1.0).at(0.0, 0.0).at(1.0, 1.0),
            Repeat::ONCE,
        );
        let (v, done) = a.advance(0.5);
        assert_eq!(v, Some(0.5));
        assert!(!done);
        let (v, done) = a.advance(0.6);
        assert_eq!(v, Some(1.0));
        assert!(done);
        assert!(a.is_finished());
    }

    /// The shake preset starts and ends at rest.
    #[test]
    fn shake_rests_at_ends() {
        let k = presets::shake(8.0);
        assert_eq!(k.sample(0.0), Some(Vec2::ZERO));
        assert_eq!(k.sample(1.0), Some(Vec2::ZERO));
        assert!(k.sample(0.35).unwrap().x > 7.0);
    }
}
