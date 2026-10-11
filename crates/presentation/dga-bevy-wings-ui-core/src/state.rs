//! Version tracked reactive values.
//!
//! [`WingsState`] wraps a value and increments a version counter every time the value really
//! changes. Consumers remember the last version they saw and can cheaply ask
//! [`WingsState::changed_since`] instead of comparing whole values every frame.
//!
//! [`WingsMemo`] caches a derived value and recomputes it only when the source version moves.

/// A value with a monotonically increasing version counter.
///
/// ```
/// use dga_bevy_wings_ui_core::WingsState;
///
/// let mut count = WingsState::new(0);
/// let seen = count.version();
/// count.set(1);
/// assert!(count.changed_since(seen));
/// assert_eq!(*count.get(), 1);
/// ```
#[derive(Debug, Clone, Default)]
pub struct WingsState<T> {
    value: T,
    version: u64,
}

impl<T> WingsState<T> {
    /// Creates a new state at version `0`.
    pub const fn new(value: T) -> Self {
        Self { value, version: 0 }
    }

    /// Returns a shared reference to the current value.
    pub fn get(&self) -> &T {
        &self.value
    }

    /// Current version. Starts at `0` and grows by one on every change.
    pub fn version(&self) -> u64 {
        self.version
    }

    /// True when the value changed after `version` was observed.
    pub fn changed_since(&self, version: u64) -> bool {
        self.version != version
    }

    /// Replaces the value unconditionally and bumps the version.
    pub fn replace(&mut self, value: T) -> T {
        self.version = self.version.wrapping_add(1);
        core::mem::replace(&mut self.value, value)
    }

    /// Mutates the value in place through a closure and bumps the version.
    pub fn update(&mut self, f: impl FnOnce(&mut T)) {
        f(&mut self.value);
        self.version = self.version.wrapping_add(1);
    }

    /// Consumes the state and returns the inner value.
    pub fn into_inner(self) -> T {
        self.value
    }
}

impl<T: PartialEq> WingsState<T> {
    /// Sets the value, bumping the version only when it is different from the current one.
    /// Returns `true` when the value changed.
    pub fn set(&mut self, value: T) -> bool {
        if self.value == value {
            return false;
        }
        self.value = value;
        self.version = self.version.wrapping_add(1);
        true
    }
}

impl<T> core::ops::Deref for WingsState<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.value
    }
}

impl<T> From<T> for WingsState<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

/// A cached derived value that is recomputed only when its source version changes.
///
/// ```
/// use dga_bevy_wings_ui_core::{WingsMemo, WingsState};
///
/// let items = WingsState::new(vec![1, 2, 3]);
/// let mut total = WingsMemo::default();
/// assert_eq!(*total.get_or_compute(items.version(), || items.iter().sum::<i32>()), 6);
/// ```
#[derive(Debug, Clone)]
pub struct WingsMemo<T> {
    cached: Option<(u64, T)>,
}

impl<T> Default for WingsMemo<T> {
    fn default() -> Self {
        Self { cached: None }
    }
}

impl<T> WingsMemo<T> {
    /// Returns the cached value for `version`, computing it with `compute` when the cache is
    /// empty or stale.
    pub fn get_or_compute(&mut self, version: u64, compute: impl FnOnce() -> T) -> &T {
        let stale = !matches!(&self.cached, Some((v, _)) if *v == version);
        if stale {
            self.cached = Some((version, compute()));
        }
        &self.cached.as_ref().expect("memo was just filled").1
    }

    /// Drops the cached value so the next access recomputes it.
    pub fn invalidate(&mut self) {
        self.cached = None;
    }

    /// True when a value is cached.
    pub fn is_cached(&self) -> bool {
        self.cached.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Setting an equal value must not bump the version (avoids useless UI updates).
    #[test]
    fn set_same_value_keeps_version() {
        let mut s = WingsState::new(5);
        assert!(!s.set(5));
        assert_eq!(s.version(), 0);
        assert!(s.set(6));
        assert_eq!(s.version(), 1);
    }

    /// `update` always bumps, because the closure may have mutated the value.
    #[test]
    fn update_bumps_version() {
        let mut s = WingsState::new(vec![1]);
        s.update(|v| v.push(2));
        assert_eq!(s.get(), &vec![1, 2]);
        assert_eq!(s.version(), 1);
    }

    /// `replace` returns the previous value.
    #[test]
    fn replace_returns_old_value() {
        let mut s = WingsState::new("a");
        assert_eq!(s.replace("b"), "a");
        assert_eq!(*s, "b");
        assert!(s.changed_since(0));
    }

    /// The memo recomputes only when the version changes.
    #[test]
    fn memo_recomputes_only_on_version_change() {
        let mut memo = WingsMemo::default();
        let mut calls = 0;
        memo.get_or_compute(1, || {
            calls += 1;
            10
        });
        memo.get_or_compute(1, || {
            calls += 1;
            20
        });
        assert_eq!(calls, 1);
        let v = *memo.get_or_compute(2, || {
            calls += 1;
            30
        });
        assert_eq!(v, 30);
        assert_eq!(calls, 2);
    }

    /// Invalidating the memo forces a recompute.
    #[test]
    fn memo_invalidate() {
        let mut memo = WingsMemo::default();
        memo.get_or_compute(0, || 1);
        assert!(memo.is_cached());
        memo.invalidate();
        assert!(!memo.is_cached());
        assert_eq!(*memo.get_or_compute(0, || 2), 2);
    }
}
