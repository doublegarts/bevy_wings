//! Key value data sources.

use std::collections::BTreeMap;
use std::sync::RwLock;

/// A key value store. Implement it for files, databases or platform storage; the repositories
/// only depend on this trait.
pub trait DataSource<K, V>: Send + Sync {
    /// Reads a value.
    fn get(&self, key: &K) -> Option<V>;
    /// Writes a value, returning the previous one.
    fn put(&self, key: K, value: V) -> Option<V>;
    /// Removes a value, returning it.
    fn remove(&self, key: &K) -> Option<V>;
    /// All values ordered by key.
    fn values(&self) -> Vec<V>;
    /// Number of entries.
    fn len(&self) -> usize;
    /// True when empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// Removes everything.
    fn clear(&self);
}

/// Thread safe, ordered, in memory [`DataSource`].
pub struct MemorySource<K, V> {
    map: RwLock<BTreeMap<K, V>>,
}

impl<K, V> Default for MemorySource<K, V> {
    fn default() -> Self {
        Self {
            map: RwLock::new(BTreeMap::new()),
        }
    }
}

impl<K: Ord, V> MemorySource<K, V> {
    /// An empty source.
    pub fn new() -> Self {
        Self::default()
    }

    /// A source prefilled with entries.
    pub fn from_entries(entries: impl IntoIterator<Item = (K, V)>) -> Self {
        Self {
            map: RwLock::new(entries.into_iter().collect()),
        }
    }
}

impl<K, V> DataSource<K, V> for MemorySource<K, V>
where
    K: Ord + Clone + Send + Sync,
    V: Clone + Send + Sync,
{
    fn get(&self, key: &K) -> Option<V> {
        self.map.read().expect("source lock").get(key).cloned()
    }

    fn put(&self, key: K, value: V) -> Option<V> {
        self.map.write().expect("source lock").insert(key, value)
    }

    fn remove(&self, key: &K) -> Option<V> {
        self.map.write().expect("source lock").remove(key)
    }

    fn values(&self) -> Vec<V> {
        self.map
            .read()
            .expect("source lock")
            .values()
            .cloned()
            .collect()
    }

    fn len(&self) -> usize {
        self.map.read().expect("source lock").len()
    }

    fn clear(&self) {
        self.map.write().expect("source lock").clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Basic map semantics, ordered values.
    #[test]
    fn memory_source() {
        let s = MemorySource::from_entries([(2, "b"), (1, "a")]);
        assert_eq!(s.values(), vec!["a", "b"]);
        assert_eq!(s.put(3, "c"), None);
        assert_eq!(s.put(3, "C"), Some("c"));
        assert_eq!(s.get(&3), Some("C"));
        assert_eq!(s.remove(&1), Some("a"));
        assert_eq!(s.len(), 2);
        s.clear();
        assert!(s.is_empty());
    }
}
