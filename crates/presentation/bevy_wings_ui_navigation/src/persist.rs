//! Saving and restoring the back stack.
//!
//! [`WingsBackStackPersistencePlugin`] restores the back stack at startup and saves it
//! whenever it changes, so an app reopens where the user left it (and survives being killed
//! in the background on mobile). Keys must be serializable:
//!
//! ```ignore
//! #[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
//! enum Screen { Home, Note(u32) }
//!
//! app.add_plugins(WingsBackStackPersistencePlugin::<Screen>::new(
//!     WingsFileStore::in_dir(std::env::temp_dir().join("my-app")),
//! ));
//! ```

use std::path::PathBuf;
use std::sync::Arc;

use bevy::prelude::*;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::back_stack::{NavKey, WingsBackStack};
use bevy_wings_ui_core::WingsUiSystems;

/// Where navigation state is kept.
pub trait WingsStateStore: Send + Sync + 'static {
    /// Reads a stored value.
    fn load(&self, key: &str) -> Option<String>;
    /// Stores a value.
    fn save(&self, key: &str, value: &str);
}

/// Stores each key as a file in a directory.
#[derive(Debug, Clone)]
pub struct WingsFileStore {
    dir: PathBuf,
}

impl WingsFileStore {
    /// Files in `dir` (created on first save).
    pub fn in_dir(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    fn path(&self, key: &str) -> PathBuf {
        let safe: String = key
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        self.dir.join(format!("{safe}.toml"))
    }
}

impl WingsStateStore for WingsFileStore {
    fn load(&self, key: &str) -> Option<String> {
        std::fs::read_to_string(self.path(key)).ok()
    }

    fn save(&self, key: &str, value: &str) {
        if std::fs::create_dir_all(&self.dir).is_ok() {
            let _ = std::fs::write(self.path(key), value);
        }
    }
}

/// An in memory store (tests, previews).
#[derive(Debug, Default, Clone)]
pub struct WingsMemoryStore {
    values: Arc<std::sync::Mutex<std::collections::HashMap<String, String>>>,
}

impl WingsStateStore for WingsMemoryStore {
    fn load(&self, key: &str) -> Option<String> {
        self.values.lock().ok()?.get(key).cloned()
    }

    fn save(&self, key: &str, value: &str) {
        if let Ok(mut values) = self.values.lock() {
            values.insert(key.to_string(), value.to_string());
        }
    }
}

#[derive(Serialize, Deserialize)]
struct SavedStack<K> {
    entries: Vec<K>,
}

/// Serializes back stack entries.
pub fn encode_stack<K: Serialize + Clone>(entries: &[K]) -> Option<String> {
    toml::to_string(&SavedStack {
        entries: entries.to_vec(),
    })
    .ok()
}

/// Parses back stack entries saved by [`encode_stack`].
pub fn decode_stack<K: DeserializeOwned>(text: &str) -> Option<Vec<K>> {
    toml::from_str::<SavedStack<K>>(text)
        .ok()
        .map(|s| s.entries)
        .filter(|e| !e.is_empty())
}

/// The store the persistence plugin of `K` uses.
#[derive(Resource)]
pub struct WingsBackStackStore<K: NavKey> {
    store: Arc<dyn WingsStateStore>,
    key: String,
    _marker: core::marker::PhantomData<fn() -> K>,
}

/// Restores the back stack of `K` at startup and saves it on every change.
pub struct WingsBackStackPersistencePlugin<K: NavKey> {
    store: Arc<dyn WingsStateStore>,
    key: String,
    _marker: core::marker::PhantomData<fn() -> K>,
}

impl<K: NavKey> WingsBackStackPersistencePlugin<K> {
    /// Persistence in `store` under the key `back_stack`.
    pub fn new(store: impl WingsStateStore) -> Self {
        Self {
            store: Arc::new(store),
            key: "back_stack".into(),
            _marker: core::marker::PhantomData,
        }
    }

    /// Uses another storage key (for apps with several navigation hierarchies).
    pub fn with_key(mut self, key: impl Into<String>) -> Self {
        self.key = key.into();
        self
    }
}

impl<K: NavKey + Serialize + DeserializeOwned> Plugin for WingsBackStackPersistencePlugin<K> {
    fn build(&self, app: &mut App) {
        app.insert_resource(WingsBackStackStore::<K> {
            store: self.store.clone(),
            key: self.key.clone(),
            _marker: core::marker::PhantomData,
        })
        .add_systems(PreStartup, restore_back_stack::<K>)
        .add_systems(
            Update,
            save_back_stack::<K>.after(WingsUiSystems::Navigation),
        );
    }
}

/// Replaces the back stack with the saved one, when there is one.
pub fn restore_back_stack<K: NavKey + DeserializeOwned>(
    store: Res<WingsBackStackStore<K>>,
    stack: Option<ResMut<WingsBackStack<K>>>,
) {
    let (Some(text), Some(mut stack)) = (store.store.load(&store.key), stack) else {
        return;
    };
    if let Some(entries) = decode_stack::<K>(&text) {
        *stack = WingsBackStack::from_entries(entries);
    }
}

/// Saves the back stack when it changed.
pub fn save_back_stack<K: NavKey + Serialize>(
    store: Res<WingsBackStackStore<K>>,
    stack: Res<WingsBackStack<K>>,
) {
    if stack.is_changed()
        && !stack.is_added()
        && let Some(text) = encode_stack(stack.entries())
    {
        store.store.save(&store.key, &text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    enum Screen {
        Home,
        Note(u32),
    }

    /// Stacks round trip through text.
    #[test]
    fn round_trip() {
        let text = encode_stack(&[Screen::Home, Screen::Note(7)]).unwrap();
        assert_eq!(
            decode_stack::<Screen>(&text),
            Some(vec![Screen::Home, Screen::Note(7)])
        );
        assert_eq!(decode_stack::<Screen>("garbage"), None);
    }

    /// The plugin restores at startup and saves changes.
    #[test]
    fn restores_and_saves() {
        let store = WingsMemoryStore::default();
        store.save(
            "back_stack",
            &encode_stack(&[Screen::Home, Screen::Note(3)]).unwrap(),
        );
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(bevy_wings_ui_core::WingsCorePlugin)
            .insert_resource(WingsBackStack::new(Screen::Home))
            .add_plugins(WingsBackStackPersistencePlugin::<Screen>::new(
                store.clone(),
            ));
        app.update();
        assert_eq!(
            app.world().resource::<WingsBackStack<Screen>>().entries(),
            &[Screen::Home, Screen::Note(3)]
        );
        app.world_mut()
            .resource_mut::<WingsBackStack<Screen>>()
            .push(Screen::Note(9));
        app.update();
        assert_eq!(
            decode_stack::<Screen>(&store.load("back_stack").unwrap()),
            Some(vec![Screen::Home, Screen::Note(3), Screen::Note(9)])
        );
    }

    /// Files are written and read back.
    #[test]
    fn file_store() {
        let dir = std::env::temp_dir().join(format!("wings-nav-test-{}", std::process::id()));
        let store = WingsFileStore::in_dir(&dir);
        store.save("back/stack", "x = 1");
        assert_eq!(store.load("back/stack").as_deref(), Some("x = 1"));
        let _ = std::fs::remove_dir_all(dir);
    }
}
