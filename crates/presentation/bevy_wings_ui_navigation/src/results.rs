//! Navigation results: a destination returns a value to the one below it.
//!
//! ```ignore
//! // The picker screen returns the chosen color and closes.
//! fn chosen(mut nav: WingsNavigator<Screen>) {
//!     nav.pop_with_result(Color::srgb(0.9, 0.2, 0.3));
//! }
//!
//! // The screen below receives it as an entity event on its entry root ...
//! bsn! { ... on(|ev: On<WingsNavResultReceived>| {
//!     if let Some(color) = ev.value.get::<Color>() { /* use it */ }
//! }) }
//!
//! // ... and as a message.
//! fn read(mut results: MessageReader<WingsNavResultMessage<Screen>>) { /* ... */ }
//! ```

use std::any::Any;
use std::sync::Arc;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::back_stack::{NavKey, WingsBackStack};
use crate::{WingsNavDisplayState, WingsNavEntry};

/// A value returned by a destination. Cheap to clone.
#[derive(Clone)]
pub struct WingsNavResult(Arc<dyn Any + Send + Sync>);

impl WingsNavResult {
    /// Wraps a value.
    pub fn new<T: Any + Send + Sync>(value: T) -> Self {
        Self(Arc::new(value))
    }

    /// The value, if it is a `T`.
    pub fn get<T: Any>(&self) -> Option<&T> {
        self.0.downcast_ref::<T>()
    }
}

impl core::fmt::Debug for WingsNavResult {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("WingsNavResult(..)")
    }
}

/// Results waiting to be delivered once the back stack settled.
#[derive(Resource)]
pub struct WingsPendingResults<K: NavKey> {
    pending: Vec<(K, WingsNavResult)>,
}

impl<K: NavKey> Default for WingsPendingResults<K> {
    fn default() -> Self {
        Self {
            pending: Vec::new(),
        }
    }
}

/// Sent when a destination popped with a result.
#[derive(Message, Debug, Clone)]
pub struct WingsNavResultMessage<K: NavKey> {
    /// The destination that returned the value.
    pub from: K,
    /// The destination that receives it (the new top).
    pub to: K,
    /// The value.
    pub value: WingsNavResult,
}

/// Triggered on the receiving entry root when a destination above it returned a value.
#[derive(EntityEvent, Debug, Clone)]
pub struct WingsNavResultReceived {
    /// The receiving entry root.
    pub entity: Entity,
    /// The value.
    pub value: WingsNavResult,
}

/// Navigation from systems: push, pop and pop with a result.
#[derive(SystemParam)]
pub struct WingsNavigator<'w, K: NavKey> {
    stack: ResMut<'w, WingsBackStack<K>>,
    results: ResMut<'w, WingsPendingResults<K>>,
}

impl<K: NavKey> WingsNavigator<'_, K> {
    /// Pushes a destination.
    pub fn push(&mut self, key: K) {
        self.stack.push(key);
    }

    /// Pops the top destination.
    pub fn pop(&mut self) -> Option<K> {
        self.stack.pop()
    }

    /// Pops the top destination and returns `value` to the destination below it. Returns
    /// `false` (and keeps the stack) when there is nothing to return to.
    pub fn pop_with_result<T: Any + Send + Sync>(&mut self, value: T) -> bool {
        if !self.stack.can_pop() {
            return false;
        }
        if let Some(from) = self.stack.pop() {
            self.results
                .pending
                .push((from, WingsNavResult::new(value)));
        }
        true
    }

    /// The back stack.
    pub fn stack(&self) -> &WingsBackStack<K> {
        &self.stack
    }
}

/// Delivers pending results to the new top destination.
pub fn deliver_nav_results<K: NavKey>(
    mut commands: Commands,
    stack: Res<WingsBackStack<K>>,
    mut results: ResMut<WingsPendingResults<K>>,
    mut messages: MessageWriter<WingsNavResultMessage<K>>,
    displays: Query<&WingsNavDisplayState<K>>,
    entries: Query<&WingsNavEntry<K>>,
) {
    if results.pending.is_empty() {
        return;
    }
    let Some(to) = stack.top().cloned() else {
        results.pending.clear();
        return;
    };
    let top_index = stack.len() - 1;
    for (from, value) in results.pending.drain(..) {
        messages.write(WingsNavResultMessage {
            from,
            to: to.clone(),
            value: value.clone(),
        });
        for display in &displays {
            if let Some(Some(root)) = display.entities().get(top_index)
                && entries.get(*root).is_ok_and(|e| e.key == to)
            {
                commands.trigger(WingsNavResultReceived {
                    entity: *root,
                    value: value.clone(),
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Results downcast to their type only.
    #[test]
    fn downcast() {
        let r = WingsNavResult::new(42_u32);
        assert_eq!(r.get::<u32>(), Some(&42));
        assert!(r.get::<i32>().is_none());
    }
}
