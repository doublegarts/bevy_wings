//! Caching policies and time based expiry.

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Where a repository reads from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CachePolicy {
    /// Fresh local data wins; otherwise fetch remotely, falling back to stale local data when
    /// the remote is unavailable.
    #[default]
    CacheFirst,
    /// Always try the remote first, falling back to local data when it is unavailable.
    RemoteFirst,
    /// Never touch the remote for reads.
    CacheOnly,
    /// Always read remotely and do not cache.
    RemoteOnly,
}

/// A source of "now". Inject [`ManualClock`] in tests to control expiry.
pub trait Clock: Send + Sync {
    /// The current instant.
    fn now(&self) -> Instant;
}

/// The real monotonic clock.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

/// A clock that only moves when told to.
#[derive(Debug)]
pub struct ManualClock {
    now: Mutex<Instant>,
}

impl Default for ManualClock {
    fn default() -> Self {
        Self {
            now: Mutex::new(Instant::now()),
        }
    }
}

impl ManualClock {
    /// A clock frozen at the current instant.
    pub fn new() -> Self {
        Self::default()
    }

    /// Moves time forward.
    pub fn advance(&self, by: Duration) {
        *self.now.lock().expect("clock lock") += by;
    }
}

impl Clock for ManualClock {
    fn now(&self) -> Instant {
        *self.now.lock().expect("clock lock")
    }
}

/// A map whose entries expire after a time to live.
pub struct TtlCache<K, V> {
    ttl: Duration,
    entries: Mutex<HashMap<K, (V, Instant)>>,
}

impl<K: Eq + Hash + Clone, V: Clone> TtlCache<K, V> {
    /// A cache whose entries live for `ttl`.
    pub fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            entries: Mutex::new(HashMap::new()),
        }
    }

    /// Stores a value stamped with `now`.
    pub fn insert(&self, key: K, value: V, now: Instant) {
        self.entries
            .lock()
            .expect("cache lock")
            .insert(key, (value, now));
    }

    /// A value that has not expired at `now`.
    pub fn get_fresh(&self, key: &K, now: Instant) -> Option<V> {
        let entries = self.entries.lock().expect("cache lock");
        entries
            .get(key)
            .filter(|(_, at)| now.duration_since(*at) < self.ttl)
            .map(|(v, _)| v.clone())
    }

    /// A value regardless of age.
    pub fn get_any(&self, key: &K) -> Option<V> {
        self.entries
            .lock()
            .expect("cache lock")
            .get(key)
            .map(|(v, _)| v.clone())
    }

    /// True when `key` holds a value younger than the time to live.
    pub fn is_fresh(&self, key: &K, now: Instant) -> bool {
        self.get_fresh(key, now).is_some()
    }

    /// Forgets one entry.
    pub fn invalidate(&self, key: &K) {
        self.entries.lock().expect("cache lock").remove(key);
    }

    /// Forgets everything.
    pub fn clear(&self) {
        self.entries.lock().expect("cache lock").clear();
    }

    /// Drops expired entries and returns how many were removed.
    pub fn purge(&self, now: Instant) -> usize {
        let mut entries = self.entries.lock().expect("cache lock");
        let before = entries.len();
        entries.retain(|_, (_, at)| now.duration_since(*at) < self.ttl);
        before - entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Entries expire after the time to live but remain readable as stale values.
    #[test]
    fn expiry() {
        let clock = ManualClock::new();
        let cache = TtlCache::new(Duration::from_secs(10));
        cache.insert("k", 1, clock.now());
        assert_eq!(cache.get_fresh(&"k", clock.now()), Some(1));
        clock.advance(Duration::from_secs(11));
        assert_eq!(cache.get_fresh(&"k", clock.now()), None);
        assert_eq!(cache.get_any(&"k"), Some(1));
        assert_eq!(cache.purge(clock.now()), 1);
        assert_eq!(cache.get_any(&"k"), None);
    }
}
