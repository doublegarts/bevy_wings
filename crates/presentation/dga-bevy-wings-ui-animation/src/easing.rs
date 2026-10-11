//! Easing curves.
//!
//! [`WingsEasing`] maps a linear progress `t` in `0.0..=1.0` to an eased progress. It includes
//! the classic polynomial curves, the motion tokens used by the Wings design system
//! (standard, emphasized, decelerate and accelerate cubic Bezier curves), physical feeling curves
//! (back, elastic, bounce), stepped easing and a bridge to every Bevy
//! [`EaseFunction`].

use bevy::curve::Curve;
use bevy::curve::easing::EaseFunction;

/// An easing curve.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum WingsEasing {
    /// No easing.
    Linear,
    /// Quadratic ease in.
    EaseIn,
    /// Quadratic ease out.
    EaseOut,
    /// Quadratic ease in and out.
    EaseInOut,
    /// Wings motion token: general purpose movement, `cubic-bezier(0.2, 0, 0, 1)`.
    #[default]
    Standard,
    /// Wings motion token: elements entering the screen, `cubic-bezier(0, 0, 0, 1)`.
    StandardDecelerate,
    /// Wings motion token: elements leaving the screen, `cubic-bezier(0.3, 0, 1, 1)`.
    StandardAccelerate,
    /// Wings motion token: expressive movement, `cubic-bezier(0.2, 0, 0, 1)` with a longer tail.
    Emphasized,
    /// Wings motion token: expressive enter, `cubic-bezier(0.05, 0.7, 0.1, 1)`.
    EmphasizedDecelerate,
    /// Wings motion token: expressive exit, `cubic-bezier(0.3, 0, 0.8, 0.15)`.
    EmphasizedAccelerate,
    /// A CSS style cubic Bezier curve through `(0,0)`, `(x1,y1)`, `(x2,y2)` and `(1,1)`.
    CubicBezier(f32, f32, f32, f32),
    /// Overshoots slightly before settling (back out).
    BackOut,
    /// Spring like oscillation at the end (elastic out).
    ElasticOut,
    /// Bounces at the end like a dropped ball.
    BounceOut,
    /// Jumps in `n` discrete steps.
    Steps(u32),
    /// Any easing function shipped with Bevy.
    Bevy(EaseFunction),
}

impl WingsEasing {
    /// Evaluates the curve. `t` is clamped to `0.0..=1.0`. The output is `0.0` at `t = 0` and
    /// `1.0` at `t = 1` but may overshoot in between for back and elastic curves.
    pub fn ease(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            WingsEasing::Linear => t,
            WingsEasing::EaseIn => t * t,
            WingsEasing::EaseOut => 1.0 - (1.0 - t) * (1.0 - t),
            WingsEasing::EaseInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
                }
            }
            WingsEasing::Standard => cubic_bezier(0.2, 0.0, 0.0, 1.0, t),
            WingsEasing::StandardDecelerate => cubic_bezier(0.0, 0.0, 0.0, 1.0, t),
            WingsEasing::StandardAccelerate => cubic_bezier(0.3, 0.0, 1.0, 1.0, t),
            WingsEasing::Emphasized => cubic_bezier(0.2, 0.0, 0.0, 1.0, t).powf(0.9),
            WingsEasing::EmphasizedDecelerate => cubic_bezier(0.05, 0.7, 0.1, 1.0, t),
            WingsEasing::EmphasizedAccelerate => cubic_bezier(0.3, 0.0, 0.8, 0.15, t),
            WingsEasing::CubicBezier(x1, y1, x2, y2) => cubic_bezier(x1, y1, x2, y2, t),
            WingsEasing::BackOut => {
                const C1: f32 = 1.70158;
                const C3: f32 = C1 + 1.0;
                1.0 + C3 * (t - 1.0).powi(3) + C1 * (t - 1.0).powi(2)
            }
            WingsEasing::ElasticOut => {
                if t == 0.0 || t == 1.0 {
                    t
                } else {
                    let c4 = core::f32::consts::TAU / 3.0;
                    2f32.powf(-10.0 * t) * ((t * 10.0 - 0.75) * c4).sin() + 1.0
                }
            }
            WingsEasing::BounceOut => bounce_out(t),
            WingsEasing::Steps(n) => {
                let n = n.max(1) as f32;
                (t * n).floor().min(n) / n
            }
            WingsEasing::Bevy(f) => f.sample_clamped(t),
        }
    }
}

fn bounce_out(t: f32) -> f32 {
    const N1: f32 = 7.5625;
    const D1: f32 = 2.75;
    if t < 1.0 / D1 {
        N1 * t * t
    } else if t < 2.0 / D1 {
        let t = t - 1.5 / D1;
        N1 * t * t + 0.75
    } else if t < 2.5 / D1 {
        let t = t - 2.25 / D1;
        N1 * t * t + 0.9375
    } else {
        let t = t - 2.625 / D1;
        N1 * t * t + 0.984375
    }
}

/// Evaluates a CSS style cubic Bezier easing curve at `x`.
///
/// Solves `bezier_x(s) = x` with Newton iterations, falling back to bisection, then returns
/// `bezier_y(s)`. Accurate to about `1e-5`.
pub fn cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32, x: f32) -> f32 {
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }
    let bez = |a: f32, b: f32, s: f32| {
        let inv = 1.0 - s;
        3.0 * inv * inv * s * a + 3.0 * inv * s * s * b + s * s * s
    };
    let deriv = |a: f32, b: f32, s: f32| {
        let inv = 1.0 - s;
        3.0 * inv * inv * a + 6.0 * inv * s * (b - a) + 3.0 * s * s * (1.0 - b)
    };
    let mut s = x;
    for _ in 0..8 {
        let err = bez(x1, x2, s) - x;
        if err.abs() < 1e-6 {
            return bez(y1, y2, s);
        }
        let d = deriv(x1, x2, s);
        if d.abs() < 1e-6 {
            break;
        }
        s -= err / d;
    }
    let (mut lo, mut hi) = (0.0f32, 1.0f32);
    s = x;
    for _ in 0..40 {
        let v = bez(x1, x2, s);
        if (v - x).abs() < 1e-6 {
            break;
        }
        if v < x {
            lo = s;
        } else {
            hi = s;
        }
        s = (lo + hi) * 0.5;
    }
    bez(y1, y2, s)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [WingsEasing; 15] = [
        WingsEasing::Linear,
        WingsEasing::EaseIn,
        WingsEasing::EaseOut,
        WingsEasing::EaseInOut,
        WingsEasing::Standard,
        WingsEasing::StandardDecelerate,
        WingsEasing::StandardAccelerate,
        WingsEasing::Emphasized,
        WingsEasing::EmphasizedDecelerate,
        WingsEasing::EmphasizedAccelerate,
        WingsEasing::CubicBezier(0.4, 0.0, 0.2, 1.0),
        WingsEasing::BackOut,
        WingsEasing::ElasticOut,
        WingsEasing::BounceOut,
        WingsEasing::Bevy(EaseFunction::CubicInOut),
    ];

    /// Every curve starts at 0 and ends at 1.
    #[test]
    fn all_curves_hit_endpoints() {
        for e in ALL {
            assert!(e.ease(0.0).abs() < 1e-4, "{e:?} at 0");
            assert!((e.ease(1.0) - 1.0).abs() < 1e-4, "{e:?} at 1");
        }
    }

    /// Input outside of the unit range is clamped.
    #[test]
    fn input_is_clamped() {
        assert_eq!(WingsEasing::Linear.ease(-3.0), 0.0);
        assert_eq!(WingsEasing::Linear.ease(3.0), 1.0);
    }

    /// The cubic Bezier `(0.25, 0.25, 0.75, 0.75)` is a straight line.
    #[test]
    fn linear_bezier_is_identity() {
        for i in 0..=10 {
            let x = i as f32 / 10.0;
            assert!((cubic_bezier(0.25, 0.25, 0.75, 0.75, x) - x).abs() < 1e-4);
        }
    }

    /// Ease out curves are ahead of linear progress at the midpoint, ease in curves behind.
    #[test]
    fn ease_in_and_out_shape() {
        assert!(WingsEasing::EaseOut.ease(0.5) > 0.5);
        assert!(WingsEasing::EaseIn.ease(0.5) < 0.5);
        assert!(WingsEasing::StandardDecelerate.ease(0.5) > 0.5);
        assert!(WingsEasing::StandardAccelerate.ease(0.5) < 0.5);
    }

    /// Back out overshoots above 1.0 before settling.
    #[test]
    fn back_out_overshoots() {
        let max = (0..100)
            .map(|i| WingsEasing::BackOut.ease(i as f32 / 100.0))
            .fold(0.0f32, f32::max);
        assert!(max > 1.0);
    }

    /// Steps produce discrete plateaus.
    #[test]
    fn steps_are_discrete() {
        let e = WingsEasing::Steps(4);
        assert_eq!(e.ease(0.1), 0.0);
        assert_eq!(e.ease(0.3), 0.25);
        assert_eq!(e.ease(0.99), 0.75);
        assert_eq!(e.ease(1.0), 1.0);
    }

    /// Monotonic curves never go backwards.
    #[test]
    fn standard_curves_are_monotonic() {
        for e in [
            WingsEasing::Standard,
            WingsEasing::EaseInOut,
            WingsEasing::EmphasizedDecelerate,
        ] {
            let mut last = 0.0;
            for i in 0..=200 {
                let v = e.ease(i as f32 / 200.0);
                assert!(v + 1e-5 >= last, "{e:?} decreased");
                last = v;
            }
        }
    }
}
