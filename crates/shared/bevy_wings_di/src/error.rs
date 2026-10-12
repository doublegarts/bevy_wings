//! Dependency injection errors.

use core::fmt;

/// Errors raised while loading modules or resolving dependencies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiError {
    /// No definition exists for the requested type (and qualifier).
    Missing {
        /// Requested type.
        type_name: &'static str,
        /// Requested qualifier.
        qualifier: Option<&'static str>,
        /// Resolution chain that led to the request, outermost first.
        chain: Vec<&'static str>,
    },
    /// The dependency graph contains a cycle.
    Cycle {
        /// The cycle, starting and ending with the same type.
        chain: Vec<&'static str>,
    },
    /// A scoped definition was requested outside of a scope.
    NoScope {
        /// Requested type.
        type_name: &'static str,
    },
    /// A module tried to redefine an existing definition without allowing overrides.
    Duplicate {
        /// Redefined type.
        type_name: &'static str,
        /// Redefined qualifier.
        qualifier: Option<&'static str>,
    },
}

impl fmt::Display for DiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiError::Missing {
                type_name,
                qualifier,
                chain,
            } => {
                write!(f, "no definition for `{type_name}`")?;
                if let Some(q) = qualifier {
                    write!(f, " named \"{q}\"")?;
                }
                if !chain.is_empty() {
                    write!(f, " (required by {})", chain.join(" -> "))?;
                }
                Ok(())
            }
            DiError::Cycle { chain } => write!(f, "dependency cycle: {}", chain.join(" -> ")),
            DiError::NoScope { type_name } => {
                write!(
                    f,
                    "`{type_name}` is scoped and was requested outside of a scope"
                )
            }
            DiError::Duplicate {
                type_name,
                qualifier,
            } => {
                write!(f, "`{type_name}`")?;
                if let Some(q) = qualifier {
                    write!(f, " named \"{q}\"")?;
                }
                write!(f, " is already defined; use `allow_override` to replace it")
            }
        }
    }
}

impl std::error::Error for DiError {}
