//! Loading state for presentation.

use crate::error::DomainError;

/// The state of data that is loaded asynchronously.
///
/// Loading and failure keep the previous value, so a screen can keep showing old content with
/// a progress indicator or an error banner instead of flashing empty.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Loadable<T> {
    /// Nothing requested yet.
    #[default]
    Idle,
    /// A request is running.
    Loading {
        /// Value from before the request, if any.
        previous: Option<T>,
    },
    /// The latest request succeeded.
    Ready(T),
    /// The latest request failed.
    Failed {
        /// The error.
        error: DomainError,
        /// Value from before the request, if any.
        previous: Option<T>,
    },
}

impl<T> Loadable<T> {
    /// The current or previous value.
    pub fn value(&self) -> Option<&T> {
        match self {
            Self::Idle => None,
            Self::Ready(v) => Some(v),
            Self::Loading { previous } | Self::Failed { previous, .. } => previous.as_ref(),
        }
    }

    /// True while a request runs.
    pub fn is_loading(&self) -> bool {
        matches!(self, Self::Loading { .. })
    }

    /// True when the latest request succeeded.
    pub fn is_ready(&self) -> bool {
        matches!(self, Self::Ready(_))
    }

    /// The error of the latest request.
    pub fn error(&self) -> Option<&DomainError> {
        match self {
            Self::Failed { error, .. } => Some(error),
            _ => None,
        }
    }

    /// Moves to [`Loadable::Loading`], keeping the current value.
    pub fn start_loading(&mut self) {
        let previous = core::mem::take(self).into_value();
        *self = Self::Loading { previous };
    }

    /// Applies the result of a request.
    pub fn finish(&mut self, result: Result<T, DomainError>) {
        *self = match result {
            Ok(v) => Self::Ready(v),
            Err(error) => Self::Failed {
                error,
                previous: core::mem::take(self).into_value(),
            },
        };
    }

    /// Takes the current or previous value.
    pub fn into_value(self) -> Option<T> {
        match self {
            Self::Idle => None,
            Self::Ready(v) => Some(v),
            Self::Loading { previous } | Self::Failed { previous, .. } => previous,
        }
    }

    /// Maps the value, keeping the state.
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Loadable<U> {
        match self {
            Self::Idle => Loadable::Idle,
            Self::Ready(v) => Loadable::Ready(f(v)),
            Self::Loading { previous } => Loadable::Loading {
                previous: previous.map(f),
            },
            Self::Failed { error, previous } => Loadable::Failed {
                error,
                previous: previous.map(f),
            },
        }
    }
}

impl<T> From<Result<T, DomainError>> for Loadable<T> {
    fn from(result: Result<T, DomainError>) -> Self {
        let mut l = Self::Idle;
        l.finish(result);
        l
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Loading and failing keep the previous value.
    #[test]
    fn keeps_previous_value() {
        let mut l = Loadable::Ready(1);
        l.start_loading();
        assert!(l.is_loading());
        assert_eq!(l.value(), Some(&1));
        l.finish(Err(DomainError::Unauthorized));
        assert_eq!(l.value(), Some(&1));
        assert_eq!(l.error(), Some(&DomainError::Unauthorized));
        l.start_loading();
        l.finish(Ok(2));
        assert_eq!(l, Loadable::Ready(2));
    }

    /// Mapping preserves the state.
    #[test]
    fn map_keeps_state() {
        let l = Loadable::Loading { previous: Some(2) }.map(|v| v * 10);
        assert_eq!(l, Loadable::Loading { previous: Some(20) });
        let r: Loadable<i32> = Ok(5).into();
        assert!(r.is_ready());
        assert_eq!(Loadable::<i32>::Idle.value(), None);
    }
}
