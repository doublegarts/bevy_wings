//! The back stack: plain, user owned navigation state.

use core::fmt::Debug;
use core::hash::Hash;

use bevy::prelude::*;

/// A navigation destination key. Any small, cloneable, comparable type works; an enum with
/// data carrying variants is the usual choice:
///
/// ```
/// #[derive(Clone, PartialEq, Eq, Hash, Debug)]
/// enum Screen { Home, Detail { id: u32 }, Settings }
/// ```
pub trait NavKey: Clone + PartialEq + Eq + Hash + Debug + Send + Sync + 'static {}
impl<T: Clone + PartialEq + Eq + Hash + Debug + Send + Sync + 'static> NavKey for T {}

/// The back stack for keys of type `K`, stored as a resource. The last entry is the visible
/// destination. Mutate it directly from any system; the navigation display reacts in the same
/// frame.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct WingsBackStack<K: NavKey> {
    entries: Vec<K>,
}

impl<K: NavKey> WingsBackStack<K> {
    /// A stack with a single start destination.
    pub fn new(start: K) -> Self {
        Self {
            entries: vec![start],
        }
    }

    /// A stack built from several entries (deep links), bottom first.
    pub fn from_entries(entries: impl IntoIterator<Item = K>) -> Self {
        Self {
            entries: entries.into_iter().collect(),
        }
    }

    /// Entries, bottom first.
    pub fn entries(&self) -> &[K] {
        &self.entries
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True when the stack is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The visible destination.
    pub fn top(&self) -> Option<&K> {
        self.entries.last()
    }

    /// True when a back navigation is possible (more than one entry).
    pub fn can_pop(&self) -> bool {
        self.entries.len() > 1
    }

    /// True when `key` is somewhere in the stack.
    pub fn contains(&self, key: &K) -> bool {
        self.entries.contains(key)
    }

    /// Pushes a destination.
    pub fn push(&mut self, key: K) {
        self.entries.push(key);
    }

    /// Pushes only when `key` is not already on top.
    pub fn push_single_top(&mut self, key: K) {
        if self.top() != Some(&key) {
            self.push(key);
        }
    }

    /// Moves an existing `key` to the top (or pushes it), keeping a single instance.
    pub fn bring_to_top(&mut self, key: K) {
        self.entries.retain(|k| *k != key);
        self.entries.push(key);
    }

    /// Pops the top entry, keeping at least one entry. Returns the popped key.
    pub fn pop(&mut self) -> Option<K> {
        if self.can_pop() {
            self.entries.pop()
        } else {
            None
        }
    }

    /// Replaces the top entry.
    pub fn replace_top(&mut self, key: K) {
        self.entries.pop();
        self.entries.push(key);
    }

    /// Pops until `key` is on top. With `inclusive`, `key` is popped too (the bottom entry is
    /// always kept). Returns `false` when `key` is not in the stack (nothing changes).
    pub fn pop_to(&mut self, key: &K, inclusive: bool) -> bool {
        let Some(pos) = self.entries.iter().rposition(|k| k == key) else {
            return false;
        };
        let keep = if inclusive { pos.max(1) } else { pos + 1 };
        self.entries.truncate(keep);
        true
    }

    /// Clears the stack and starts over at `key` (for example after logging out).
    pub fn reset(&mut self, key: K) {
        self.entries.clear();
        self.entries.push(key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, PartialEq, Eq, Hash, Debug)]
    enum S {
        Home,
        Detail(u32),
        Settings,
    }

    /// Push and pop keep the start destination.
    #[test]
    fn push_pop_keeps_root() {
        let mut s = WingsBackStack::new(S::Home);
        s.push(S::Detail(1));
        assert_eq!(s.top(), Some(&S::Detail(1)));
        assert_eq!(s.pop(), Some(S::Detail(1)));
        assert_eq!(s.pop(), None);
        assert_eq!(s.len(), 1);
    }

    /// Single top avoids duplicates on top.
    #[test]
    fn single_top() {
        let mut s = WingsBackStack::new(S::Home);
        s.push_single_top(S::Settings);
        s.push_single_top(S::Settings);
        assert_eq!(s.len(), 2);
    }

    /// Bring to top keeps one instance.
    #[test]
    fn bring_to_top() {
        let mut s = WingsBackStack::from_entries([S::Home, S::Settings, S::Detail(2)]);
        s.bring_to_top(S::Settings);
        assert_eq!(s.entries(), &[S::Home, S::Detail(2), S::Settings]);
    }

    /// Pop to, inclusive and exclusive.
    #[test]
    fn pop_to() {
        let mut s =
            WingsBackStack::from_entries([S::Home, S::Detail(1), S::Detail(2), S::Settings]);
        assert!(s.pop_to(&S::Detail(1), false));
        assert_eq!(s.entries(), &[S::Home, S::Detail(1)]);
        let mut s = WingsBackStack::from_entries([S::Home, S::Detail(1), S::Settings]);
        assert!(s.pop_to(&S::Detail(1), true));
        assert_eq!(s.entries(), &[S::Home]);
        assert!(!s.pop_to(&S::Settings, true));
        // The bottom entry is never popped.
        assert!(s.pop_to(&S::Home, true));
        assert_eq!(s.entries(), &[S::Home]);
    }

    /// Replace and reset.
    #[test]
    fn replace_and_reset() {
        let mut s = WingsBackStack::from_entries([S::Home, S::Detail(1)]);
        s.replace_top(S::Detail(2));
        assert_eq!(s.top(), Some(&S::Detail(2)));
        s.reset(S::Settings);
        assert_eq!(s.entries(), &[S::Settings]);
        assert!(!s.can_pop());
        assert!(s.contains(&S::Settings));
    }
}
