//! Offline first repository.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use dga_bevy_wings_domain::{
    DomainEntity, DomainError, DomainResult, Id, ReadRepository, WriteRepository,
};

use crate::cache::{CachePolicy, Clock, SystemClock, TtlCache};
use crate::remote::RemoteSource;
use crate::source::{DataSource, MemorySource};

/// A pending write that did not reach the remote yet.
#[derive(Debug, Clone, PartialEq)]
enum PendingWrite<T: DomainEntity> {
    Save(T),
    Delete(Id<T>),
}

/// A domain repository that keeps a local copy of a remote source.
///
/// * Reads follow the [`CachePolicy`]. With the default `CacheFirst` policy fresh local data
///   is returned without a remote call; expired data is refreshed; when the remote is
///   unavailable, stale local data is served instead of an error.
/// * Writes go to the local copy immediately (the UI updates at once) and are pushed to the
///   remote. If the remote is unavailable the write is queued, and [`sync`](Self::sync) retries
///   queued writes in order.
pub struct OfflineFirstRepository<T: DomainEntity> {
    local: Box<dyn DataSource<Id<T>, T>>,
    remote: Arc<dyn RemoteSource<T>>,
    policy: CachePolicy,
    freshness: TtlCache<Option<Id<T>>, ()>,
    clock: Arc<dyn Clock>,
    pending: Mutex<Vec<PendingWrite<T>>>,
}

impl<T: DomainEntity> OfflineFirstRepository<T> {
    /// A cache first repository with an in memory local copy and a five minute freshness.
    pub fn new(remote: Arc<dyn RemoteSource<T>>) -> Self {
        Self {
            local: Box::new(MemorySource::new()),
            remote,
            policy: CachePolicy::default(),
            freshness: TtlCache::new(Duration::from_secs(300)),
            clock: Arc::new(SystemClock),
            pending: Mutex::new(Vec::new()),
        }
    }

    /// Uses another local store (file, database).
    pub fn with_local(mut self, local: impl DataSource<Id<T>, T> + 'static) -> Self {
        self.local = Box::new(local);
        self
    }

    /// Sets the read policy.
    pub fn with_policy(mut self, policy: CachePolicy) -> Self {
        self.policy = policy;
        self
    }

    /// Sets how long fetched data counts as fresh.
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.freshness = TtlCache::new(ttl);
        self
    }

    /// Injects a clock (use [`ManualClock`](crate::ManualClock) in tests).
    pub fn with_clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }

    /// The read policy.
    pub fn policy(&self) -> CachePolicy {
        self.policy
    }

    /// Number of writes waiting for the remote.
    pub fn pending_writes(&self) -> usize {
        self.pending.lock().expect("pending lock").len()
    }

    /// Marks everything stale so the next read goes to the remote.
    pub fn invalidate(&self) {
        self.freshness.clear();
    }

    /// Pushes queued writes in order. Stops at the first unavailable error and keeps the rest.
    /// Returns how many writes were delivered.
    pub fn sync(&self) -> DomainResult<usize> {
        let mut pending = self.pending.lock().expect("pending lock");
        let mut delivered = 0;
        while let Some(write) = pending.first().cloned() {
            let result = match write {
                PendingWrite::Save(e) => self.remote.push(e),
                PendingWrite::Delete(id) => self.remote.remove(id),
            };
            match result {
                Ok(()) => {
                    pending.remove(0);
                    delivered += 1;
                }
                Err(e) if e.is_retryable() => return Ok(delivered),
                Err(e) => return Err(e),
            }
        }
        Ok(delivered)
    }

    fn fresh(&self, key: Option<Id<T>>) -> bool {
        self.freshness.is_fresh(&key, self.clock.now())
    }

    fn touch(&self, key: Option<Id<T>>) {
        self.freshness.insert(key, (), self.clock.now());
    }

    fn fetch_one(&self, id: Id<T>) -> DomainResult<Option<T>> {
        let mut fetched = self.remote.fetch(id)?;
        // A reconnect must not overwrite an unsynchronized edit or resurrect a deletion.
        // As in fetch_every, replay the latest pending write over the remote snapshot.
        let pending = self.pending.lock().expect("pending lock");
        for write in pending.iter().rev() {
            match write {
                PendingWrite::Save(entity) if entity.id() == id => {
                    fetched = Some(entity.clone());
                    break;
                }
                PendingWrite::Delete(deleted) if *deleted == id => {
                    fetched = None;
                    break;
                }
                _ => {}
            }
        }
        match &fetched {
            Some(e) => {
                self.local.put(id, e.clone());
            }
            None => {
                self.local.remove(&id);
            }
        }
        self.touch(Some(id));
        Ok(fetched)
    }

    fn fetch_every(&self) -> DomainResult<Vec<T>> {
        let all = self.remote.fetch_all()?;
        self.local.clear();
        for e in &all {
            self.local.put(e.id(), e.clone());
        }
        // Queued local writes stay visible until they reach the remote.
        for write in self.pending.lock().expect("pending lock").iter() {
            match write {
                PendingWrite::Save(e) => {
                    self.local.put(e.id(), e.clone());
                }
                PendingWrite::Delete(id) => {
                    self.local.remove(id);
                }
            }
        }
        self.touch(None);
        Ok(self.local.values())
    }

    fn queue_or_fail(&self, write: PendingWrite<T>, error: DomainError) -> DomainResult<()> {
        if error.is_retryable() {
            self.pending.lock().expect("pending lock").push(write);
            Ok(())
        } else {
            Err(error)
        }
    }
}

impl<T: DomainEntity> ReadRepository<T> for OfflineFirstRepository<T> {
    fn get(&self, id: Id<T>) -> DomainResult<Option<T>> {
        let local = || Ok(self.local.get(&id));
        match self.policy {
            CachePolicy::CacheOnly => local(),
            CachePolicy::RemoteOnly => self.remote.fetch(id),
            CachePolicy::CacheFirst => {
                if self.fresh(Some(id)) || self.fresh(None) {
                    return local();
                }
                match self.fetch_one(id) {
                    Err(e) if e.is_retryable() => local(),
                    other => other,
                }
            }
            CachePolicy::RemoteFirst => match self.fetch_one(id) {
                Err(e) if e.is_retryable() => local(),
                other => other,
            },
        }
    }

    fn all(&self) -> DomainResult<Vec<T>> {
        let local = || Ok(self.local.values());
        match self.policy {
            CachePolicy::CacheOnly => local(),
            CachePolicy::RemoteOnly => self.remote.fetch_all(),
            CachePolicy::CacheFirst => {
                if self.fresh(None) {
                    return local();
                }
                match self.fetch_every() {
                    Err(e) if e.is_retryable() && !self.local.is_empty() => local(),
                    other => other,
                }
            }
            CachePolicy::RemoteFirst => match self.fetch_every() {
                Err(e) if e.is_retryable() && !self.local.is_empty() => local(),
                other => other,
            },
        }
    }
}

impl<T: DomainEntity> WriteRepository<T> for OfflineFirstRepository<T> {
    fn save(&self, entity: T) -> DomainResult<()> {
        self.local.put(entity.id(), entity.clone());
        match self.remote.push(entity.clone()) {
            Ok(()) => Ok(()),
            Err(e) => self.queue_or_fail(PendingWrite::Save(entity), e),
        }
    }

    fn delete(&self, id: Id<T>) -> DomainResult<bool> {
        let existed = self.local.remove(&id).is_some();
        match self.remote.remove(id) {
            Ok(()) => Ok(existed),
            Err(e) => self
                .queue_or_fail(PendingWrite::Delete(id), e)
                .map(|_| existed),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FakeRemote, ManualClock};

    #[derive(Clone, Debug, PartialEq)]
    struct Todo {
        id: Id<Todo>,
        title: &'static str,
    }
    impl DomainEntity for Todo {
        fn id(&self) -> Id<Todo> {
            self.id
        }
    }

    fn todo(id: u64, title: &'static str) -> Todo {
        Todo {
            id: Id::new(id),
            title,
        }
    }

    fn setup(
        policy: CachePolicy,
    ) -> (
        Arc<FakeRemote<Todo>>,
        Arc<ManualClock>,
        OfflineFirstRepository<Todo>,
    ) {
        let remote = Arc::new(FakeRemote::with_items([todo(1, "a"), todo(2, "b")]));
        let clock = Arc::new(ManualClock::new());
        let repo = OfflineFirstRepository::new(remote.clone())
            .with_policy(policy)
            .with_ttl(Duration::from_secs(60))
            .with_clock(clock.clone());
        (remote, clock, repo)
    }

    /// Cache first: one remote call, then local reads until the data expires.
    #[test]
    fn cache_first_reads_locally_while_fresh() {
        let (remote, clock, repo) = setup(CachePolicy::CacheFirst);
        assert_eq!(repo.all().unwrap().len(), 2);
        assert_eq!(repo.all().unwrap().len(), 2);
        assert_eq!(repo.get(Id::new(1)).unwrap(), Some(todo(1, "a")));
        assert_eq!(remote.calls(), 1);
        clock.advance(Duration::from_secs(61));
        repo.all().unwrap();
        assert_eq!(remote.calls(), 2);
    }

    /// When the remote is down, stale local data is served instead of an error.
    #[test]
    fn stale_data_when_offline() {
        let (remote, clock, repo) = setup(CachePolicy::CacheFirst);
        repo.all().unwrap();
        clock.advance(Duration::from_secs(120));
        remote.set_online(false);
        assert_eq!(repo.all().unwrap().len(), 2);
        assert_eq!(repo.get(Id::new(2)).unwrap(), Some(todo(2, "b")));
    }

    /// With nothing cached, an offline remote surfaces the error.
    #[test]
    fn offline_without_cache_fails() {
        let (remote, _, repo) = setup(CachePolicy::CacheFirst);
        remote.set_online(false);
        assert!(matches!(repo.all(), Err(DomainError::Unavailable(_))));
    }

    /// Offline writes apply locally, are queued, and reach the remote on sync.
    #[test]
    fn offline_writes_are_queued_and_synced() {
        let (remote, _, repo) = setup(CachePolicy::CacheFirst);
        repo.all().unwrap();
        remote.set_online(false);
        repo.save(todo(3, "c")).unwrap();
        assert!(repo.delete(Id::new(1)).unwrap());
        assert_eq!(repo.pending_writes(), 2);
        let titles: Vec<_> = repo.all().unwrap().iter().map(|t| t.title).collect();
        assert_eq!(titles, vec!["b", "c"]);
        assert_eq!(repo.sync().unwrap(), 0);
        remote.set_online(true);
        assert_eq!(repo.sync().unwrap(), 2);
        assert_eq!(repo.pending_writes(), 0);
        let remote_titles: Vec<_> = remote.items().iter().map(|t| t.title).collect();
        assert_eq!(remote_titles, vec!["b", "c"]);
    }

    /// Remote first always asks the remote; cache only never does.
    #[test]
    fn other_policies() {
        let (remote, _, repo) = setup(CachePolicy::RemoteFirst);
        repo.all().unwrap();
        repo.all().unwrap();
        assert_eq!(remote.calls(), 2);

        let (remote, _, repo) = setup(CachePolicy::CacheOnly);
        assert!(repo.all().unwrap().is_empty());
        assert_eq!(remote.calls(), 0);

        let (remote, _, repo) = setup(CachePolicy::RemoteOnly);
        assert_eq!(repo.all().unwrap().len(), 2);
        assert_eq!(repo.all().unwrap().len(), 2);
        assert_eq!(remote.calls(), 2);
    }

    /// `require` from the domain trait works through the repository.
    #[test]
    fn require_reports_not_found() {
        let (_, _, repo) = setup(CachePolicy::RemoteFirst);
        assert!(matches!(
            repo.require(Id::new(42)),
            Err(DomainError::NotFound { .. })
        ));
    }

    /// A remote-first read after reconnecting preserves an edit awaiting synchronization.
    #[test]
    fn reconnect_preserves_pending_edit() {
        let (remote, _, repo) = setup(CachePolicy::RemoteFirst);
        remote.set_online(false);
        repo.save(todo(1, "offline edit")).unwrap();
        remote.set_online(true);
        assert_eq!(repo.get(Id::new(1)).unwrap(), Some(todo(1, "offline edit")));
        assert_eq!(repo.pending_writes(), 1);
        repo.sync().unwrap();
        assert_eq!(repo.get(Id::new(1)).unwrap(), Some(todo(1, "offline edit")));
    }

    /// Fetching a deleted item must not restore it before the queued deletion is synced.
    #[test]
    fn reconnect_preserves_pending_deletion() {
        let (remote, _, repo) = setup(CachePolicy::RemoteFirst);
        repo.all().unwrap();
        remote.set_online(false);
        repo.save(todo(1, "offline edit")).unwrap();
        repo.delete(Id::new(1)).unwrap();
        remote.set_online(true);
        assert_eq!(repo.get(Id::new(1)).unwrap(), None);
        assert_eq!(repo.pending_writes(), 2);
        repo.sync().unwrap();
        assert_eq!(repo.get(Id::new(1)).unwrap(), None);
    }
}
