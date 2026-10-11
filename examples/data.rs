//! Cached reads, offline writes and synchronization after reconnecting.
use dga_bevy_wings_framework::prelude::*;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq)]
struct Note {
    id: Id<Note>,
    title: String,
}
impl DomainEntity for Note {
    fn id(&self) -> Id<Note> {
        self.id
    }
}

fn main() -> DomainResult<()> {
    let remote = Arc::new(FakeRemote::with_items([Note {
        id: Id::new(1),
        title: "Read me".into(),
    }]));
    let repo = OfflineFirstRepository::new(remote.clone());
    println!("First read: {:?}", repo.all()?);
    let _ = repo.all()?;
    println!("Two reads, {} remote call", remote.calls());
    remote.set_online(false);
    repo.save(Note {
        id: Id::new(2),
        title: "Written offline".into(),
    })?;
    println!(
        "Offline: {:?}; queued writes: {}",
        repo.all()?,
        repo.pending_writes()
    );
    remote.set_online(true);
    println!(
        "Synced {} write; remote: {:?}",
        repo.sync()?,
        remote.items()
    );
    Ok(())
}
