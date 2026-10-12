//! Use cases: one business operation each.

use crate::error::DomainResult;

/// A single business operation, for example "complete a todo" or "load the inbox".
///
/// Use cases depend on repository traits, never on concrete storage, so they are trivial to
/// test with in memory fakes. Register them in the DI container and call them from view models.
pub trait UseCase: Send + Sync {
    /// Input of the operation.
    type Input;
    /// Output of the operation.
    type Output;

    /// Runs the operation.
    fn execute(&self, input: Self::Input) -> DomainResult<Self::Output>;
}

/// A use case backed by a closure. Build it with [`use_case`].
pub struct FnUseCase<I, O, F> {
    f: F,
    _marker: core::marker::PhantomData<fn(I) -> O>,
}

impl<I, O, F> UseCase for FnUseCase<I, O, F>
where
    F: Fn(I) -> DomainResult<O> + Send + Sync,
{
    type Input = I;
    type Output = O;

    fn execute(&self, input: I) -> DomainResult<O> {
        (self.f)(input)
    }
}

/// Builds a use case from a closure.
///
/// ```
/// use bevy_wings_domain::*;
/// let double = use_case(|x: i32| Ok(x * 2));
/// assert_eq!(double.execute(21), Ok(42));
/// ```
pub fn use_case<I, O, F>(f: F) -> FnUseCase<I, O, F>
where
    F: Fn(I) -> DomainResult<O> + Send + Sync,
{
    FnUseCase {
        f,
        _marker: core::marker::PhantomData,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DomainError;

    struct Divide;
    impl UseCase for Divide {
        type Input = (i32, i32);
        type Output = i32;
        fn execute(&self, (a, b): (i32, i32)) -> DomainResult<i32> {
            if b == 0 {
                return Err(DomainError::invalid("divisor", "must not be zero"));
            }
            Ok(a / b)
        }
    }

    /// Struct and closure use cases share one trait.
    #[test]
    fn trait_and_closure() {
        assert_eq!(Divide.execute((9, 3)), Ok(3));
        assert!(matches!(
            Divide.execute((1, 0)),
            Err(DomainError::Validation(_))
        ));
        let boxed: Box<dyn UseCase<Input = i32, Output = i32>> =
            Box::new(use_case(|x: i32| Ok(x + 1)));
        assert_eq!(boxed.execute(1), Ok(2));
    }
}
