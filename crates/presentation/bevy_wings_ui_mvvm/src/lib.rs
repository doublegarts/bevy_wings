//! # bevy_wings_ui_mvvm
//!
//! Model, View, ViewModel for Bevy UI, built on plain ECS concepts:
//!
//! * **View model** = a component implementing [`WingsViewModel`], placed on a screen root.
//!   Bevy change detection *is* the reactivity: mutate the view model and every binding in its
//!   subtree refreshes in the same frame. Because the view model lives on an entity, its
//!   lifetime is the screen's lifetime: it survives while the screen is kept in a navigation
//!   back stack and [`WingsViewModel::on_cleared`] runs when the screen is despawned.
//! * **Bindings** ([`WingsBindText`], [`WingsBindVisible`], [`WingsBindEnabled`],
//!   [`WingsBindChecked`] and the generic [`WingsBind`]) are components on view entities. They
//!   find the closest view model of their type up the hierarchy, recompute only when that view
//!   model changed, and write only when the computed value differs.
//! * **Intents** are observers created with [`on_vm`] / [`on_activate`] that forward UI events
//!   to view model methods.
//! * **Effects** ([`WingsEffects`]) carry one shot events (show a snackbar, navigate) from a
//!   view model to the rest of the app as Bevy messages.
//! * **Dependency injection**: [`WingsViewModelModuleExt::view_model`] declares how to build a
//!   view model from the DI container, and [`WingsProvideViewModel`] creates it on spawn.
//!
//! ```ignore
//! #[derive(Component, Default, Clone)]
//! struct CounterVm { count: i32 }
//! impl WingsViewModel for CounterVm {}
//!
//! app.register_view_model::<CounterVm>();
//!
//! let label = bind_text(|vm: &CounterVm| format!("Count: {}", vm.count));
//! commands.spawn_scene(bsn! {
//!     CounterVm
//!     Children [
//!         Text label
//!         --
//!         Button on(on_activate(|vm: &mut CounterVm| vm.count += 1))
//!     ]
//! });
//! ```

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/doublegarts/bevy_wings/main/assets/branding/bevy-wings-icon.png",
    html_favicon_url = "https://raw.githubusercontent.com/doublegarts/bevy_wings/main/assets/branding/bevy-wings-icon.png"
)]

pub mod loadable;

use core::marker::PhantomData;
use std::sync::{Arc, Mutex};

use bevy::ecs::component::Mutable;
use bevy::ecs::event::EntityEvent;
use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;
use bevy::ui::{Checked, Display, InteractionDisabled, Node};
use bevy::ui_widgets::Activate;

use bevy_wings_di::{ScopeId, WingsDi, WingsDiScope, WingsModule};
use bevy_wings_ui_core::hierarchy::find_ancestor_with;
use bevy_wings_ui_core::{WingsUiSystems, ensure_core};

/// A view model: UI state and logic for one screen or component.
///
/// Implement it on a component. All methods have defaults.
pub trait WingsViewModel: Component<Mutability = Mutable> {
    /// Called once when the owning entity is despawned or the component removed. Cancel work,
    /// flush state, release resources here.
    fn on_cleared(&mut self) {}
}

type Selector<VM, T> = Arc<dyn Fn(&VM) -> T + Send + Sync>;

/// Shared state of every binding: the selector, the resolved owner, and the last value.
pub struct BindingCore<VM, T> {
    selector: Option<Selector<VM, T>>,
    owner: Option<Entity>,
    last: Option<T>,
}

impl<VM, T: Clone> Clone for BindingCore<VM, T> {
    fn clone(&self) -> Self {
        Self {
            selector: self.selector.clone(),
            owner: None,
            last: None,
        }
    }
}

impl<VM, T> Default for BindingCore<VM, T> {
    fn default() -> Self {
        Self {
            selector: None,
            owner: None,
            last: None,
        }
    }
}

impl<VM: 'static, T: PartialEq + Clone> BindingCore<VM, T> {
    fn new(f: impl Fn(&VM) -> T + Send + Sync + 'static) -> Self {
        Self {
            selector: Some(Arc::new(f)),
            owner: None,
            last: None,
        }
    }

    /// Computes the selector and returns the new value when it differs from the last one.
    fn evaluate(&mut self, vm: &VM) -> Option<T> {
        let value = (self.selector.as_ref()?)(vm);
        if self.last.as_ref() == Some(&value) {
            return None;
        }
        self.last = Some(value.clone());
        Some(value)
    }
}

macro_rules! binding_component {
    ($(#[$doc:meta])* $name:ident, $t:ty, $ctor:ident) => {
        $(#[$doc])*
        #[derive(Component)]
        pub struct $name<VM: WingsViewModel>(pub BindingCore<VM, $t>);

        impl<VM: WingsViewModel> Default for $name<VM> {
            fn default() -> Self {
                Self(BindingCore::default())
            }
        }

        impl<VM: WingsViewModel> Clone for $name<VM> {
            fn clone(&self) -> Self {
                Self(self.0.clone())
            }
        }

        impl<VM: WingsViewModel> $name<VM> {
            /// Creates the binding from a selector.
            pub fn new(f: impl Fn(&VM) -> $t + Send + Sync + 'static) -> Self {
                Self(BindingCore::new(f))
            }
        }

        /// Shorthand constructor.
        pub fn $ctor<VM: WingsViewModel>(f: impl Fn(&VM) -> $t + Send + Sync + 'static) -> $name<VM> {
            $name::new(f)
        }
    };
}

binding_component!(
    /// Binds a [`Text`] to a string computed from the view model.
    WingsBindText, String, bind_text
);
binding_component!(
    /// Shows (`true`) or collapses (`false`) the node.
    WingsBindVisible, bool, bind_visible
);
binding_component!(
    /// Enables (`true`) or disables (`false`) interaction ([`InteractionDisabled`]).
    WingsBindEnabled, bool, bind_enabled
);
binding_component!(
    /// Adds or removes [`Checked`] (checkboxes, switches, radio buttons).
    WingsBindChecked, bool, bind_checked
);

/// Binds any component `C` to a value computed from the view model. Register the pair with
/// [`WingsMvvmAppExt::register_binding`].
#[derive(Component)]
pub struct WingsBind<VM: WingsViewModel, C: Component + PartialEq + Clone>(pub BindingCore<VM, C>);

impl<VM: WingsViewModel, C: Component + PartialEq + Clone> Default for WingsBind<VM, C> {
    fn default() -> Self {
        Self(BindingCore::default())
    }
}

impl<VM: WingsViewModel, C: Component + PartialEq + Clone> Clone for WingsBind<VM, C> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<VM: WingsViewModel, C: Component + PartialEq + Clone> WingsBind<VM, C> {
    /// Creates the binding.
    pub fn new(f: impl Fn(&VM) -> C + Send + Sync + 'static) -> Self {
        Self(BindingCore::new(f))
    }
}

/// Shorthand for [`WingsBind::new`].
pub fn bind<VM: WingsViewModel, C: Component + PartialEq + Clone>(
    f: impl Fn(&VM) -> C + Send + Sync + 'static,
) -> WingsBind<VM, C> {
    WingsBind::new(f)
}

/// Resolves the owner view model of a binding (cached after the first lookup) and returns it
/// together with whether the binding must be refreshed.
fn owner_state<'a, VM: WingsViewModel, T: PartialEq + Clone>(
    entity: Entity,
    core: &mut BindingCore<VM, T>,
    added: bool,
    parents: &Query<&ChildOf>,
    vms: &'a Query<Ref<VM>>,
) -> Option<(&'a VM, bool)> {
    let owner = match core.owner.filter(|o| vms.contains(*o)) {
        Some(o) => o,
        None => {
            let o = find_ancestor_with(entity, parents, vms)?;
            core.owner = Some(o);
            core.last = None;
            o
        }
    };
    let vm = vms.get(owner).ok()?;
    let refresh = added || core.last.is_none() || vm.is_changed();
    Some((vm.into_inner(), refresh))
}

/// Refreshes text bindings.
pub fn sync_text<VM: WingsViewModel>(
    parents: Query<&ChildOf>,
    vms: Query<Ref<VM>>,
    mut bindings: Query<(Entity, &mut WingsBindText<VM>, &mut Text)>,
) {
    for (e, mut binding, mut text) in &mut bindings {
        let added = binding.is_added();
        let core = &mut binding.bypass_change_detection().0;
        if let Some((vm, true)) = owner_state(e, core, added, &parents, &vms)
            && let Some(v) = core.evaluate(vm)
        {
            text.0 = v;
        }
    }
}

/// Refreshes visibility bindings.
pub fn sync_visible<VM: WingsViewModel>(
    parents: Query<&ChildOf>,
    vms: Query<Ref<VM>>,
    mut bindings: Query<(Entity, &mut WingsBindVisible<VM>, &mut Node)>,
) {
    for (e, mut binding, mut node) in &mut bindings {
        let added = binding.is_added();
        let core = &mut binding.bypass_change_detection().0;
        if let Some((vm, true)) = owner_state(e, core, added, &parents, &vms)
            && let Some(v) = core.evaluate(vm)
        {
            node.display = if v { Display::Flex } else { Display::None };
        }
    }
}

/// Refreshes enabled bindings.
pub fn sync_enabled<VM: WingsViewModel>(
    mut commands: Commands,
    parents: Query<&ChildOf>,
    vms: Query<Ref<VM>>,
    mut bindings: Query<(Entity, &mut WingsBindEnabled<VM>)>,
) {
    for (e, mut binding) in &mut bindings {
        let added = binding.is_added();
        let core = &mut binding.bypass_change_detection().0;
        if let Some((vm, true)) = owner_state(e, core, added, &parents, &vms)
            && let Some(enabled) = core.evaluate(vm)
        {
            if enabled {
                commands.entity(e).remove::<InteractionDisabled>();
            } else {
                commands.entity(e).insert(InteractionDisabled);
            }
        }
    }
}

/// Refreshes checked bindings.
pub fn sync_checked<VM: WingsViewModel>(
    mut commands: Commands,
    parents: Query<&ChildOf>,
    vms: Query<Ref<VM>>,
    mut bindings: Query<(Entity, &mut WingsBindChecked<VM>)>,
) {
    for (e, mut binding) in &mut bindings {
        let added = binding.is_added();
        let core = &mut binding.bypass_change_detection().0;
        if let Some((vm, true)) = owner_state(e, core, added, &parents, &vms)
            && let Some(checked) = core.evaluate(vm)
        {
            if checked {
                commands.entity(e).insert(Checked);
            } else {
                commands.entity(e).remove::<Checked>();
            }
        }
    }
}

/// Refreshes generic component bindings.
pub fn sync_component<VM: WingsViewModel, C: Component + PartialEq + Clone>(
    mut commands: Commands,
    parents: Query<&ChildOf>,
    vms: Query<Ref<VM>>,
    mut bindings: Query<(Entity, &mut WingsBind<VM, C>)>,
) {
    for (e, mut binding) in &mut bindings {
        let added = binding.is_added();
        let core = &mut binding.bypass_change_detection().0;
        if let Some((vm, true)) = owner_state(e, core, added, &parents, &vms)
            && let Some(value) = core.evaluate(vm)
        {
            commands.entity(e).insert(value);
        }
    }
}

/// Creates an observer that forwards entity event `E` to the closest view model `VM` of the
/// event target.
///
/// ```ignore
/// bsn! { Slider on(on_vm(|vm: &mut PlayerVm, e: &ValueChange<f32>| vm.set_volume(e.value))) }
/// ```
pub fn on_vm<VM: WingsViewModel, E: EntityEvent>(
    f: impl Fn(&mut VM, &E) + Send + Sync + Clone + 'static,
) -> impl FnMut(On<E>, Query<&mut VM>, Query<&ChildOf>) + Clone {
    move |event: On<E>, mut vms: Query<&mut VM>, parents: Query<&ChildOf>| {
        let target = event.event_target();
        if let Some(owner) = find_ancestor_with(target, &parents, &vms)
            && let Ok(mut vm) = vms.get_mut(owner)
        {
            f(&mut vm, event.event());
        }
    }
}

/// Creates an observer that calls `f` on the closest view model when a button is activated.
pub fn on_activate<VM: WingsViewModel>(
    f: impl Fn(&mut VM) + Send + Sync + Clone + 'static,
) -> impl FnMut(On<Activate>, Query<&mut VM>, Query<&ChildOf>) + Clone {
    on_vm::<VM, Activate>(move |vm, _| f(vm))
}

/// A queue of one shot effects emitted by a view model. Store it as a field and expose it via
/// [`WingsHasEffects`]; registered queues are drained into Bevy messages every frame.
#[derive(Debug, Clone)]
pub struct WingsEffects<E> {
    queue: Vec<E>,
}

impl<E> Default for WingsEffects<E> {
    fn default() -> Self {
        Self { queue: Vec::new() }
    }
}

impl<E> WingsEffects<E> {
    /// Emits an effect.
    pub fn send(&mut self, effect: E) {
        self.queue.push(effect);
    }

    /// Takes all pending effects.
    pub fn drain(&mut self) -> Vec<E> {
        core::mem::take(&mut self.queue)
    }

    /// Number of pending effects.
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// True when nothing is pending.
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

/// Exposes the effect queue of a view model.
pub trait WingsHasEffects<E> {
    /// The queue.
    fn effects(&mut self) -> &mut WingsEffects<E>;
}

/// Drains effect queues into messages, without marking view models as changed when empty.
pub fn drain_effects<VM: WingsViewModel + WingsHasEffects<E>, E: Message>(
    mut vms: Query<&mut VM>,
    mut writer: MessageWriter<E>,
) {
    for mut vm in &mut vms {
        if vm.bypass_change_detection().effects().is_empty() {
            continue;
        }
        writer.write_batch(vm.bypass_change_detection().effects().drain());
    }
}

/// Holder used to move an owned view model out of the DI container.
struct ViewModelSlot<VM>(Mutex<Option<VM>>);

/// DI module extension to declare view model factories.
pub trait WingsViewModelModuleExt {
    /// Declares how to build `VM` from the container. Each request builds a fresh instance.
    fn view_model<VM: WingsViewModel + Send + Sync>(
        self,
        ctor: impl Fn(&bevy_wings_di::Resolver) -> VM + Send + Sync + 'static,
    ) -> Self;
}

impl WingsViewModelModuleExt for WingsModule {
    fn view_model<VM: WingsViewModel + Send + Sync>(
        self,
        ctor: impl Fn(&bevy_wings_di::Resolver) -> VM + Send + Sync + 'static,
    ) -> Self {
        self.factory(move |r| ViewModelSlot(Mutex::new(Some(ctor(r)))))
    }
}

/// Creates view model `VM` from the DI container when spawned. If the entity (or an ancestor)
/// has a [`WingsDiScope`], scoped dependencies are resolved in that scope.
#[derive(Component)]
pub struct WingsProvideViewModel<VM: WingsViewModel>(PhantomData<fn() -> VM>);

impl<VM: WingsViewModel> Default for WingsProvideViewModel<VM> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<VM: WingsViewModel> Clone for WingsProvideViewModel<VM> {
    fn clone(&self) -> Self {
        Self(PhantomData)
    }
}

fn provide_view_model<VM: WingsViewModel + Send + Sync>(
    add: On<Add<WingsProvideViewModel<VM>>>,
    di: Option<Res<WingsDi>>,
    parents: Query<&ChildOf>,
    scopes: Query<(), With<WingsDiScope>>,
    mut commands: Commands,
) {
    let Some(di) = di else {
        bevy::log::error!(
            "WingsProvideViewModel<{}> needs a WingsDi container",
            core::any::type_name::<VM>()
        );
        return;
    };
    let resolver = match find_ancestor_with(add.entity, &parents, &scopes) {
        Some(scope) => di.scope(ScopeId::Entity(scope)),
        None => di.resolver(),
    };
    match resolver.try_get::<ViewModelSlot<VM>>() {
        Ok(slot) => {
            if let Some(vm) = slot.0.lock().expect("slot lock").take() {
                commands.entity(add.entity).insert(vm);
            }
        }
        Err(e) => bevy::log::error!("cannot provide view model: {e}"),
    }
}

fn clear_view_model<VM: WingsViewModel>(mut world: DeferredWorld, ctx: HookContext) {
    if let Some(mut vm) = world.get_mut::<VM>(ctx.entity) {
        vm.on_cleared();
    }
}

/// App extension to register view models and bindings.
pub trait WingsMvvmAppExt {
    /// Registers a view model type: lifecycle hook, provider observer and the built in
    /// binding systems.
    fn register_view_model<VM: WingsViewModel + Send + Sync>(&mut self) -> &mut Self;

    /// Enables [`WingsBind<VM, C>`] for component `C`.
    fn register_binding<VM: WingsViewModel, C: Component + PartialEq + Clone>(
        &mut self,
    ) -> &mut Self;

    /// Drains `VM`'s effect queue of type `E` into `Messages<E>` every frame.
    fn register_effects<VM: WingsViewModel + WingsHasEffects<E>, E: Message>(
        &mut self,
    ) -> &mut Self;
}

impl WingsMvvmAppExt for App {
    fn register_view_model<VM: WingsViewModel + Send + Sync>(&mut self) -> &mut Self {
        ensure_core(self);
        self.world_mut()
            .register_component_hooks::<VM>()
            .on_remove(clear_view_model::<VM>);
        self.add_observer(provide_view_model::<VM>);
        self.add_systems(
            Update,
            (
                sync_text::<VM>,
                sync_visible::<VM>,
                sync_enabled::<VM>,
                sync_checked::<VM>,
            )
                .in_set(WingsUiSystems::Bind),
        )
    }

    fn register_binding<VM: WingsViewModel, C: Component + PartialEq + Clone>(
        &mut self,
    ) -> &mut Self {
        ensure_core(self);
        self.add_systems(Update, sync_component::<VM, C>.in_set(WingsUiSystems::Bind))
    }

    fn register_effects<VM: WingsViewModel + WingsHasEffects<E>, E: Message>(
        &mut self,
    ) -> &mut Self {
        ensure_core(self);
        self.add_message::<E>();
        self.add_systems(Update, drain_effects::<VM, E>.in_set(WingsUiSystems::Bind))
    }
}

/// Commonly used items.
pub mod prelude {
    pub use crate::loadable::{
        WingsLoadPhase, bind_error_text, bind_failed, bind_loaded, bind_loading, bind_refreshing,
    };
    pub use crate::{
        WingsBind, WingsBindChecked, WingsBindEnabled, WingsBindText, WingsBindVisible,
        WingsEffects, WingsHasEffects, WingsMvvmAppExt, WingsProvideViewModel, WingsViewModel,
        WingsViewModelModuleExt, bind, bind_checked, bind_enabled, bind_text, bind_visible,
        on_activate, on_vm,
    };
}

#[cfg(test)]
mod tests {
    use super::prelude::*;
    use super::*;
    use bevy::ui_widgets::ValueChange;
    use bevy_wings_di::{WingsDiAppExt, WingsModule};
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Component, Default)]
    struct CounterVm {
        count: i32,
        effects: WingsEffects<Toast>,
    }

    static CLEARED: AtomicUsize = AtomicUsize::new(0);

    impl WingsViewModel for CounterVm {
        fn on_cleared(&mut self) {
            CLEARED.fetch_add(1, Ordering::SeqCst);
        }
    }

    impl WingsHasEffects<Toast> for CounterVm {
        fn effects(&mut self) -> &mut WingsEffects<Toast> {
            &mut self.effects
        }
    }

    #[derive(Message, Debug, Clone, PartialEq)]
    struct Toast(String);

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.register_view_model::<CounterVm>();
        app
    }

    #[derive(Component, Default, Clone)]
    struct ScoreVm {
        score: u32,
    }
    impl WingsViewModel for ScoreVm {}

    /// View models and bindings work together inside `bsn!` scenes: the view model is a component of
    /// the screen root and bindings are template values on descendants.
    #[test]
    fn bindings_work_in_bsn_scenes() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            bevy::asset::AssetPlugin::default(),
            bevy::scene::ScenePlugin,
        ));
        app.register_view_model::<ScoreVm>();
        let label = bind_text(|vm: &ScoreVm| format!("Score {}", vm.score));
        let screen = app
            .world_mut()
            .spawn_scene(bsn! {
                ScoreVm { score: 3 }
                Children [ Text label ]
            })
            .unwrap()
            .id();
        app.update();
        let child = app.world().get::<Children>(screen).unwrap()[0];
        assert_eq!(text(&app, child), "Score 3");
        app.world_mut().get_mut::<ScoreVm>(screen).unwrap().score = 10;
        app.update();
        assert_eq!(text(&app, child), "Score 10");
    }

    fn text(app: &App, e: Entity) -> String {
        app.world().get::<Text>(e).unwrap().0.clone()
    }

    /// A text binding renders the initial state and follows view model changes.
    #[test]
    fn text_binding_follows_vm() {
        let mut app = app();
        let screen = app.world_mut().spawn(CounterVm::default()).id();
        let label = app
            .world_mut()
            .spawn((
                ChildOf(screen),
                Text::default(),
                bind_text(|vm: &CounterVm| format!("Count: {}", vm.count)),
            ))
            .id();
        app.update();
        assert_eq!(text(&app, label), "Count: 0");
        app.world_mut().get_mut::<CounterVm>(screen).unwrap().count = 5;
        app.update();
        assert_eq!(text(&app, label), "Count: 5");
    }

    /// Unchanged values do not rewrite the bound component (no layout churn).
    #[test]
    fn unchanged_value_is_not_written() {
        let mut app = app();
        let screen = app.world_mut().spawn(CounterVm::default()).id();
        let label = app
            .world_mut()
            .spawn((
                ChildOf(screen),
                Text::default(),
                bind_text(|vm: &CounterVm| (vm.count > 10).to_string()),
            ))
            .id();
        app.update();
        let t1 = app
            .world()
            .entity(label)
            .get_ref::<Text>()
            .unwrap()
            .last_changed();
        app.world_mut().get_mut::<CounterVm>(screen).unwrap().count = 3;
        app.update();
        let t2 = app
            .world()
            .entity(label)
            .get_ref::<Text>()
            .unwrap()
            .last_changed();
        assert_eq!(t1, t2);
    }

    /// Bindings resolve the *closest* view model, so nested screens are independent.
    #[test]
    fn closest_view_model_wins() {
        let mut app = app();
        let outer = app
            .world_mut()
            .spawn(CounterVm {
                count: 1,
                ..default()
            })
            .id();
        let inner = app
            .world_mut()
            .spawn((
                ChildOf(outer),
                CounterVm {
                    count: 2,
                    ..default()
                },
            ))
            .id();
        let label = app
            .world_mut()
            .spawn((
                ChildOf(inner),
                Text::default(),
                bind_text(|vm: &CounterVm| vm.count.to_string()),
            ))
            .id();
        app.update();
        assert_eq!(text(&app, label), "2");
    }

    /// Visibility, enabled and checked bindings.
    #[test]
    fn flag_bindings() {
        let mut app = app();
        let screen = app.world_mut().spawn(CounterVm::default()).id();
        let e = app
            .world_mut()
            .spawn((
                ChildOf(screen),
                Node::default(),
                bind_visible(|vm: &CounterVm| vm.count > 0),
                bind_enabled(|vm: &CounterVm| vm.count < 3),
                bind_checked(|vm: &CounterVm| vm.count % 2 == 1),
            ))
            .id();
        app.update();
        assert_eq!(app.world().get::<Node>(e).unwrap().display, Display::None);
        assert!(!app.world().entity(e).contains::<Checked>());
        app.world_mut().get_mut::<CounterVm>(screen).unwrap().count = 3;
        app.update();
        assert_eq!(app.world().get::<Node>(e).unwrap().display, Display::Flex);
        assert!(app.world().entity(e).contains::<InteractionDisabled>());
        assert!(app.world().entity(e).contains::<Checked>());
    }

    #[derive(Component, Clone, PartialEq, Debug)]
    struct Progress(f32);

    /// Generic bindings insert any component.
    #[test]
    fn generic_binding() {
        let mut app = app();
        app.register_binding::<CounterVm, Progress>();
        let screen = app
            .world_mut()
            .spawn(CounterVm {
                count: 5,
                ..default()
            })
            .id();
        let bar = app
            .world_mut()
            .spawn((
                ChildOf(screen),
                bind(|vm: &CounterVm| Progress(vm.count as f32 / 10.0)),
            ))
            .id();
        app.update();
        assert_eq!(app.world().get::<Progress>(bar), Some(&Progress(0.5)));
    }

    /// Intents forward activation events to the view model.
    #[test]
    fn activate_intent_updates_vm() {
        let mut app = app();
        let screen = app.world_mut().spawn(CounterVm::default()).id();
        let button = app.world_mut().spawn(ChildOf(screen)).id();
        app.world_mut()
            .entity_mut(button)
            .observe(on_activate(|vm: &mut CounterVm| vm.count += 1));
        app.world_mut().trigger(Activate { entity: button });
        app.world_mut().trigger(Activate { entity: button });
        assert_eq!(app.world().get::<CounterVm>(screen).unwrap().count, 2);
    }

    /// Typed intents receive the event payload.
    #[test]
    fn value_change_intent() {
        let mut app = app();
        let screen = app.world_mut().spawn(CounterVm::default()).id();
        let slider = app.world_mut().spawn(ChildOf(screen)).id();
        app.world_mut().entity_mut(slider).observe(on_vm(
            |vm: &mut CounterVm, e: &ValueChange<f32>| vm.count = e.value as i32,
        ));
        app.world_mut().trigger(ValueChange {
            source: slider,
            value: 42.0f32,
            is_final: true,
        });
        assert_eq!(app.world().get::<CounterVm>(screen).unwrap().count, 42);
    }

    /// `on_cleared` runs when the screen is despawned.
    #[test]
    fn on_cleared_runs_on_despawn() {
        let mut app = app();
        let before = CLEARED.load(Ordering::SeqCst);
        let screen = app.world_mut().spawn(CounterVm::default()).id();
        app.world_mut().despawn(screen);
        assert_eq!(CLEARED.load(Ordering::SeqCst), before + 1);
    }

    #[derive(Resource, Default)]
    struct Toasts(Vec<Toast>);

    /// Effects become messages.
    #[test]
    fn effects_become_messages() {
        let mut app = app();
        app.register_effects::<CounterVm, Toast>();
        app.init_resource::<Toasts>();
        app.add_systems(
            Last,
            |mut r: MessageReader<Toast>, mut t: ResMut<Toasts>| {
                t.0.extend(r.read().cloned());
            },
        );
        let screen = app.world_mut().spawn(CounterVm::default()).id();
        app.world_mut()
            .get_mut::<CounterVm>(screen)
            .unwrap()
            .effects
            .send(Toast("saved".into()));
        app.update();
        assert_eq!(
            app.world().resource::<Toasts>().0,
            vec![Toast("saved".into())]
        );
    }

    struct Repository(i32);

    #[derive(Component)]
    struct DetailVm {
        start: i32,
    }
    impl WingsViewModel for DetailVm {}

    /// View models can be created from the DI container on spawn.
    #[test]
    fn provide_from_di() {
        let mut app = app();
        app.register_view_model::<DetailVm>();
        app.wings_modules([WingsModule::new("vm")
            .instance(std::sync::Arc::new(Repository(7)))
            .view_model(|r| DetailVm {
                start: r.get::<std::sync::Arc<Repository>>().0,
            })]);
        let screen = app
            .world_mut()
            .spawn(WingsProvideViewModel::<DetailVm>::default())
            .id();
        app.update();
        assert_eq!(app.world().get::<DetailVm>(screen).unwrap().start, 7);
    }
}
