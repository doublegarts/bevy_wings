//! [`DatabaseSource`]: a [`DataSource`] in an embedded `redb` database (feature `database`).
//!
//! redb is a pure Rust, ACID, crash safe key value store in a single file: no server, no C
//! library, works on desktop and mobile. Each source is one table; several sources can share
//! a database file.

use std::marker::PhantomData;
use std::path::Path;
use std::sync::Arc;

use redb::{Database, ReadableDatabase, ReadableTable, ReadableTableMetadata, TableDefinition};
use serde::Serialize;
use serde::de::DeserializeOwned;

use bevy_wings_domain::{DomainError, DomainResult};

use crate::codec::{Codec, JsonCodec};
use crate::source::DataSource;

fn db_error(e: impl std::fmt::Display) -> DomainError {
    DomainError::Unavailable(format!("database: {e}"))
}

/// An open database file, shared by the sources that live in it.
#[derive(Clone)]
pub struct WingsDatabase {
    db: Arc<Database>,
}

impl WingsDatabase {
    /// Opens or creates the database file.
    pub fn open(path: impl AsRef<Path>) -> DomainResult<Self> {
        if let Some(dir) = path.as_ref().parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        Ok(Self {
            db: Arc::new(Database::create(path).map_err(db_error)?),
        })
    }

    /// A source stored in the table `name`.
    pub fn source<K, V>(&self, name: &str) -> DatabaseSource<K, V> {
        DatabaseSource {
            db: self.db.clone(),
            table: name.to_string(),
            codec: JsonCodec::default(),
            _marker: PhantomData,
        }
    }
}

/// A key value source in one table of a [`WingsDatabase`]. Keys and values are stored as
/// JSON.
pub struct DatabaseSource<K, V> {
    db: Arc<Database>,
    table: String,
    codec: JsonCodec,
    _marker: PhantomData<fn() -> (K, V)>,
}

impl<K, V> DatabaseSource<K, V>
where
    K: Serialize + DeserializeOwned,
    V: Serialize + DeserializeOwned,
{
    fn def(&self) -> TableDefinition<'_, &'static [u8], &'static [u8]> {
        TableDefinition::new(&self.table)
    }

    fn key(&self, key: &K) -> DomainResult<Vec<u8>> {
        Codec::<K>::encode(&self.codec, key)
    }

    /// Fallible get.
    pub fn try_get(&self, key: &K) -> DomainResult<Option<V>> {
        let k = self.key(key)?;
        let read = self.db.begin_read().map_err(db_error)?;
        let table = match read.open_table(self.def()) {
            Ok(t) => t,
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(None),
            Err(e) => return Err(db_error(e)),
        };
        match table.get(k.as_slice()).map_err(db_error)? {
            Some(v) => Codec::<V>::decode(&self.codec, v.value()).map(Some),
            None => Ok(None),
        }
    }

    /// Fallible put.
    pub fn try_put(&self, key: K, value: V) -> DomainResult<Option<V>> {
        let k = self.key(&key)?;
        let v = Codec::<V>::encode(&self.codec, &value)?;
        let write = self.db.begin_write().map_err(db_error)?;
        let old = {
            let mut table = write.open_table(self.def()).map_err(db_error)?;
            let old = table.insert(k.as_slice(), v.as_slice()).map_err(db_error)?;
            old.map(|o| Codec::<V>::decode(&self.codec, o.value()))
                .transpose()?
        };
        write.commit().map_err(db_error)?;
        Ok(old)
    }

    /// Fallible remove.
    pub fn try_remove(&self, key: &K) -> DomainResult<Option<V>> {
        let k = self.key(key)?;
        let write = self.db.begin_write().map_err(db_error)?;
        let old = {
            let mut table = write.open_table(self.def()).map_err(db_error)?;
            let old = table.remove(k.as_slice()).map_err(db_error)?;
            old.map(|o| Codec::<V>::decode(&self.codec, o.value()))
                .transpose()?
        };
        write.commit().map_err(db_error)?;
        Ok(old)
    }

    /// Fallible listing of every value, in key byte order.
    pub fn try_values(&self) -> DomainResult<Vec<V>> {
        let read = self.db.begin_read().map_err(db_error)?;
        let table = match read.open_table(self.def()) {
            Ok(t) => t,
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(Vec::new()),
            Err(e) => return Err(db_error(e)),
        };
        let mut out = Vec::new();
        for entry in table.iter().map_err(db_error)? {
            let (_, v) = entry.map_err(db_error)?;
            out.push(Codec::<V>::decode(&self.codec, v.value())?);
        }
        Ok(out)
    }

    fn try_len(&self) -> DomainResult<usize> {
        let read = self.db.begin_read().map_err(db_error)?;
        match read.open_table(self.def()) {
            Ok(t) => t.len().map(|n| n as usize).map_err(db_error),
            Err(redb::TableError::TableDoesNotExist(_)) => Ok(0),
            Err(e) => Err(db_error(e)),
        }
    }

    fn try_clear(&self) -> DomainResult<()> {
        let write = self.db.begin_write().map_err(db_error)?;
        let _ = write.delete_table(self.def()).map_err(db_error)?;
        write.commit().map_err(db_error)
    }
}

impl<K, V> DataSource<K, V> for DatabaseSource<K, V>
where
    K: Serialize + DeserializeOwned + Send + Sync,
    V: Serialize + DeserializeOwned + Send + Sync,
{
    fn get(&self, key: &K) -> Option<V> {
        self.try_get(key).ok().flatten()
    }

    fn put(&self, key: K, value: V) -> Option<V> {
        self.try_put(key, value).ok().flatten()
    }

    fn remove(&self, key: &K) -> Option<V> {
        self.try_remove(key).ok().flatten()
    }

    fn values(&self) -> Vec<V> {
        self.try_values().unwrap_or_default()
    }

    fn len(&self) -> usize {
        self.try_len().unwrap_or(0)
    }

    fn clear(&self) {
        let _ = self.try_clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Values persist across database reopens; tables are independent.
    #[test]
    fn database_round_trip() {
        let path = std::env::temp_dir().join(format!("wings-db-{}.redb", std::process::id()));
        let _ = std::fs::remove_file(&path);
        {
            let db = WingsDatabase::open(&path).unwrap();
            let notes = db.source::<u64, String>("notes");
            let tags = db.source::<String, u32>("tags");
            assert_eq!(notes.len(), 0);
            notes.put(1, "milk".into());
            notes.put(2, "ship".into());
            assert_eq!(notes.put(2, "ship 0.2".into()).as_deref(), Some("ship"));
            tags.put("rust".into(), 3);
            assert_eq!(notes.remove(&1).as_deref(), Some("milk"));
        }
        let db = WingsDatabase::open(&path).unwrap();
        let notes = db.source::<u64, String>("notes");
        assert_eq!(notes.values(), vec!["ship 0.2".to_string()]);
        assert_eq!(
            db.source::<String, u32>("tags").get(&"rust".into()),
            Some(3)
        );
        notes.clear();
        assert!(notes.is_empty());
        let _ = std::fs::remove_file(&path);
    }
}
