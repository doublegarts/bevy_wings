//! Modules: groups of definitions.

use core::any::{Any, TypeId};
use std::sync::Arc;

use crate::container::Resolver;

/// Erased, cloneable storage for an `Arc<T>` (where `T` may be a trait object).
pub(crate) type Erased = Arc<dyn Any + Send + Sync>;

/// Erased constructor.
pub(crate) type Ctor = Arc<dyn Fn(&Resolver) -> Erased + Send + Sync>;

/// Erased constructor taking parameters.
pub(crate) type ParamCtor = Arc<dyn Fn(&Resolver, Box<dyn Any>) -> Erased + Send + Sync>;

/// How long a resolved instance lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Lifetime {
    /// One shared instance for the whole app, created on first use (or at startup when eager).
    Single,
    /// A new instance every time it is requested.
    Factory,
    /// One instance per scope (for example per screen or per navigation entry).
    Scoped,
}

/// Identifies a definition: a type plus an optional qualifier name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DefKey {
    pub(crate) type_id: TypeId,
    pub(crate) qualifier: Option<&'static str>,
}

impl DefKey {
    /// Key for `T` with an optional qualifier.
    pub fn of<T: ?Sized + 'static>(qualifier: Option<&'static str>) -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            qualifier,
        }
    }
}

/// A single definition.
#[derive(Clone)]
pub struct Definition {
    pub(crate) key: DefKey,
    pub(crate) type_name: &'static str,
    pub(crate) lifetime: Lifetime,
    pub(crate) ctor: Ctor,
    pub(crate) eager: bool,
}

impl Definition {
    /// Lifetime of the definition.
    pub fn lifetime(&self) -> Lifetime {
        self.lifetime
    }

    /// Type name of the provided value.
    pub fn type_name(&self) -> &'static str {
        self.type_name
    }

    /// Qualifier of the definition.
    pub fn qualifier(&self) -> Option<&'static str> {
        self.key.qualifier
    }
}

/// A parameterized factory definition.
#[derive(Clone)]
pub struct ParamDefinition {
    pub(crate) key: (TypeId, TypeId),
    pub(crate) type_name: &'static str,
    pub(crate) ctor: ParamCtor,
}

impl ParamDefinition {
    /// Type name of the produced value.
    pub fn type_name(&self) -> &'static str {
        self.type_name
    }
}

/// A named group of definitions, loaded into a [`WingsDi`](crate::WingsDi) container.
///
/// ```
/// use std::sync::Arc;
/// use bevy_wings_di::prelude::*;
///
/// struct Config { url: String }
/// struct Api { config: Arc<Config> }
///
/// let module = WingsModule::new("network")
///     .single(|_| Config { url: "https://example.com".into() })
///     .single(|r| Api { config: r.get() });
///
/// let di = WingsDi::from_modules([module]).unwrap();
/// assert_eq!(di.get::<Api>().config.url, "https://example.com");
/// ```
#[derive(Clone)]
pub struct WingsModule {
    pub(crate) name: &'static str,
    pub(crate) definitions: Vec<Definition>,
    pub(crate) params: Vec<ParamDefinition>,
    pub(crate) allow_override: bool,
}

impl WingsModule {
    /// Creates an empty module.
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            definitions: Vec::new(),
            params: Vec::new(),
            allow_override: false,
        }
    }

    /// Module name (used in diagnostics).
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// Definitions of this module.
    pub fn definitions(&self) -> &[Definition] {
        &self.definitions
    }

    /// Parameterized factory definitions of this module.
    pub fn param_definitions(&self) -> &[ParamDefinition] {
        &self.params
    }

    /// Lets this module replace definitions loaded by earlier modules (useful for tests and
    /// fakes).
    pub fn allow_override(mut self) -> Self {
        self.allow_override = true;
        self
    }

    fn push<T: Send + Sync + 'static>(
        mut self,
        lifetime: Lifetime,
        qualifier: Option<&'static str>,
        eager: bool,
        ctor: impl Fn(&Resolver) -> T + Send + Sync + 'static,
    ) -> Self {
        self.definitions.push(Definition {
            key: DefKey::of::<T>(qualifier),
            type_name: core::any::type_name::<T>(),
            lifetime,
            ctor: Arc::new(move |r| Arc::new(Arc::new(ctor(r))) as Erased),
            eager,
        });
        self
    }

    fn push_arc<T: ?Sized + Send + Sync + 'static>(
        mut self,
        lifetime: Lifetime,
        qualifier: Option<&'static str>,
        ctor: impl Fn(&Resolver) -> Arc<T> + Send + Sync + 'static,
    ) -> Self {
        self.definitions.push(Definition {
            key: DefKey::of::<T>(qualifier),
            type_name: core::any::type_name::<T>(),
            lifetime,
            ctor: Arc::new(move |r| Arc::new(ctor(r)) as Erased),
            eager: false,
        });
        self
    }

    /// A lazily created singleton.
    pub fn single<T: Send + Sync + 'static>(
        self,
        ctor: impl Fn(&Resolver) -> T + Send + Sync + 'static,
    ) -> Self {
        self.push(Lifetime::Single, None, false, ctor)
    }

    /// A singleton created when the app starts.
    pub fn single_eager<T: Send + Sync + 'static>(
        self,
        ctor: impl Fn(&Resolver) -> T + Send + Sync + 'static,
    ) -> Self {
        self.push(Lifetime::Single, None, true, ctor)
    }

    /// A named singleton, for several instances of the same type.
    pub fn single_named<T: Send + Sync + 'static>(
        self,
        qualifier: &'static str,
        ctor: impl Fn(&Resolver) -> T + Send + Sync + 'static,
    ) -> Self {
        self.push(Lifetime::Single, Some(qualifier), false, ctor)
    }

    /// Registers an existing value as a singleton.
    pub fn instance<T: Clone + Send + Sync + 'static>(self, value: T) -> Self {
        self.single(move |_| value.clone())
    }

    /// A factory: a fresh instance per request.
    pub fn factory<T: Send + Sync + 'static>(
        self,
        ctor: impl Fn(&Resolver) -> T + Send + Sync + 'static,
    ) -> Self {
        self.push(Lifetime::Factory, None, false, ctor)
    }

    /// A named factory.
    pub fn factory_named<T: Send + Sync + 'static>(
        self,
        qualifier: &'static str,
        ctor: impl Fn(&Resolver) -> T + Send + Sync + 'static,
    ) -> Self {
        self.push(Lifetime::Factory, Some(qualifier), false, ctor)
    }

    /// One instance per scope.
    pub fn scoped<T: Send + Sync + 'static>(
        self,
        ctor: impl Fn(&Resolver) -> T + Send + Sync + 'static,
    ) -> Self {
        self.push(Lifetime::Scoped, None, false, ctor)
    }

    /// Binds an interface (trait object) to an implementation as a singleton:
    /// `.bind::<dyn Repository>(|r| Arc::new(SqlRepository::new(r.get())))`.
    pub fn bind<T: ?Sized + Send + Sync + 'static>(
        self,
        ctor: impl Fn(&Resolver) -> Arc<T> + Send + Sync + 'static,
    ) -> Self {
        self.push_arc(Lifetime::Single, None, ctor)
    }

    /// Binds an interface to an implementation as a factory.
    pub fn bind_factory<T: ?Sized + Send + Sync + 'static>(
        self,
        ctor: impl Fn(&Resolver) -> Arc<T> + Send + Sync + 'static,
    ) -> Self {
        self.push_arc(Lifetime::Factory, None, ctor)
    }

    /// A factory that receives runtime parameters of type `P` (for example an item id).
    /// Resolve it with [`Resolver::get_with`] or [`WingsDi::get_with`](crate::WingsDi::get_with).
    pub fn factory_with<T: Send + Sync + 'static, P: 'static>(
        mut self,
        ctor: impl Fn(&Resolver, P) -> T + Send + Sync + 'static,
    ) -> Self {
        self.params.push(ParamDefinition {
            key: (TypeId::of::<T>(), TypeId::of::<P>()),
            type_name: core::any::type_name::<T>(),
            ctor: Arc::new(move |r, p| {
                let p = *p.downcast::<P>().expect("parameter type checked by key");
                Arc::new(Arc::new(ctor(r, p))) as Erased
            }),
        });
        self
    }
}
