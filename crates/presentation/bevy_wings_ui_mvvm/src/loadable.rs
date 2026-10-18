//! [`Loadable`] bindings: show progress, content or an error from one view model field.
//!
//! ```ignore
//! #[derive(Component)]
//! struct NotesVm { notes: Loadable<Vec<Note>> }
//!
//! bsn! {
//!     @WingsColumn
//!     Children [
//!         @WingsLinearProgress   {bind_loading(|vm: &NotesVm| &vm.notes)}
//!         --
//!         @WingsText             {bind_error_text(|vm: &NotesVm| &vm.notes)} {bind_failed(|vm: &NotesVm| &vm.notes)}
//!         --
//!         @WingsColumn           {bind_loaded(|vm: &NotesVm| &vm.notes)}
//!         Children [ /* the list */ ]
//!     ]
//! }
//! ```
//!
//! The helpers build ordinary [`WingsBindVisible`] and [`WingsBindText`] bindings, so they
//! need nothing beyond [`register_view_model`](crate::WingsMvvmAppExt::register_view_model).

use bevy_wings_domain::Loadable;

use crate::{WingsBindText, WingsBindVisible, WingsViewModel};

/// Phase of a [`Loadable`] as the user sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WingsLoadPhase {
    /// Nothing requested yet.
    Idle,
    /// Loading and nothing to show yet.
    Loading,
    /// Loading a newer value while showing the previous one.
    Refreshing,
    /// A value is shown.
    Ready,
    /// Failed with nothing to show.
    Failed,
    /// Failed while showing a previous value.
    Stale,
}

impl WingsLoadPhase {
    /// The phase of a loadable.
    pub fn of<T>(l: &Loadable<T>) -> Self {
        match (l.is_loading(), l.error().is_some(), l.value().is_some()) {
            (true, _, false) => Self::Loading,
            (true, _, true) => Self::Refreshing,
            (false, true, false) => Self::Failed,
            (false, true, true) => Self::Stale,
            (false, false, true) => Self::Ready,
            (false, false, false) => Self::Idle,
        }
    }
}

/// Visible while loading with nothing to show yet (a progress indicator or skeletons).
pub fn bind_loading<VM, T, F>(select: F) -> WingsBindVisible<VM>
where
    VM: WingsViewModel,
    F: for<'a> Fn(&'a VM) -> &'a Loadable<T> + Send + Sync + 'static,
{
    WingsBindVisible::new(move |vm| WingsLoadPhase::of(select(vm)) == WingsLoadPhase::Loading)
}

/// Visible while a newer value loads over a shown one (a small refresh indicator).
pub fn bind_refreshing<VM, T, F>(select: F) -> WingsBindVisible<VM>
where
    VM: WingsViewModel,
    F: for<'a> Fn(&'a VM) -> &'a Loadable<T> + Send + Sync + 'static,
{
    WingsBindVisible::new(move |vm| WingsLoadPhase::of(select(vm)) == WingsLoadPhase::Refreshing)
}

/// Visible whenever there is a value to show (also while refreshing or after a failed
/// refresh).
pub fn bind_loaded<VM, T, F>(select: F) -> WingsBindVisible<VM>
where
    VM: WingsViewModel,
    F: for<'a> Fn(&'a VM) -> &'a Loadable<T> + Send + Sync + 'static,
{
    WingsBindVisible::new(move |vm| select(vm).value().is_some())
}

/// Visible when the last load failed.
pub fn bind_failed<VM, T, F>(select: F) -> WingsBindVisible<VM>
where
    VM: WingsViewModel,
    F: for<'a> Fn(&'a VM) -> &'a Loadable<T> + Send + Sync + 'static,
{
    WingsBindVisible::new(move |vm| select(vm).error().is_some())
}

/// The error message of the last failed load (empty otherwise).
pub fn bind_error_text<VM, T, F>(select: F) -> WingsBindText<VM>
where
    VM: WingsViewModel,
    F: for<'a> Fn(&'a VM) -> &'a Loadable<T> + Send + Sync + 'static,
{
    WingsBindText::new(move |vm| {
        select(vm)
            .error()
            .map(|e| e.to_string())
            .unwrap_or_default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_wings_domain::DomainError;

    /// Phases follow the loadable's lifecycle.
    #[test]
    fn phases() {
        let mut l: Loadable<u32> = Loadable::Idle;
        assert_eq!(WingsLoadPhase::of(&l), WingsLoadPhase::Idle);
        l.start_loading();
        assert_eq!(WingsLoadPhase::of(&l), WingsLoadPhase::Loading);
        l.finish(Ok(3));
        assert_eq!(WingsLoadPhase::of(&l), WingsLoadPhase::Ready);
        l.start_loading();
        assert_eq!(WingsLoadPhase::of(&l), WingsLoadPhase::Refreshing);
        l.finish(Err(DomainError::Unavailable("down".into())));
        assert_eq!(WingsLoadPhase::of(&l), WingsLoadPhase::Stale);
        let failed: Loadable<u32> = {
            let mut f = Loadable::Idle;
            f.start_loading();
            f.finish(Err(DomainError::Unavailable("down".into())));
            f
        };
        assert_eq!(WingsLoadPhase::of(&failed), WingsLoadPhase::Failed);
    }
}
