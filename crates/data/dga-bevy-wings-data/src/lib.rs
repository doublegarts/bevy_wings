//! # dga-bevy-wings-data
//!
//! The **data layer** of the Wings framework. It implements the repository contracts of
//! [`dga_bevy_wings_domain`] on top of interchangeable data sources and moves slow work off
//! the main thread with ECS friendly background loading.
//!
//! | Building block | Purpose |
//! | --- | --- |
//! | [`DataSource`] and [`MemorySource`] | Key value storage contract and a thread safe in memory implementation |
//! | [`RemoteSource`] and [`FakeRemote`] | Contract for a backend (HTTP, database, save file) and a configurable fake for tests and previews |
//! | [`OfflineFirstRepository`] | A domain repository that reads through a local cache, honors a [`CachePolicy`], serves stale data when the remote is down and queues writes for [`sync`](OfflineFirstRepository::sync) |
//! | [`TtlCache`], [`Clock`] | Time based expiry with an injectable clock |
//! | [`Mapper`] | Conversions between transfer objects and domain entities |
//! | [`WingsLoadable`], [`WingsLoadExt`] | Run a closure on Bevy's async compute pool and receive a [`Loadable`] component |
//!
//! ```ignore
//! // Data layer wiring, usually inside a DI module:
//! let repo = OfflineFirstRepository::new(Arc::new(HttpTodos::new(url)))
//!     .with_policy(CachePolicy::CacheFirst)
//!     .with_ttl(Duration::from_secs(60));
//!
//! // Presentation: load in the background, render the Loadable.
//! commands.entity(screen).load(move || repo.all());
//! ```

mod cache;
mod loader;
mod mapper;
mod remote;
mod repository;
mod source;

pub use cache::{CachePolicy, Clock, ManualClock, SystemClock, TtlCache};
pub use dga_bevy_wings_domain::Loadable;
pub use loader::{
    WingsDataAppExt, WingsLoadExt, WingsLoadFinished, WingsLoadTask, WingsLoadable, poll_loads,
};
pub use mapper::{FnMapper, Mapper, mapper};
pub use remote::{FakeRemote, RemoteSource};
pub use repository::OfflineFirstRepository;
pub use source::{DataSource, MemorySource};

/// Commonly used items.
pub mod prelude {
    pub use crate::{
        CachePolicy, Clock, DataSource, FakeRemote, ManualClock, Mapper, MemorySource,
        OfflineFirstRepository, RemoteSource, SystemClock, TtlCache, WingsDataAppExt, WingsLoadExt,
        WingsLoadFinished, WingsLoadable, mapper,
    };
}
