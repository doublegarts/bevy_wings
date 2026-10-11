//! Repository contracts. The data layer implements them; use cases depend only on these traits.

use crate::entity::{DomainEntity, Id};
use crate::error::DomainResult;
use crate::page::{Page, PageRequest};

/// Read access to a collection of entities.
pub trait ReadRepository<T: DomainEntity>: Send + Sync {
    /// Looks up one entity.
    fn get(&self, id: Id<T>) -> DomainResult<Option<T>>;

    /// All entities, ordered by id.
    fn all(&self) -> DomainResult<Vec<T>>;

    /// One page of entities, ordered by id.
    fn page(&self, request: PageRequest) -> DomainResult<Page<T>> {
        Ok(Page::from_slice(&self.all()?, request))
    }

    /// Looks up one entity and turns absence into [`DomainError::NotFound`](crate::DomainError::NotFound).
    fn require(&self, id: Id<T>) -> DomainResult<T> {
        self.get(id)?.ok_or_else(|| {
            crate::DomainError::not_found(format!(
                "{} {id}",
                short_type_name(core::any::type_name::<T>())
            ))
        })
    }
}

/// Write access to a collection of entities.
pub trait WriteRepository<T: DomainEntity>: ReadRepository<T> {
    /// Inserts or replaces an entity.
    fn save(&self, entity: T) -> DomainResult<()>;

    /// Removes an entity. Returns whether it existed.
    fn delete(&self, id: Id<T>) -> DomainResult<bool>;
}

fn short_type_name(full: &str) -> &str {
    full.rsplit("::").next().unwrap_or(full)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DomainError;

    #[derive(Clone, Debug, PartialEq)]
    struct Note {
        id: Id<Note>,
    }
    impl DomainEntity for Note {
        fn id(&self) -> Id<Note> {
            self.id
        }
    }

    struct Fixed(Vec<Note>);
    impl ReadRepository<Note> for Fixed {
        fn get(&self, id: Id<Note>) -> DomainResult<Option<Note>> {
            Ok(self.0.iter().find(|n| n.id == id).cloned())
        }
        fn all(&self) -> DomainResult<Vec<Note>> {
            Ok(self.0.clone())
        }
    }

    /// Default methods: paging and `require`.
    #[test]
    fn default_methods() {
        let repo = Fixed((1..=5).map(|i| Note { id: Id::new(i) }).collect());
        let page = repo.page(PageRequest::new(1, 2)).unwrap();
        assert_eq!(page.items[0].id, Id::new(3));
        assert!(repo.require(Id::new(2)).is_ok());
        assert_eq!(
            repo.require(Id::new(9)),
            Err(DomainError::not_found("Note 9"))
        );
    }
}
