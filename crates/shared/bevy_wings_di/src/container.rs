//! The container and the resolver.

use core::any::{Any, TypeId};
use core::cell::RefCell;
use std::sync::{Arc, Mutex};

use bevy::ecs::entity::Entity;
use bevy::ecs::resource::Resource;
use bevy::platform::collections::HashMap;

use crate::error::DiError;
use crate::module::{DefKey, Definition, Erased, Lifetime, ParamDefinition, WingsModule};

/// Identifies a scope. Scopes are usually tied to an entity (a screen root or a navigation
/// entry), see [`WingsDiScope`](crate::WingsDiScope).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScopeId {
    /// A scope owned by an entity.
    Entity(Entity),
    /// A scope identified by a number (for scopes not tied to the ECS).
    Id(u64),
}

/// The dependency container. Stored as a Bevy resource.
#[derive(Resource, Default)]
pub struct WingsDi {
    definitions: HashMap<DefKey, Definition>,
    params: HashMap<(TypeId, TypeId), ParamDefinition>,
    singles: Mutex<HashMap<DefKey, Erased>>,
    scopes: Mutex<HashMap<ScopeId, HashMap<DefKey, Erased>>>,
    modules: Vec<&'static str>,
}

impl WingsDi {
    /// An empty container.
    pub fn new() -> Self {
        Self::default()
    }

    /// Builds a container from modules.
    pub fn from_modules(modules: impl IntoIterator<Item = WingsModule>) -> Result<Self, DiError> {
        let mut di = Self::new();
        for m in modules {
            di.load(m)?;
        }
        Ok(di)
    }

    /// Loads a module. Fails on duplicate definitions unless the module allows overrides.
    pub fn load(&mut self, module: WingsModule) -> Result<(), DiError> {
        for def in &module.definitions {
            if !module.allow_override && self.definitions.contains_key(&def.key) {
                return Err(DiError::Duplicate {
                    type_name: def.type_name,
                    qualifier: def.key.qualifier,
                });
            }
        }
        for def in module.definitions {
            self.singles.lock().expect("di lock").remove(&def.key);
            self.definitions.insert(def.key, def);
        }
        for p in module.params {
            self.params.insert(p.key, p);
        }
        self.modules.push(module.name);
        Ok(())
    }

    /// Names of the loaded modules, in load order.
    pub fn modules(&self) -> &[&'static str] {
        &self.modules
    }

    /// Number of definitions.
    pub fn len(&self) -> usize {
        self.definitions.len()
    }

    /// True when nothing is defined.
    pub fn is_empty(&self) -> bool {
        self.definitions.is_empty()
    }

    /// True when `T` (with `qualifier`) is defined.
    pub fn contains<T: ?Sized + 'static>(&self, qualifier: Option<&'static str>) -> bool {
        self.definitions.contains_key(&DefKey::of::<T>(qualifier))
    }

    /// A resolver for the global (unscoped) context.
    pub fn resolver(&self) -> Resolver<'_> {
        Resolver {
            di: self,
            scope: None,
            stack: RefCell::new(Vec::new()),
        }
    }

    /// A resolver bound to a scope.
    pub fn scope(&self, scope: ScopeId) -> Resolver<'_> {
        Resolver {
            di: self,
            scope: Some(scope),
            stack: RefCell::new(Vec::new()),
        }
    }

    /// Resolves `T`, panicking with a descriptive message when it cannot be resolved.
    pub fn get<T: ?Sized + Send + Sync + 'static>(&self) -> Arc<T> {
        self.resolver().get()
    }

    /// Resolves `T`.
    pub fn try_get<T: ?Sized + Send + Sync + 'static>(&self) -> Result<Arc<T>, DiError> {
        self.resolver().try_get()
    }

    /// Resolves a named `T`.
    pub fn get_named<T: ?Sized + Send + Sync + 'static>(&self, qualifier: &'static str) -> Arc<T> {
        self.resolver().get_named(qualifier)
    }

    /// Resolves a parameterized factory.
    pub fn get_with<T: Send + Sync + 'static, P: 'static>(&self, params: P) -> Arc<T> {
        self.resolver().get_with(params)
    }

    /// Drops every instance cached for a scope.
    pub fn close_scope(&self, scope: ScopeId) -> bool {
        self.scopes
            .lock()
            .expect("di lock")
            .remove(&scope)
            .is_some()
    }

    /// Number of open scopes (scopes with at least one cached instance).
    pub fn open_scopes(&self) -> usize {
        self.scopes.lock().expect("di lock").len()
    }

    /// Instantiates all eager singletons.
    pub fn create_eager(&self) -> Result<usize, DiError> {
        let eager: Vec<DefKey> = self
            .definitions
            .values()
            .filter(|d| d.eager)
            .map(|d| d.key)
            .collect();
        let r = self.resolver();
        for key in &eager {
            r.resolve_key(*key, None)?;
        }
        Ok(eager.len())
    }

    /// Verifies the whole graph: every definition is resolved once in a throw away container
    /// (scoped ones inside a temporary scope). Returns every error found. Ideal for a unit test
    /// guarding your modules.
    pub fn verify(&self) -> Vec<DiError> {
        let probe = WingsDi {
            definitions: self.definitions.clone(),
            params: self.params.clone(),
            ..Default::default()
        };
        let mut errors = Vec::new();
        let scope = ScopeId::Id(u64::MAX);
        for def in self.definitions.values() {
            let r = probe.scope(scope);
            if let Err(e) = r.resolve_key(def.key, Some(def.type_name))
                && !errors.contains(&e)
            {
                errors.push(e);
            }
        }
        errors
    }
}

/// Resolves dependencies. Constructors receive a resolver to fetch their own dependencies.
pub struct Resolver<'a> {
    di: &'a WingsDi,
    scope: Option<ScopeId>,
    stack: RefCell<Vec<(DefKey, &'static str)>>,
}

impl Resolver<'_> {
    /// The scope this resolver is bound to.
    pub fn scope_id(&self) -> Option<ScopeId> {
        self.scope
    }

    /// Resolves `T`, panicking with a descriptive message on failure.
    pub fn get<T: ?Sized + Send + Sync + 'static>(&self) -> Arc<T> {
        self.try_get().unwrap_or_else(|e| fail(e))
    }

    /// Resolves `T`.
    pub fn try_get<T: ?Sized + Send + Sync + 'static>(&self) -> Result<Arc<T>, DiError> {
        self.typed(DefKey::of::<T>(None))
    }

    /// Resolves a named `T`, panicking on failure.
    pub fn get_named<T: ?Sized + Send + Sync + 'static>(&self, qualifier: &'static str) -> Arc<T> {
        self.try_get_named(qualifier).unwrap_or_else(|e| fail(e))
    }

    /// Resolves a named `T`.
    pub fn try_get_named<T: ?Sized + Send + Sync + 'static>(
        &self,
        qualifier: &'static str,
    ) -> Result<Arc<T>, DiError> {
        self.typed(DefKey::of::<T>(Some(qualifier)))
    }

    /// Resolves `T` if it is defined, `None` otherwise.
    pub fn get_optional<T: ?Sized + Send + Sync + 'static>(&self) -> Option<Arc<T>> {
        self.try_get().ok()
    }

    /// Resolves a parameterized factory, panicking on failure.
    pub fn get_with<T: Send + Sync + 'static, P: 'static>(&self, params: P) -> Arc<T> {
        self.try_get_with(params).unwrap_or_else(|e| fail(e))
    }

    /// Resolves a parameterized factory.
    pub fn try_get_with<T: Send + Sync + 'static, P: 'static>(
        &self,
        params: P,
    ) -> Result<Arc<T>, DiError> {
        let key = (TypeId::of::<T>(), TypeId::of::<P>());
        let def = self.di.params.get(&key).ok_or_else(|| DiError::Missing {
            type_name: core::any::type_name::<T>(),
            qualifier: None,
            chain: self.chain(),
        })?;
        let erased = (def.ctor)(self, Box::new(params) as Box<dyn Any>);
        Ok(downcast::<T>(&erased))
    }

    fn typed<T: ?Sized + Send + Sync + 'static>(&self, key: DefKey) -> Result<Arc<T>, DiError> {
        let erased = self.resolve_key(key, Some(core::any::type_name::<T>()))?;
        Ok(downcast::<T>(&erased))
    }

    fn chain(&self) -> Vec<&'static str> {
        self.stack.borrow().iter().map(|(_, n)| *n).collect()
    }

    pub(crate) fn resolve_key(
        &self,
        key: DefKey,
        type_name: Option<&'static str>,
    ) -> Result<Erased, DiError> {
        let Some(def) = self.di.definitions.get(&key) else {
            return Err(DiError::Missing {
                type_name: type_name.unwrap_or("<unknown>"),
                qualifier: key.qualifier,
                chain: self.chain(),
            });
        };
        if self.stack.borrow().iter().any(|(k, _)| *k == key) {
            let mut chain = self.chain();
            let start = self
                .stack
                .borrow()
                .iter()
                .position(|(k, _)| *k == key)
                .unwrap_or(0);
            chain.drain(..start);
            chain.push(def.type_name);
            return Err(DiError::Cycle { chain });
        }
        // Fast path: cached instances. Locks are never held while constructing so that
        // constructors can resolve their own dependencies.
        match def.lifetime {
            Lifetime::Single => {
                if let Some(v) = self.di.singles.lock().expect("di lock").get(&key) {
                    return Ok(v.clone());
                }
            }
            Lifetime::Scoped => {
                let Some(scope) = self.scope else {
                    return Err(DiError::NoScope {
                        type_name: def.type_name,
                    });
                };
                if let Some(v) = self
                    .di
                    .scopes
                    .lock()
                    .expect("di lock")
                    .get(&scope)
                    .and_then(|s| s.get(&key))
                {
                    return Ok(v.clone());
                }
            }
            Lifetime::Factory => {}
        }

        self.stack.borrow_mut().push((key, def.type_name));
        LAST_ERROR.with(|l| l.borrow_mut().take());
        let built = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| (def.ctor)(self)));
        self.stack.borrow_mut().pop();
        let value = match built {
            Ok(v) => v,
            Err(payload) => {
                // A nested `get` failed. Recover the structured error when there is one so
                // that `try_get` and `verify` report it instead of panicking.
                if let Some(e) = LAST_ERROR.with(|l| l.borrow_mut().take()) {
                    return Err(e);
                }
                std::panic::resume_unwind(payload);
            }
        };

        match def.lifetime {
            Lifetime::Single => Ok(self
                .di
                .singles
                .lock()
                .expect("di lock")
                .entry(key)
                .or_insert(value)
                .clone()),
            Lifetime::Scoped => {
                let scope = self.scope.expect("checked above");
                Ok(self
                    .di
                    .scopes
                    .lock()
                    .expect("di lock")
                    .entry(scope)
                    .or_default()
                    .entry(key)
                    .or_insert(value)
                    .clone())
            }
            Lifetime::Factory => Ok(value),
        }
    }
}

fn downcast<T: ?Sized + Send + Sync + 'static>(erased: &Erased) -> Arc<T> {
    erased
        .downcast_ref::<Arc<T>>()
        .expect("definition key matches its stored type")
        .clone()
}

std::thread_local! {
    static LAST_ERROR: RefCell<Option<DiError>> = const { RefCell::new(None) };
}

/// Panics with a readable message, remembering the structured error so that an enclosing
/// `try_get` can turn the panic back into a [`DiError`].
fn fail(e: DiError) -> ! {
    LAST_ERROR.with(|l| *l.borrow_mut() = Some(e.clone()));
    panic!("{e}")
}
