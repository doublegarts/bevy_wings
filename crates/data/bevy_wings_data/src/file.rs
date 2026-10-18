//! [`FileSource`]: a [`DataSource`] kept in a JSON file.
//!
//! Every write replaces the file atomically (write to a temporary file, then rename), so a
//! crash never leaves a half written file. Good for settings, drafts, small offline caches
//! and save games; use the `database` feature's `DatabaseSource` for large data sets.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use serde::Serialize;
use serde::de::DeserializeOwned;

use bevy_wings_domain::{DomainError, DomainResult};

use crate::source::DataSource;

/// A key value source persisted as a JSON file.
pub struct FileSource<K, V> {
    path: PathBuf,
    map: RwLock<BTreeMap<K, V>>,
}

impl<K, V> FileSource<K, V>
where
    K: Ord + Clone + Serialize + DeserializeOwned,
    V: Clone + Serialize + DeserializeOwned,
{
    /// Opens (or starts) the file at `path`. A missing file is an empty source; a broken one
    /// is an error.
    pub fn open(path: impl AsRef<Path>) -> DomainResult<Self> {
        let path = path.as_ref().to_path_buf();
        let map = match std::fs::read(&path) {
            Ok(bytes) => {
                let entries: Vec<(K, V)> = serde_json::from_slice(&bytes)
                    .map_err(|e| DomainError::Unexpected(format!("{}: {e}", path.display())))?;
                entries.into_iter().collect()
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => BTreeMap::new(),
            Err(e) => return Err(DomainError::Unavailable(format!("{}: {e}", path.display()))),
        };
        Ok(Self {
            path,
            map: RwLock::new(map),
        })
    }

    /// The file this source writes.
    pub fn path(&self) -> &Path {
        &self.path
    }

    fn persist(&self, map: &BTreeMap<K, V>) {
        let entries: Vec<(&K, &V)> = map.iter().collect();
        let Ok(bytes) = serde_json::to_vec_pretty(&entries) else {
            return;
        };
        if let Some(dir) = self.path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let tmp = self.path.with_extension("tmp");
        if std::fs::write(&tmp, bytes).is_ok() {
            let _ = std::fs::rename(&tmp, &self.path);
        }
    }
}

impl<K, V> DataSource<K, V> for FileSource<K, V>
where
    K: Ord + Clone + Serialize + DeserializeOwned + Send + Sync,
    V: Clone + Serialize + DeserializeOwned + Send + Sync,
{
    fn get(&self, key: &K) -> Option<V> {
        self.map.read().expect("file source lock").get(key).cloned()
    }

    fn put(&self, key: K, value: V) -> Option<V> {
        let mut map = self.map.write().expect("file source lock");
        let old = map.insert(key, value);
        self.persist(&map);
        old
    }

    fn remove(&self, key: &K) -> Option<V> {
        let mut map = self.map.write().expect("file source lock");
        let old = map.remove(key);
        if old.is_some() {
            self.persist(&map);
        }
        old
    }

    fn values(&self) -> Vec<V> {
        self.map
            .read()
            .expect("file source lock")
            .values()
            .cloned()
            .collect()
    }

    fn len(&self) -> usize {
        self.map.read().expect("file source lock").len()
    }

    fn clear(&self) {
        let mut map = self.map.write().expect("file source lock");
        map.clear();
        self.persist(&map);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Values survive reopening the file.
    #[test]
    fn persists_across_opens() {
        let path =
            std::env::temp_dir().join(format!("wings-file-source-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&path);
        {
            let s = FileSource::<u64, String>::open(&path).unwrap();
            s.put(2, "two".into());
            s.put(1, "one".into());
            s.remove(&2);
        }
        let s = FileSource::<u64, String>::open(&path).unwrap();
        assert_eq!(s.values(), vec!["one".to_string()]);
        assert_eq!(s.get(&1).as_deref(), Some("one"));
        std::fs::write(&path, "broken").unwrap();
        assert!(FileSource::<u64, String>::open(&path).is_err());
        let _ = std::fs::remove_file(&path);
    }
}
