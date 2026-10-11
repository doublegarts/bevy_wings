//! Component property animation.
//!
//! A [`WingsProperty`] describes one animatable field of one component (for example the scale
//! of a [`UiTransform`] or the color of a [`BackgroundColor`]). A [`WingsAnimator<P>`]
//! component drives that property on its entity. Animators are started with the
//! [`WingsAnimateExt`] command extension:
//!
//! ```ignore
//! commands.entity(card).animate::<ScaleProperty>(Vec2::splat(1.05), AnimationSpec::spring());
//! ```
//!
//! Every registered property gets its own tiny, cache friendly system that only touches
//! entities with an active animator, and only writes the target component when the value
//! actually changed. When an animation completes a [`WingsAnimationFinished`] entity event is
//! triggered on the entity.

use core::marker::PhantomData;

use bevy::color::Color;
use bevy::ecs::component::Mutable;
use bevy::ecs::system::EntityCommands;
use bevy::math::{Rot2, Vec2};
use bevy::prelude::*;
use bevy::text::TextColor;
use bevy::ui::{BackgroundColor, BorderColor, Node, UiTransform, Val, Val2};

use dga_bevy_wings_ui_core::WingsOpacity;

use crate::lerp::WingsLerp;
use crate::spec::{AnimationSpec, Repeat};
use crate::value::WingsAnimatedValue;

/// An animatable property: a value of type [`WingsProperty::Value`] stored inside the
/// component [`WingsProperty::Target`].
pub trait WingsProperty: Send + Sync + 'static {
    /// The component holding the property.
    type Target: Component<Mutability = Mutable>;
    /// The animated value type.
    type Value: WingsLerp + PartialEq;
    /// Reads the property.
    fn get(target: &Self::Target) -> Self::Value;
    /// Writes the property.
    fn set(target: &mut Self::Target, value: Self::Value);
}

/// Background color of a UI node.
pub struct BackgroundColorProperty;
impl WingsProperty for BackgroundColorProperty {
    type Target = BackgroundColor;
    type Value = Color;
    fn get(t: &BackgroundColor) -> Color {
        t.0
    }
    fn set(t: &mut BackgroundColor, v: Color) {
        t.0 = v;
    }
}

/// Text color.
pub struct TextColorProperty;
impl WingsProperty for TextColorProperty {
    type Target = TextColor;
    type Value = Color;
    fn get(t: &TextColor) -> Color {
        t.0
    }
    fn set(t: &mut TextColor, v: Color) {
        t.0 = v;
    }
}

/// Border color (all four sides; reads the top side).
pub struct BorderColorProperty;
impl WingsProperty for BorderColorProperty {
    type Target = BorderColor;
    type Value = Color;
    fn get(t: &BorderColor) -> Color {
        t.top
    }
    fn set(t: &mut BorderColor, v: Color) {
        *t = BorderColor::all(v);
    }
}

/// Scale of a [`UiTransform`].
pub struct ScaleProperty;
impl WingsProperty for ScaleProperty {
    type Target = UiTransform;
    type Value = Vec2;
    fn get(t: &UiTransform) -> Vec2 {
        t.scale
    }
    fn set(t: &mut UiTransform, v: Vec2) {
        t.scale = v;
    }
}

/// Rotation of a [`UiTransform`].
pub struct RotationProperty;
impl WingsProperty for RotationProperty {
    type Target = UiTransform;
    type Value = Rot2;
    fn get(t: &UiTransform) -> Rot2 {
        t.rotation
    }
    fn set(t: &mut UiTransform, v: Rot2) {
        t.rotation = v;
    }
}

/// Translation of a [`UiTransform`] in logical pixels. Non pixel translations read as zero.
pub struct TranslationProperty;
impl WingsProperty for TranslationProperty {
    type Target = UiTransform;
    type Value = Vec2;
    fn get(t: &UiTransform) -> Vec2 {
        let px = |v: Val| match v {
            Val::Px(p) => p,
            _ => 0.0,
        };
        Vec2::new(px(t.translation.x), px(t.translation.y))
    }
    fn set(t: &mut UiTransform, v: Vec2) {
        t.translation = Val2::px(v.x, v.y);
    }
}

/// Width of a [`Node`].
pub struct WidthProperty;
impl WingsProperty for WidthProperty {
    type Target = Node;
    type Value = Val;
    fn get(t: &Node) -> Val {
        t.width
    }
    fn set(t: &mut Node, v: Val) {
        t.width = v;
    }
}

/// Height of a [`Node`].
pub struct HeightProperty;
impl WingsProperty for HeightProperty {
    type Target = Node;
    type Value = Val;
    fn get(t: &Node) -> Val {
        t.height
    }
    fn set(t: &mut Node, v: Val) {
        t.height = v;
    }
}

/// Subtree opacity ([`WingsOpacity`]).
pub struct OpacityProperty;
impl WingsProperty for OpacityProperty {
    type Target = WingsOpacity;
    type Value = f32;
    fn get(t: &WingsOpacity) -> f32 {
        t.0
    }
    fn set(t: &mut WingsOpacity, v: f32) {
        t.0 = v;
    }
}

/// A queued animation request, applied by the property system on the next tick.
#[derive(Debug, Clone)]
struct Request<V> {
    from: Option<V>,
    to: V,
    spec: AnimationSpec,
    repeat: Repeat,
}

/// Drives one [`WingsProperty`] of its entity.
#[derive(Component)]
pub struct WingsAnimator<P: WingsProperty> {
    value: Option<WingsAnimatedValue<P::Value>>,
    request: Option<Request<P::Value>>,
    _marker: PhantomData<P>,
}

impl<P: WingsProperty> Default for WingsAnimator<P> {
    fn default() -> Self {
        Self {
            value: None,
            request: None,
            _marker: PhantomData,
        }
    }
}

impl<P: WingsProperty> WingsAnimator<P> {
    /// Creates an animator that will animate to `to` starting from the current component value.
    pub fn to(to: P::Value, spec: AnimationSpec) -> Self {
        let mut a = Self::default();
        a.request(None, to, spec, Repeat::ONCE);
        a
    }

    /// Creates an animator with an explicit start value and repetition.
    pub fn from_to(from: P::Value, to: P::Value, spec: AnimationSpec, repeat: Repeat) -> Self {
        let mut a = Self::default();
        a.request(Some(from), to, spec, repeat);
        a
    }

    /// Queues a new animation. It replaces any running one on the next tick.
    pub fn request(
        &mut self,
        from: Option<P::Value>,
        to: P::Value,
        spec: AnimationSpec,
        repeat: Repeat,
    ) {
        self.request = Some(Request {
            from,
            to,
            spec,
            repeat,
        });
    }

    /// True while an animation is queued or running.
    pub fn is_active(&self) -> bool {
        self.request.is_some() || self.value.as_ref().is_some_and(|v| v.is_running())
    }

    /// The target value of the running or queued animation.
    pub fn target(&self) -> Option<&P::Value> {
        self.request
            .as_ref()
            .map(|r| &r.to)
            .or_else(|| self.value.as_ref().map(|v| v.target()))
    }

    /// Stops the animation where it is.
    pub fn stop(&mut self) {
        self.request = None;
        if let Some(v) = &mut self.value {
            let current = v.value().clone();
            v.snap_to(current);
        }
    }
}

/// Triggered on an entity when one of its property animations completes.
#[derive(EntityEvent, Debug, Clone)]
pub struct WingsAnimationFinished {
    /// The animated entity.
    pub entity: Entity,
    /// Type name of the [`WingsProperty`] that finished.
    pub property: &'static str,
}

/// The per property animation system.
pub fn drive_property<P: WingsProperty>(
    time: Res<Time>,
    mut query: Query<(Entity, &mut WingsAnimator<P>, &mut P::Target)>,
    mut commands: Commands,
) {
    let dt = time.delta_secs();
    for (entity, mut animator, mut target) in &mut query {
        if !animator.is_active() {
            continue;
        }
        let animator = &mut *animator;
        if let Some(req) = animator.request.take() {
            let value = animator
                .value
                .get_or_insert_with(|| WingsAnimatedValue::new(P::get(&target)));
            match req.from {
                Some(from) => value.animate_from_to(from, req.to, req.spec, req.repeat),
                None => value.animate_to_with_repeat(req.to, req.spec, req.repeat),
            }
            if !value.is_running() {
                // Snap requests finish immediately.
                P::set(&mut target, value.value().clone());
                commands.trigger(WingsAnimationFinished {
                    entity,
                    property: core::any::type_name::<P>(),
                });
                continue;
            }
        }
        let Some(value) = animator.value.as_mut() else {
            continue;
        };
        let result = value.tick(dt);
        if result.changed {
            let v = value.value().clone();
            if P::get(&target) != v {
                P::set(&mut target, v);
            }
        }
        if result.finished {
            commands.trigger(WingsAnimationFinished {
                entity,
                property: core::any::type_name::<P>(),
            });
        }
    }
}

/// Extension methods to start animations from [`EntityCommands`].
pub trait WingsAnimateExt {
    /// Animates property `P` from its current value to `to`.
    fn animate<P: WingsProperty>(&mut self, to: P::Value, spec: AnimationSpec) -> &mut Self;

    /// Animates property `P` from `from` to `to` with repetition (pulses, loaders).
    fn animate_from_to<P: WingsProperty>(
        &mut self,
        from: P::Value,
        to: P::Value,
        spec: AnimationSpec,
        repeat: Repeat,
    ) -> &mut Self;

    /// Stops the animation of property `P`, leaving the value where it is.
    fn stop_animation<P: WingsProperty>(&mut self) -> &mut Self;
}

impl WingsAnimateExt for EntityCommands<'_> {
    fn animate<P: WingsProperty>(&mut self, to: P::Value, spec: AnimationSpec) -> &mut Self {
        self.queue(move |mut e: EntityWorldMut| {
            request_on::<P>(&mut e, None, to, spec, Repeat::ONCE);
        })
    }

    fn animate_from_to<P: WingsProperty>(
        &mut self,
        from: P::Value,
        to: P::Value,
        spec: AnimationSpec,
        repeat: Repeat,
    ) -> &mut Self {
        self.queue(move |mut e: EntityWorldMut| {
            request_on::<P>(&mut e, Some(from), to, spec, repeat);
        })
    }

    fn stop_animation<P: WingsProperty>(&mut self) -> &mut Self {
        self.queue(|mut e: EntityWorldMut| {
            if let Some(mut a) = e.get_mut::<WingsAnimator<P>>() {
                a.stop();
            }
        })
    }
}

/// Queues an animation request on an entity, inserting the animator when missing.
pub fn request_on<P: WingsProperty>(
    entity: &mut EntityWorldMut,
    from: Option<P::Value>,
    to: P::Value,
    spec: AnimationSpec,
    repeat: Repeat,
) {
    if let Some(mut a) = entity.get_mut::<WingsAnimator<P>>() {
        a.request(from, to, spec, repeat);
    } else {
        let mut a = WingsAnimator::<P>::default();
        a.request(from, to, spec, repeat);
        entity.insert(a);
    }
}

/// App extension to register custom animatable properties.
pub trait WingsAnimationAppExt {
    /// Registers the system driving property `P`. Safe to call more than once.
    fn register_wings_property<P: WingsProperty>(&mut self) -> &mut Self;
}

#[derive(Resource, Default)]
struct RegisteredProperties(bevy::platform::collections::HashSet<core::any::TypeId>);

impl WingsAnimationAppExt for App {
    fn register_wings_property<P: WingsProperty>(&mut self) -> &mut Self {
        let id = core::any::TypeId::of::<P>();
        let mut registered = self
            .world_mut()
            .get_resource_or_insert_with(RegisteredProperties::default);
        if !registered.0.insert(id) {
            return self;
        }
        self.add_systems(
            Update,
            drive_property::<P>.in_set(dga_bevy_wings_ui_core::WingsUiSystems::Animate),
        )
    }
}
