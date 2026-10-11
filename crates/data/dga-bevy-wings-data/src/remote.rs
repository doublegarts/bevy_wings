//! Remote sources.

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use dga_bevy_wings_domain::{DomainEntity, DomainError, DomainResult, Id};

/// The authoritative store of entities: an HTTP API, a database, a cloud save. Calls may be
/// slow and may fail with [`DomainError::Unavailable`]; repositories handle both.
pub trait RemoteSource<T: DomainEntity>: Send + Sync {
    /// Fetches one entity.
    fn fetch(&self, id: Id<T>) -> DomainResult<Option<T>>;
    /// Fetches every entity.
    fn fetch_all(&self) -> DomainResult<Vec<T>>;
    /// Creates or replaces an entity.
    fn push(&self, entity: T) -> DomainResult<()>;
    /// Deletes an entity.
    fn remove(&self, id: Id<T>) -> DomainResult<()>;
}

/// An in memory [`RemoteSource`] for tests, previews and offline demos. It can be switched
/// offline and counts calls, so tests can assert on caching behavior.
pub struct FakeRemote<T: DomainEntity> {
    items: Mutex<BTreeMap<Id<T>, T>>,
    online: AtomicBool,
    calls: AtomicUsize,
}

impl<T: DomainEntity> Default for FakeRemote<T> {
    fn default() -> Self {
        Self {
            items: Mutex::new(BTreeMap::new()),
            online: AtomicBool::new(true),
            calls: AtomicUsize::new(0),
        }
    }
}

impl<T: DomainEntity> FakeRemote<T> {
    /// An online remote holding `items`.
    pub fn with_items(items: impl IntoIterator<Item = T>) -> Self {
        let r = Self::default();
        {
            let mut map = r.items.lock().expect("remote lock");
            for item in items {
                map.insert(item.id(), item);
            }
        }
        r
    }

    /// Simulates connectivity.
    pub fn set_online(&self, online: bool) {
        self.online.store(online, Ordering::SeqCst);
    }

    /// Number of calls made so far (successful or not).
    pub fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }

    /// Snapshot of the stored items.
    pub fn items(&self) -> Vec<T> {
        self.items
            .lock()
            .expect("remote lock")
            .values()
            .cloned()
            .collect()
    }

    fn enter(&self) -> DomainResult<()> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.online.load(Ordering::SeqCst) {
            Ok(())
        } else {
            Err(DomainError::Unavailable("remote offline".into()))
        }
    }
}

impl<T: DomainEntity> RemoteSource<T> for FakeRemote<T> {
    fn fetch(&self, id: Id<T>) -> DomainResult<Option<T>> {
        self.enter()?;
        Ok(self.items.lock().expect("remote lock").get(&id).cloned())
    }

    fn fetch_all(&self) -> DomainResult<Vec<T>> {
        self.enter()?;
        Ok(self.items())
    }

    fn push(&self, entity: T) -> DomainResult<()> {
        self.enter()?;
        self.items
            .lock()
            .expect("remote lock")
            .insert(entity.id(), entity);
        Ok(())
    }

    fn remove(&self, id: Id<T>) -> DomainResult<()> {
        self.enter()?;
        self.items.lock().expect("remote lock").remove(&id);
        Ok(())
    }
}
