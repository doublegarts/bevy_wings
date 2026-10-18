//! # bevy_wings_domain
//!
//! The **domain layer** of the Wings framework: the business rules of an app, written in plain
//! Rust with no engine, UI or storage dependency. Everything here can be unit tested in
//! microseconds and reused by any presentation (Bevy UI, a server, a CLI tool).
//!
//! | Building block | Purpose |
//! | --- | --- |
//! | [`DomainEntity`] and [`Id<T>`] | Entities with typed ids, so a `Id<User>` never mixes with an `Id<Order>` |
//! | [`UseCase`] | One business operation with an input and an output ([`use_case`] builds one from a closure) |
//! | [`DomainError`] and [`DomainResult`] | One error type shared by data, domain and presentation |
//! | [`Validator`] | Declarative field validation that reports every broken rule at once |
//! | [`ReadRepository`] and [`WriteRepository`] | Contracts the data layer implements |
//! | [`PageRequest`] and [`Page`] | Paging |
//! | [`Loadable<T>`] | Loading state for the presentation layer (idle, loading, ready, failed) |
//! | [`UseCaseExt`] | Pipelines: [`retry`](UseCaseExt::retry), [`timeout`](UseCaseExt::timeout), [`logged`](UseCaseExt::logged) and [`map`](UseCaseExt::map) decorators |
//!
//! With the `serde` feature, [`Id`], [`PageRequest`] and [`Page`] serialize, so entities that
//! contain them can be persisted.
//!
//! Dependencies point inwards: the data layer implements the repository traits defined here,
//! and the presentation layer calls use cases and renders [`Loadable`] values.
//!
//! ```
//! use bevy_wings_domain::*;
//!
//! #[derive(Clone, Debug, PartialEq)]
//! struct Todo { id: Id<Todo>, title: String, done: bool }
//! impl DomainEntity for Todo {
//!     fn id(&self) -> Id<Todo> { self.id }
//! }
//!
//! let validator = Validator::<Todo>::new().rule("title", |t| !t.title.trim().is_empty(), "title is required");
//! let complete = use_case(move |todo: Todo| {
//!     validator.validate(&todo)?;
//!     Ok(Todo { done: true, ..todo })
//! });
//!
//! let done = complete.execute(Todo { id: Id::new(1), title: "Ship 0.1".into(), done: false }).unwrap();
//! assert!(done.done);
//! ```

mod entity;
mod error;
mod loadable;
mod page;
mod pipeline;
mod repository;
mod use_case;
mod validation;

pub use entity::{DomainEntity, Id};
pub use error::{DomainError, DomainResult, FieldError};
pub use loadable::Loadable;
pub use page::{Page, PageRequest};
pub use pipeline::{Logged, Mapped, RetryPolicy, Retrying, Timeout, UseCaseEvent, UseCaseExt};
pub use repository::{ReadRepository, WriteRepository};
pub use use_case::{FnUseCase, UseCase, use_case};
pub use validation::{Validator, rules};

/// Commonly used items.
pub mod prelude {
    pub use crate::{
        DomainEntity, DomainError, DomainResult, FieldError, FnUseCase, Id, Loadable, Page,
        PageRequest, ReadRepository, RetryPolicy, UseCase, UseCaseEvent, UseCaseExt, Validator,
        WriteRepository, rules, use_case,
    };
}
