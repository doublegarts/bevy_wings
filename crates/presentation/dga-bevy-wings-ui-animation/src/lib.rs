//! # dga-bevy-wings-ui-animation
//!
//! A small, ECS first animation toolkit for Bevy UI.
//!
//! * [`WingsEasing`](easing::WingsEasing): polynomial, cubic Bezier, motion token, back,
//!   elastic, bounce and stepped curves, plus a bridge to every Bevy ease function.
//! * [`AnimationSpec`](spec::AnimationSpec): tween, spring or snap, with delays and
//!   staggering; [`Repeat`](spec::Repeat) for finite, infinite and ping pong loops.
//! * [`WingsAnimatedValue`](value::WingsAnimatedValue): the state machine, usable anywhere.
//! * [`WingsAnimator<P>`](property::WingsAnimator): a component that animates one property of
//!   one component. Built in properties cover colors, transform, size and subtree opacity.
//! * [`WingsKeyframeAnimator<P>`](keyframes::WingsKeyframeAnimator): keyframe tracks.
//! * [`WingsAnimatedVisibility`](visibility::WingsAnimatedVisibility): enter and exit
//!   transitions for whole subtrees.
//!
//! ```ignore
//! use dga_bevy_wings_ui_animation::prelude::*;
//!
//! fn grow_on_hover(mut commands: Commands, hovered: Query<(Entity, &Hovered), Changed<Hovered>>) {
//!     for (e, h) in &hovered {
//!         let target = if h.0 { Vec2::splat(1.05) } else { Vec2::ONE };
//!         commands.entity(e).animate::<ScaleProperty>(target, AnimationSpec::spring());
//!     }
//! }
//! ```

pub mod easing;
pub mod keyframes;
pub mod lerp;
pub mod property;
pub mod spec;
pub mod value;
pub mod visibility;

use bevy::prelude::*;
use dga_bevy_wings_ui_core::{WingsUiSystems, ensure_core};

use keyframes::drive_keyframes;
use property::*;

/// Registers the animation systems for all built in properties and the visibility transitions.
#[derive(Default)]
pub struct WingsAnimationPlugin;

impl Plugin for WingsAnimationPlugin {
    fn build(&self, app: &mut App) {
        ensure_core(app);
        app.register_wings_property::<BackgroundColorProperty>()
            .register_wings_property::<TextColorProperty>()
            .register_wings_property::<BorderColorProperty>()
            .register_wings_property::<ScaleProperty>()
            .register_wings_property::<RotationProperty>()
            .register_wings_property::<TranslationProperty>()
            .register_wings_property::<WidthProperty>()
            .register_wings_property::<HeightProperty>()
            .register_wings_property::<OpacityProperty>();
        app.add_systems(
            Update,
            (
                drive_keyframes::<ScaleProperty>,
                drive_keyframes::<TranslationProperty>,
                drive_keyframes::<RotationProperty>,
                drive_keyframes::<OpacityProperty>,
                drive_keyframes::<BackgroundColorProperty>,
            )
                .in_set(WingsUiSystems::Animate),
        );
        app.add_systems(
            Update,
            (
                visibility::start_visibility_transitions.before(WingsUiSystems::Animate),
                visibility::finish_visibility_transitions.after(WingsUiSystems::Animate),
            ),
        );
    }
}

/// Commonly used items.
pub mod prelude {
    pub use crate::WingsAnimationPlugin;
    pub use crate::easing::WingsEasing;
    pub use crate::keyframes::{
        WingsKeyframeAnimator, WingsKeyframes, presets as keyframe_presets,
    };
    pub use crate::lerp::WingsLerp;
    pub use crate::property::{
        BackgroundColorProperty, BorderColorProperty, HeightProperty, OpacityProperty,
        RotationProperty, ScaleProperty, TextColorProperty, TranslationProperty, WidthProperty,
        WingsAnimateExt, WingsAnimationAppExt, WingsAnimationFinished, WingsAnimator,
        WingsProperty,
    };
    pub use crate::spec::{AnimationSpec, Repeat, RepeatCount, RepeatMode, SpringSpec};
    pub use crate::value::{TickResult, WingsAnimatedValue};
    pub use crate::visibility::{WingsAnimatedVisibility, WingsTransition};
}

#[cfg(test)]
mod tests {
    use super::prelude::*;
    use bevy::prelude::*;
    use bevy::time::TimeUpdateStrategy;
    use core::time::Duration;
    use dga_bevy_wings_ui_core::WingsOpacity;

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, WingsAnimationPlugin));
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            50,
        )));
        // The first update initializes time with a zero delta.
        app.update();
        app
    }

    #[derive(Resource, Default)]
    struct Finished(u32);

    /// The animate command drives the target component to its final value and fires the
    /// finished event exactly once.
    #[test]
    fn command_animates_component_and_fires_event() {
        let mut app = app();
        app.init_resource::<Finished>();
        app.add_observer(|_: On<WingsAnimationFinished>, mut f: ResMut<Finished>| f.0 += 1);
        let e = app.world_mut().spawn(UiTransform::default()).id();
        app.world_mut()
            .commands()
            .entity(e)
            .animate::<ScaleProperty>(Vec2::splat(2.0), AnimationSpec::millis(200));
        for _ in 0..10 {
            app.update();
        }
        let scale = app.world().get::<UiTransform>(e).unwrap().scale;
        assert_eq!(scale, Vec2::splat(2.0));
        assert_eq!(app.world().resource::<Finished>().0, 1);
    }

    /// Opacity can be tweened like any other property.
    #[test]
    fn opacity_tween_reaches_zero() {
        let mut app = app();
        let e = app
            .world_mut()
            .spawn((
                WingsOpacity(1.0),
                WingsAnimator::<OpacityProperty>::to(0.0, AnimationSpec::millis(100)),
            ))
            .id();
        for _ in 0..5 {
            app.update();
        }
        assert_eq!(app.world().get::<WingsOpacity>(e).unwrap().0, 0.0);
    }

    /// Hiding with an exit transition collapses the node once the fade completes, and
    /// showing restores the original display mode.
    #[test]
    fn animated_visibility_collapses_and_restores() {
        let mut app = app();
        let e = app
            .world_mut()
            .spawn((
                Node {
                    display: Display::Grid,
                    ..default()
                },
                WingsAnimatedVisibility::new(true),
            ))
            .id();
        app.update();
        app.world_mut()
            .get_mut::<WingsAnimatedVisibility>(e)
            .unwrap()
            .visible = false;
        app.update();
        // Still displayed while fading out.
        assert_eq!(app.world().get::<Node>(e).unwrap().display, Display::Grid);
        for _ in 0..10 {
            app.update();
        }
        assert_eq!(app.world().get::<Node>(e).unwrap().display, Display::None);
        assert_eq!(app.world().get::<WingsOpacity>(e).unwrap().0, 0.0);

        app.world_mut()
            .get_mut::<WingsAnimatedVisibility>(e)
            .unwrap()
            .visible = true;
        app.update();
        assert_eq!(app.world().get::<Node>(e).unwrap().display, Display::Grid);
        for _ in 0..10 {
            app.update();
        }
        assert_eq!(app.world().get::<WingsOpacity>(e).unwrap().0, 1.0);
    }

    /// A node that starts hidden is collapsed immediately, without animation.
    #[test]
    fn initially_hidden_is_collapsed() {
        let mut app = app();
        let e = app
            .world_mut()
            .spawn(WingsAnimatedVisibility::new(false))
            .id();
        app.update();
        assert_eq!(app.world().get::<Node>(e).unwrap().display, Display::None);
    }

    /// Keyframe animators write sampled values into the component.
    #[test]
    fn keyframes_drive_component() {
        let mut app = app();
        let e = app
            .world_mut()
            .spawn((
                UiTransform::default(),
                WingsKeyframeAnimator::<ScaleProperty>::new(
                    keyframe_presets::heartbeat(1.3),
                    Repeat::ONCE,
                ),
            ))
            .id();
        let mut max = 0.0f32;
        for _ in 0..20 {
            app.update();
            max = max.max(app.world().get::<UiTransform>(e).unwrap().scale.x);
        }
        assert!(max > 1.1);
        assert_eq!(app.world().get::<UiTransform>(e).unwrap().scale, Vec2::ONE);
    }

    /// Registering the same property twice does not add duplicate systems.
    #[test]
    fn duplicate_registration_is_ignored() {
        let mut app = app();
        app.register_wings_property::<ScaleProperty>();
        let e = app.world_mut().spawn(UiTransform::default()).id();
        app.world_mut()
            .commands()
            .entity(e)
            .animate::<ScaleProperty>(Vec2::splat(3.0), AnimationSpec::Snap);
        app.update();
        assert_eq!(
            app.world().get::<UiTransform>(e).unwrap().scale,
            Vec2::splat(3.0)
        );
    }
}
