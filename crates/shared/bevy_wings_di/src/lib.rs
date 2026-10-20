//! # bevy_wings_di
//!
//! Small, fast, explicit dependency injection for Bevy apps.
//!
//! * [`WingsModule`]: a named group of definitions declared with a fluent builder:
//!   [`single`](WingsModule::single), [`single_eager`](WingsModule::single_eager),
//!   [`factory`](WingsModule::factory), [`scoped`](WingsModule::scoped),
//!   [`bind`](WingsModule::bind) (trait objects), named qualifiers and
//!   [`factory_with`](WingsModule::factory_with) for runtime parameters.
//! * [`WingsDi`]: the container resource, with cycle detection, descriptive errors and a
//!   [`verify`](WingsDi::verify) method to check the whole graph in a unit test.
//! * [`Inject<T>`]: a system parameter that resolves `T` inside any system.
//! * [`WingsDiScope`]: a component that opens a scope tied to an entity (for example a screen).
//!   The scope and its instances are dropped automatically when the entity is despawned.
//!
//! ```
//! use std::sync::Arc;
//! use bevy::prelude::*;
//! use bevy_wings_di::prelude::*;
//!
//! trait Greeter: Send + Sync { fn greet(&self) -> String; }
//! struct English;
//! impl Greeter for English { fn greet(&self) -> String { "hello".into() } }
//!
//! fn app_module() -> WingsModule {
//!     WingsModule::new("app").bind::<dyn Greeter>(|_| Arc::new(English))
//! }
//!
//! fn greet(greeter: Inject<dyn Greeter>) {
//!     assert_eq!(greeter.get().greet(), "hello");
//! }
//!
//! let mut app = App::new();
//! app.add_plugins(WingsDiPlugin::new([app_module()]));
//! app.add_systems(Update, greet);
//! app.update();
//! ```

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/doublegarts/bevy_wings/main/assets/branding/bevy-wings-icon.png",
    html_favicon_url = "https://raw.githubusercontent.com/doublegarts/bevy_wings/main/assets/branding/bevy-wings-icon.png"
)]

mod container;
mod error;
mod module;

use core::marker::PhantomData;
use std::sync::Arc;

use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::system::SystemParam;
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;

pub use container::{Resolver, ScopeId, WingsDi};
pub use error::DiError;
pub use module::{DefKey, Definition, Lifetime, ParamDefinition, WingsModule};

/// Inserts the [`WingsDi`] container built from the given modules and instantiates eager
/// singletons at startup.
pub struct WingsDiPlugin {
    modules: std::sync::Mutex<Vec<WingsModule>>,
}

impl WingsDiPlugin {
    /// Plugin loading the given modules.
    pub fn new(modules: impl IntoIterator<Item = WingsModule>) -> Self {
        Self {
            modules: std::sync::Mutex::new(modules.into_iter().collect()),
        }
    }
}

impl Default for WingsDiPlugin {
    fn default() -> Self {
        Self::new([])
    }
}

impl Plugin for WingsDiPlugin {
    fn build(&self, app: &mut App) {
        let modules = core::mem::take(&mut *self.modules.lock().expect("plugin lock"));
        app.wings_modules(modules);
        app.add_systems(Startup, create_eager_singletons);
    }
}

fn create_eager_singletons(di: Res<WingsDi>) {
    if let Err(e) = di.create_eager() {
        bevy::log::error!("Wings DI: failed to create eager singletons: {e}");
    }
}

/// App extension to load modules.
pub trait WingsDiAppExt {
    /// Loads modules into the app container, creating it when missing.
    ///
    /// # Panics
    /// Panics when a module redefines an existing definition without
    /// [`WingsModule::allow_override`].
    fn wings_modules(&mut self, modules: impl IntoIterator<Item = WingsModule>) -> &mut Self;
}

impl WingsDiAppExt for App {
    fn wings_modules(&mut self, modules: impl IntoIterator<Item = WingsModule>) -> &mut Self {
        let world = self.world_mut();
        if !world.contains_resource::<WingsDi>() {
            world.insert_resource(WingsDi::new());
        }
        let mut di = world.resource_mut::<WingsDi>();
        for m in modules {
            if let Err(e) = di.load(m) {
                panic!("Wings DI: {e}");
            }
        }
        self
    }
}

/// System parameter resolving `T` from the container.
///
/// ```ignore
/// fn load_profile(repo: Inject<dyn ProfileRepository>) {
///     let profile = repo.get().load();
/// }
/// ```
#[derive(SystemParam)]
pub struct Inject<'w, T: ?Sized + Send + Sync + 'static> {
    di: Res<'w, WingsDi>,
    _marker: PhantomData<fn() -> Arc<T>>,
}

impl<T: ?Sized + Send + Sync + 'static> Inject<'_, T> {
    /// Resolves the dependency (cached singletons are a hash lookup).
    pub fn get(&self) -> Arc<T> {
        self.di.get::<T>()
    }

    /// Resolves the dependency, returning an error instead of panicking.
    pub fn try_get(&self) -> Result<Arc<T>, DiError> {
        self.di.try_get::<T>()
    }

    /// Resolves a named variant.
    pub fn named(&self, qualifier: &'static str) -> Arc<T> {
        self.di.get_named::<T>(qualifier)
    }

    /// Resolves within the scope of `entity` (see [`WingsDiScope`]).
    pub fn in_scope(&self, entity: Entity) -> Arc<T> {
        self.di.scope(ScopeId::Entity(entity)).get::<T>()
    }

    /// The container.
    pub fn container(&self) -> &WingsDi {
        &self.di
    }
}

/// Opens a dependency scope owned by this entity. Scoped definitions resolved with
/// `ScopeId::Entity(entity)` live as long as the entity; despawning it closes the scope.
#[derive(Component, Debug, Clone, Copy, Default)]
#[component(on_remove = close_entity_scope)]
pub struct WingsDiScope;

fn close_entity_scope(world: DeferredWorld, ctx: HookContext) {
    if let Some(di) = world.get_resource::<WingsDi>() {
        di.close_scope(ScopeId::Entity(ctx.entity));
    }
}

/// World extension for exclusive systems and tests.
pub trait WingsDiWorldExt {
    /// Resolves `T` from the container.
    fn inject<T: ?Sized + Send + Sync + 'static>(&self) -> Arc<T>;
}

impl WingsDiWorldExt for World {
    fn inject<T: ?Sized + Send + Sync + 'static>(&self) -> Arc<T> {
        self.resource::<WingsDi>().get::<T>()
    }
}

/// Commonly used items.
pub mod prelude {
    pub use crate::{
        DiError, Inject, Lifetime, Resolver, ScopeId, WingsDi, WingsDiAppExt, WingsDiPlugin,
        WingsDiScope, WingsDiWorldExt, WingsModule,
    };
}

#[cfg(test)]
mod tests {
    use super::prelude::*;
    use bevy::prelude::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Config(&'static str);
    struct Api(Arc<Config>);
    struct Repo(Arc<Api>);

    fn graph() -> WingsModule {
        WingsModule::new("graph")
            .single(|_| Config("prod"))
            .single(|r| Api(r.get()))
            .factory(|r| Repo(r.get()))
    }

    /// Singletons are shared, factories are fresh, and dependencies are wired.
    #[test]
    fn single_and_factory_semantics() {
        let di = WingsDi::from_modules([graph()]).unwrap();
        let a = di.get::<Api>();
        let b = di.get::<Api>();
        assert!(Arc::ptr_eq(&a, &b));
        let r1 = di.get::<Repo>();
        let r2 = di.get::<Repo>();
        assert!(!Arc::ptr_eq(&r1, &r2));
        assert!(Arc::ptr_eq(&r1.0, &a));
        assert_eq!(a.0.0, "prod");
    }

    /// Missing definitions produce a descriptive error with the resolution chain.
    #[test]
    fn missing_dependency_reports_chain() {
        let di = WingsDi::from_modules([WingsModule::new("m").single(|r| Api(r.get()))]).unwrap();
        let err = di.try_get::<Api>().err().unwrap();
        match &err {
            DiError::Missing {
                type_name, chain, ..
            } => {
                assert!(type_name.ends_with("Config"));
                assert!(chain[0].ends_with("Api"));
            }
            other => panic!("unexpected {other:?}"),
        }
        assert!(err.to_string().contains("required by"));
    }

    struct A(#[allow(dead_code)] Arc<B>);
    struct B(#[allow(dead_code)] Arc<A>);

    /// Cycles are detected instead of overflowing the stack.
    #[test]
    fn cycles_are_detected() {
        let di = WingsDi::from_modules([WingsModule::new("cycle")
            .single(|r| A(r.get()))
            .single(|r| B(r.get()))])
        .unwrap();
        match di.try_get::<A>() {
            Err(DiError::Cycle { chain }) => {
                assert_eq!(chain.len(), 3);
                assert_eq!(chain.first(), chain.last());
            }
            other => panic!("expected cycle, got {:?}", other.err()),
        }
    }

    /// Duplicates are rejected unless overrides are allowed.
    #[test]
    fn duplicates_and_overrides() {
        let mut di = WingsDi::from_modules([WingsModule::new("a").instance(1u32)]).unwrap();
        assert!(matches!(
            di.load(WingsModule::new("b").instance(2u32)),
            Err(DiError::Duplicate { .. })
        ));
        di.load(WingsModule::new("fake").instance(3u32).allow_override())
            .unwrap();
        assert_eq!(*di.get::<u32>(), 3);
        assert_eq!(di.modules(), &["a", "fake"]);
    }

    /// Qualifiers allow several instances of one type.
    #[test]
    fn named_definitions() {
        let di = WingsDi::from_modules([WingsModule::new("urls")
            .single_named("auth", |_| String::from("https://auth"))
            .single_named("cdn", |_| String::from("https://cdn"))])
        .unwrap();
        assert_eq!(*di.get_named::<String>("auth"), "https://auth");
        assert_eq!(*di.get_named::<String>("cdn"), "https://cdn");
        assert!(di.try_get::<String>().is_err());
    }

    trait Clock: Send + Sync {
        fn now(&self) -> u64;
    }
    struct Fixed(u64);
    impl Clock for Fixed {
        fn now(&self) -> u64 {
            self.0
        }
    }

    /// Trait objects can be bound to implementations.
    #[test]
    fn trait_binding() {
        let di = WingsDi::from_modules([
            WingsModule::new("time").bind::<dyn Clock>(|_| Arc::new(Fixed(42)))
        ])
        .unwrap();
        assert_eq!(di.get::<dyn Clock>().now(), 42);
    }

    /// Scoped instances are shared inside a scope, separate across scopes, and released when
    /// the scope closes.
    #[test]
    fn scoped_lifetime() {
        static CREATED: AtomicUsize = AtomicUsize::new(0);
        struct Session;
        let di = WingsDi::from_modules([WingsModule::new("s").scoped(|_| {
            CREATED.fetch_add(1, Ordering::SeqCst);
            Session
        })])
        .unwrap();
        let s1 = ScopeId::Id(1);
        let s2 = ScopeId::Id(2);
        let a = di.scope(s1).get::<Session>();
        let b = di.scope(s1).get::<Session>();
        let c = di.scope(s2).get::<Session>();
        assert!(Arc::ptr_eq(&a, &b));
        assert!(!Arc::ptr_eq(&a, &c));
        assert_eq!(CREATED.load(Ordering::SeqCst), 2);
        assert!(matches!(
            di.try_get::<Session>(),
            Err(DiError::NoScope { .. })
        ));
        assert!(di.close_scope(s1));
        assert_eq!(di.open_scopes(), 1);
    }

    /// Parameterized factories receive runtime arguments.
    #[test]
    fn factory_with_params() {
        struct Detail {
            id: u32,
            config: Arc<Config>,
        }
        let di = WingsDi::from_modules([graph().factory_with(|r, id: u32| Detail {
            id,
            config: r.get(),
        })])
        .unwrap();
        let d = di.get_with::<Detail, u32>(7);
        assert_eq!(d.id, 7);
        assert_eq!(d.config.0, "prod");
    }

    /// `verify` reports broken definitions without touching the real container.
    #[test]
    fn verify_reports_errors() {
        let ok = WingsDi::from_modules([graph()]).unwrap();
        assert!(ok.verify().is_empty());
        let broken =
            WingsDi::from_modules([WingsModule::new("m").single(|r| Repo(r.get()))]).unwrap();
        let errors = broken.verify();
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0], DiError::Missing { .. }));
    }

    /// The `Inject` system parameter resolves inside systems and eager singletons are built at
    /// startup.
    #[test]
    fn inject_in_systems_and_eager() {
        static EAGER: AtomicUsize = AtomicUsize::new(0);
        struct Analytics;
        #[derive(Resource, Default)]
        struct Seen(&'static str);
        let mut app = App::new();
        app.add_plugins(WingsDiPlugin::new([graph().single_eager(|_| {
            EAGER.fetch_add(1, Ordering::SeqCst);
            Analytics
        })]));
        app.init_resource::<Seen>();
        app.add_systems(Update, |api: Inject<Api>, mut seen: ResMut<Seen>| {
            seen.0 = api.get().0.0;
        });
        app.update();
        assert_eq!(EAGER.load(Ordering::SeqCst), 1);
        assert_eq!(app.world().resource::<Seen>().0, "prod");
        assert_eq!(app.world().inject::<Config>().0, "prod");
    }

    /// Despawning a scope entity closes its scope.
    #[test]
    fn entity_scope_closes_on_despawn() {
        struct ScreenState;
        let mut app = App::new();
        app.add_plugins(WingsDiPlugin::new([
            WingsModule::new("s").scoped(|_| ScreenState)
        ]));
        let screen = app.world_mut().spawn(WingsDiScope).id();
        app.world()
            .resource::<WingsDi>()
            .scope(ScopeId::Entity(screen))
            .get::<ScreenState>();
        assert_eq!(app.world().resource::<WingsDi>().open_scopes(), 1);
        app.world_mut().despawn(screen);
        assert_eq!(app.world().resource::<WingsDi>().open_scopes(), 0);
    }
}
