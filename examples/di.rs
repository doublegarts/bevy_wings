//! Singleton sharing, factories, scoped services and dependency resolution.
use bevy_wings::prelude::*;
use std::sync::Arc;

struct Config {
    greeting: &'static str,
}
struct Greeting {
    config: Arc<Config>,
}
struct ScreenState;

fn main() -> Result<(), DiError> {
    let di = WingsDi::from_modules([WingsModule::new("example")
        .single(|_| Config {
            greeting: "Hello from DI",
        })
        .factory(|r| Greeting { config: r.get() })
        .scoped(|_| ScreenState)])?;
    assert!(di.verify().is_empty());
    let first = di.get::<Greeting>();
    let second = di.get::<Greeting>();
    assert!(!Arc::ptr_eq(&first, &second));
    assert!(Arc::ptr_eq(&first.config, &second.config));
    let scope = ScopeId::Id(7);
    assert!(Arc::ptr_eq(
        &di.scope(scope).get::<ScreenState>(),
        &di.scope(scope).get::<ScreenState>()
    ));
    di.close_scope(scope);
    println!(
        "{}; factories differ, singleton is shared, screen scope closed",
        first.config.greeting
    );
    Ok(())
}
