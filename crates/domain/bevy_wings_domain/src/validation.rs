//! Declarative validation.

use crate::error::{DomainError, FieldError};

type Rule<T> = Box<dyn Fn(&T) -> bool + Send + Sync>;

/// A list of rules checked together. Every broken rule is reported, not only the first one,
/// so forms can highlight all invalid fields at once.
///
/// ```
/// use bevy_wings_domain::{Validator, rules};
/// struct Signup { name: String, age: u32 }
/// let v = Validator::<Signup>::new()
///     .rule("name", |s| rules::not_blank(&s.name), "name is required")
///     .rule("age", |s| rules::in_range(s.age, 13, 130), "age must be 13 or more");
/// let err = v.validate(&Signup { name: " ".into(), age: 9 }).unwrap_err();
/// assert_eq!(err.field_errors("name").len(), 1);
/// assert_eq!(err.field_errors("age").len(), 1);
/// ```
pub struct Validator<T> {
    rules: Vec<(&'static str, Rule<T>, String)>,
}

impl<T> Default for Validator<T> {
    fn default() -> Self {
        Self { rules: Vec::new() }
    }
}

impl<T> Validator<T> {
    /// An empty validator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a rule: `check` must return true for valid input.
    pub fn rule(
        mut self,
        field: &'static str,
        check: impl Fn(&T) -> bool + Send + Sync + 'static,
        message: impl Into<String>,
    ) -> Self {
        self.rules.push((field, Box::new(check), message.into()));
        self
    }

    /// Number of rules.
    pub fn len(&self) -> usize {
        self.rules.len()
    }

    /// True without rules.
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Every broken rule.
    pub fn errors(&self, value: &T) -> Vec<FieldError> {
        self.rules
            .iter()
            .filter(|(_, check, _)| !check(value))
            .map(|(field, _, message)| FieldError {
                field,
                message: message.clone(),
            })
            .collect()
    }

    /// `Ok` when every rule holds, otherwise [`DomainError::Validation`] with all errors.
    pub fn validate(&self, value: &T) -> Result<(), DomainError> {
        let errors = self.errors(value);
        if errors.is_empty() {
            Ok(())
        } else {
            Err(DomainError::Validation(errors))
        }
    }
}

/// Reusable rule predicates.
pub mod rules {
    /// Not empty after trimming whitespace.
    pub fn not_blank(s: &str) -> bool {
        !s.trim().is_empty()
    }

    /// Character count within `min..=max`.
    pub fn length_between(s: &str, min: usize, max: usize) -> bool {
        (min..=max).contains(&s.chars().count())
    }

    /// Value within `min..=max`.
    pub fn in_range<N: PartialOrd>(value: N, min: N, max: N) -> bool {
        value >= min && value <= max
    }

    /// A simple email shape check (`local@domain.tld`), not a full RFC parser.
    pub fn looks_like_email(s: &str) -> bool {
        let Some((local, domain)) = s.split_once('@') else {
            return false;
        };
        !local.is_empty()
            && domain.contains('.')
            && !domain.starts_with('.')
            && !domain.ends_with('.')
            && !s.contains(char::is_whitespace)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Built in predicates.
    #[test]
    fn predicates() {
        assert!(rules::not_blank("a"));
        assert!(!rules::not_blank("  "));
        assert!(rules::length_between("héllo", 5, 5));
        assert!(rules::in_range(5, 1, 10));
        assert!(!rules::in_range(0.5, 1.0, 2.0));
        assert!(rules::looks_like_email("ada@example.com"));
        assert!(!rules::looks_like_email("ada@example"));
        assert!(!rules::looks_like_email("a da@example.com"));
    }

    /// A valid value passes, and an empty validator accepts anything.
    #[test]
    fn passes() {
        let v = Validator::<i32>::new().rule("n", |n| *n > 0, "positive");
        assert!(v.validate(&3).is_ok());
        assert_eq!(v.len(), 1);
        assert!(Validator::<i32>::new().validate(&-1).is_ok());
    }
}
