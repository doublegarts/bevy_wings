//! Errors shared by every layer.

use core::fmt;

/// A broken validation rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldError {
    /// Field name.
    pub field: &'static str,
    /// Human readable message.
    pub message: String,
}

impl fmt::Display for FieldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}

/// Errors produced by use cases and repositories.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainError {
    /// Input broke one or more rules.
    Validation(Vec<FieldError>),
    /// The requested item does not exist.
    NotFound {
        /// What was looked up, for example `"todo 7"`.
        what: String,
    },
    /// The operation conflicts with the current state (duplicate, stale version).
    Conflict(String),
    /// The caller is not allowed to do this.
    Unauthorized,
    /// A dependency (network, disk, service) is unavailable. Retrying may help.
    Unavailable(String),
    /// Anything else.
    Unexpected(String),
}

impl DomainError {
    /// Shorthand for [`DomainError::NotFound`].
    pub fn not_found(what: impl Into<String>) -> Self {
        Self::NotFound { what: what.into() }
    }

    /// Shorthand for a single field validation error.
    pub fn invalid(field: &'static str, message: impl Into<String>) -> Self {
        Self::Validation(vec![FieldError {
            field,
            message: message.into(),
        }])
    }

    /// True when retrying the same operation later may succeed.
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::Unavailable(_))
    }

    /// Validation errors of one field.
    pub fn field_errors(&self, field: &str) -> Vec<&FieldError> {
        match self {
            Self::Validation(errors) => errors.iter().filter(|e| e.field == field).collect(),
            _ => Vec::new(),
        }
    }
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(errors) => {
                write!(f, "invalid input")?;
                for (i, e) in errors.iter().enumerate() {
                    write!(f, "{}{e}", if i == 0 { ": " } else { ", " })?;
                }
                Ok(())
            }
            Self::NotFound { what } => write!(f, "{what} not found"),
            Self::Conflict(m) => write!(f, "conflict: {m}"),
            Self::Unauthorized => write!(f, "unauthorized"),
            Self::Unavailable(m) => write!(f, "unavailable: {m}"),
            Self::Unexpected(m) => write!(f, "unexpected error: {m}"),
        }
    }
}

impl std::error::Error for DomainError {}

/// Result of domain operations.
pub type DomainResult<T> = Result<T, DomainError>;

#[cfg(test)]
mod tests {
    use super::*;

    /// Messages list every broken field.
    #[test]
    fn display_lists_fields() {
        let e = DomainError::Validation(vec![
            FieldError {
                field: "name",
                message: "required".into(),
            },
            FieldError {
                field: "age",
                message: "too low".into(),
            },
        ]);
        assert_eq!(e.to_string(), "invalid input: name: required, age: too low");
        assert_eq!(e.field_errors("age").len(), 1);
        assert_eq!(
            DomainError::not_found("todo 3").to_string(),
            "todo 3 not found"
        );
    }

    /// Only unavailability is retryable.
    #[test]
    fn retryable() {
        assert!(DomainError::Unavailable("offline".into()).is_retryable());
        assert!(!DomainError::Unauthorized.is_retryable());
    }
}
