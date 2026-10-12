//! Interpolation of animatable values.

use bevy::color::{Color, Mix, Oklaba};
use bevy::math::{Rot2, Vec2, Vec3, Vec4};
use bevy::ui::Val;

/// A value that can be interpolated by the Wings animation system.
pub trait WingsLerp: Clone + Send + Sync + 'static {
    /// Interpolates between `self` (at `t = 0`) and `other` (at `t = 1`). `t` may leave the
    /// unit range for overshooting curves and springs.
    fn wings_lerp(&self, other: &Self, t: f32) -> Self;

    /// A non negative distance between two values, used to rescale spring velocity when an
    /// animation is retargeted. Return `1.0` when no meaningful metric exists.
    fn wings_distance(&self, other: &Self) -> f32;
}

impl WingsLerp for f32 {
    fn wings_lerp(&self, other: &Self, t: f32) -> Self {
        self + (other - self) * t
    }
    fn wings_distance(&self, other: &Self) -> f32 {
        (other - self).abs()
    }
}

impl WingsLerp for Vec2 {
    fn wings_lerp(&self, other: &Self, t: f32) -> Self {
        *self + (*other - *self) * t
    }
    fn wings_distance(&self, other: &Self) -> f32 {
        self.distance(*other)
    }
}

impl WingsLerp for Vec3 {
    fn wings_lerp(&self, other: &Self, t: f32) -> Self {
        *self + (*other - *self) * t
    }
    fn wings_distance(&self, other: &Self) -> f32 {
        self.distance(*other)
    }
}

impl WingsLerp for Vec4 {
    fn wings_lerp(&self, other: &Self, t: f32) -> Self {
        *self + (*other - *self) * t
    }
    fn wings_distance(&self, other: &Self) -> f32 {
        self.distance(*other)
    }
}

/// Colors interpolate in the perceptual Oklab space, which avoids muddy midpoints.
impl WingsLerp for Color {
    fn wings_lerp(&self, other: &Self, t: f32) -> Self {
        let a = Oklaba::from(*self);
        let b = Oklaba::from(*other);
        Color::Oklaba(a.mix(&b, t))
    }
    fn wings_distance(&self, other: &Self) -> f32 {
        let a = Oklaba::from(*self);
        let b = Oklaba::from(*other);
        Vec4::new(
            a.lightness - b.lightness,
            a.a - b.a,
            a.b - b.b,
            a.alpha - b.alpha,
        )
        .length()
    }
}

/// Rotations interpolate along the shortest arc.
impl WingsLerp for Rot2 {
    fn wings_lerp(&self, other: &Self, t: f32) -> Self {
        let delta = self.angle_to(*other);
        Rot2::radians(self.as_radians() + delta * t)
    }
    fn wings_distance(&self, other: &Self) -> f32 {
        self.angle_to(*other).abs()
    }
}

/// Values of the same unit interpolate numerically. Mixed units snap at the midpoint because
/// they cannot be compared without layout information.
impl WingsLerp for Val {
    fn wings_lerp(&self, other: &Self, t: f32) -> Self {
        use Val::*;
        match (*self, *other) {
            (Px(a), Px(b)) => Px(a.wings_lerp(&b, t)),
            (Percent(a), Percent(b)) => Percent(a.wings_lerp(&b, t)),
            (Vw(a), Vw(b)) => Vw(a.wings_lerp(&b, t)),
            (Vh(a), Vh(b)) => Vh(a.wings_lerp(&b, t)),
            (VMin(a), VMin(b)) => VMin(a.wings_lerp(&b, t)),
            (VMax(a), VMax(b)) => VMax(a.wings_lerp(&b, t)),
            (Em(a), Em(b)) => Em(a.wings_lerp(&b, t)),
            (Rem(a), Rem(b)) => Rem(a.wings_lerp(&b, t)),
            (a, b) => {
                if t < 0.5 {
                    a
                } else {
                    b
                }
            }
        }
    }
    fn wings_distance(&self, other: &Self) -> f32 {
        use Val::*;
        match (*self, *other) {
            (Px(a), Px(b))
            | (Percent(a), Percent(b))
            | (Vw(a), Vw(b))
            | (Vh(a), Vh(b))
            | (VMin(a), VMin(b))
            | (VMax(a), VMax(b))
            | (Em(a), Em(b))
            | (Rem(a), Rem(b)) => (a - b).abs(),
            _ => 1.0,
        }
    }
}

impl<A: WingsLerp, B: WingsLerp> WingsLerp for (A, B) {
    fn wings_lerp(&self, other: &Self, t: f32) -> Self {
        (
            self.0.wings_lerp(&other.0, t),
            self.1.wings_lerp(&other.1, t),
        )
    }
    fn wings_distance(&self, other: &Self) -> f32 {
        self.0.wings_distance(&other.0) + self.1.wings_distance(&other.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Scalars interpolate linearly and extrapolate for overshoot.
    #[test]
    fn f32_lerp_and_extrapolate() {
        assert_eq!(0.0f32.wings_lerp(&10.0, 0.5), 5.0);
        assert_eq!(0.0f32.wings_lerp(&10.0, 1.1), 11.0);
        assert_eq!(2.0f32.wings_distance(&-1.0), 3.0);
    }

    /// Colors hit both endpoints exactly.
    #[test]
    fn color_endpoints() {
        let a = Color::srgb(1.0, 0.0, 0.0);
        let b = Color::srgb(0.0, 0.0, 1.0);
        let start = a.wings_lerp(&b, 0.0).to_srgba();
        let end = a.wings_lerp(&b, 1.0).to_srgba();
        assert!((start.red - 1.0).abs() < 1e-3);
        assert!((end.blue - 1.0).abs() < 1e-3);
        assert!(a.wings_distance(&b) > 0.0);
        assert!(a.wings_distance(&a) < 1e-6);
    }

    /// Rotations take the shortest path across the +/- PI boundary.
    #[test]
    fn rotation_shortest_arc() {
        let a = Rot2::degrees(170.0);
        let b = Rot2::degrees(-170.0);
        let mid = a.wings_lerp(&b, 0.5);
        assert!((mid.as_degrees().abs() - 180.0).abs() < 1e-3);
    }

    /// `Val` interpolates within a unit and snaps across units.
    #[test]
    fn val_units() {
        assert_eq!(
            Val::Px(0.0).wings_lerp(&Val::Px(100.0), 0.25),
            Val::Px(25.0)
        );
        assert_eq!(
            Val::Px(0.0).wings_lerp(&Val::Percent(50.0), 0.4),
            Val::Px(0.0)
        );
        assert_eq!(
            Val::Px(0.0).wings_lerp(&Val::Percent(50.0), 0.6),
            Val::Percent(50.0)
        );
    }

    /// Tuples interpolate component wise.
    #[test]
    fn tuple_lerp() {
        let a = (0.0f32, Vec2::ZERO);
        let b = (2.0f32, Vec2::splat(4.0));
        assert_eq!(a.wings_lerp(&b, 0.5), (1.0, Vec2::splat(2.0)));
    }
}
