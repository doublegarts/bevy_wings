//! Entities and typed ids.

use core::fmt;
use core::hash::{Hash, Hasher};
use core::marker::PhantomData;

/// A typed identifier. `Id<User>` and `Id<Order>` are different types, so they can not be
/// mixed up, yet both are a plain `u64` at runtime.
pub struct Id<T: ?Sized> {
    value: u64,
    _marker: PhantomData<fn() -> T>,
}

impl<T: ?Sized> Id<T> {
    /// Wraps a raw value.
    pub const fn new(value: u64) -> Self {
        Self {
            value,
            _marker: PhantomData,
        }
    }

    /// The raw value.
    pub const fn value(self) -> u64 {
        self.value
    }
}

impl<T: ?Sized> Clone for Id<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: ?Sized> Copy for Id<T> {}
impl<T: ?Sized> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}
impl<T: ?Sized> Eq for Id<T> {}
impl<T: ?Sized> PartialOrd for Id<T> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl<T: ?Sized> Ord for Id<T> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.value.cmp(&other.value)
    }
}
impl<T: ?Sized> Hash for Id<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}
impl<T: ?Sized> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Id({})", self.value)
    }
}
impl<T: ?Sized> fmt::Display for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}
impl<T: ?Sized> From<u64> for Id<T> {
    fn from(value: u64) -> Self {
        Self::new(value)
    }
}

/// An object with an identity that persists while its attributes change.
pub trait DomainEntity: Clone + Send + Sync + 'static {
    /// The identity.
    fn id(&self) -> Id<Self>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    struct User;

    /// Ids compare, hash and print by value.
    #[test]
    fn ids_behave_like_values() {
        let a: Id<User> = Id::new(7);
        let b: Id<User> = 7.into();
        assert_eq!(a, b);
        assert_eq!(a.to_string(), "7");
        assert_eq!(format!("{a:?}"), "Id(7)");
        let set: HashSet<_> = [a, b, Id::new(8)].into_iter().collect();
        assert_eq!(set.len(), 2);
        assert!(Id::<User>::new(1) < Id::new(2));
    }
}
