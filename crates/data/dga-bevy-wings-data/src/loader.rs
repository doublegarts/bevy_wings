//! Background loading integrated with the ECS.

use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task, futures::check_ready};

use dga_bevy_wings_domain::{DomainResult, Loadable};

/// The [`Loadable`] state of an entity, for example a screen's data.
///
/// Bind it with MVVM bindings, or read it in systems: `Loading` keeps the previous value so
/// screens do not flash empty while refreshing.
#[derive(Component, Debug, Clone, PartialEq, Deref, DerefMut)]
pub struct WingsLoadable<T: Send + Sync + 'static>(pub Loadable<T>);

impl<T: Send + Sync + 'static> Default for WingsLoadable<T> {
    fn default() -> Self {
        Self(Loadable::Idle)
    }
}

/// A running background load for [`WingsLoadable<T>`].
#[derive(Component)]
pub struct WingsLoadTask<T: Send + 'static>(pub Task<DomainResult<T>>);

/// Triggered on the entity when a background load finished (successfully or not).
#[derive(EntityEvent, Debug, Clone, Copy)]
pub struct WingsLoadFinished {
    /// The entity whose [`WingsLoadable`] was updated.
    pub entity: Entity,
    /// True on success.
    pub success: bool,
}

/// Starts background loads from commands.
pub trait WingsLoadExt {
    /// Runs `work` on the async compute pool and stores its result in this entity's
    /// [`WingsLoadable<T>`]. Starting a new load replaces a running one.
    fn load<T, F>(&mut self, work: F) -> &mut Self
    where
        T: Send + Sync + 'static,
        F: FnOnce() -> DomainResult<T> + Send + 'static;
}

impl WingsLoadExt for EntityCommands<'_> {
    fn load<T, F>(&mut self, work: F) -> &mut Self
    where
        T: Send + Sync + 'static,
        F: FnOnce() -> DomainResult<T> + Send + 'static,
    {
        let task = AsyncComputeTaskPool::get().spawn(async move { work() });
        self.queue(move |mut entity: EntityWorldMut| {
            match entity.get_mut::<WingsLoadable<T>>() {
                Some(mut l) => l.start_loading(),
                None => {
                    entity.insert(WingsLoadable::<T>(Loadable::Loading { previous: None }));
                }
            }
            entity.insert(WingsLoadTask(task));
        })
    }
}

/// Completes finished loads of type `T`.
pub fn poll_loads<T: Send + Sync + 'static>(
    mut commands: Commands,
    mut tasks: Query<(Entity, &mut WingsLoadTask<T>, &mut WingsLoadable<T>)>,
) {
    for (entity, mut task, mut loadable) in &mut tasks {
        if let Some(result) = check_ready(&mut task.0) {
            let success = result.is_ok();
            loadable.finish(result);
            commands.entity(entity).remove::<WingsLoadTask<T>>();
            commands.trigger(WingsLoadFinished { entity, success });
        }
    }
}

/// App extension to enable background loading of `T`.
pub trait WingsDataAppExt {
    /// Polls [`WingsLoadTask<T>`] every frame in `PreUpdate`, so results are visible to
    /// view models and bindings in the same frame.
    fn register_loadable<T: Send + Sync + 'static>(&mut self) -> &mut Self;
}

impl WingsDataAppExt for App {
    fn register_loadable<T: Send + Sync + 'static>(&mut self) -> &mut Self {
        self.add_systems(PreUpdate, poll_loads::<T>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dga_bevy_wings_domain::DomainError;

    #[derive(Resource, Default)]
    struct Finished(Vec<bool>);

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .register_loadable::<Vec<u32>>()
            .init_resource::<Finished>()
            .add_observer(|ev: On<WingsLoadFinished>, mut f: ResMut<Finished>| {
                f.0.push(ev.success)
            });
        app
    }

    fn run_until_done(app: &mut App, e: Entity) {
        for _ in 0..200 {
            app.update();
            if app.world().get::<WingsLoadTask<Vec<u32>>>(e).is_none() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        panic!("load did not finish");
    }

    /// A background load moves the component through Loading to Ready and notifies.
    #[test]
    fn load_success() {
        let mut app = app();
        let e = app.world_mut().spawn_empty().id();
        app.world_mut()
            .commands()
            .entity(e)
            .load(|| Ok(vec![1u32, 2, 3]));
        app.world_mut().flush();
        assert!(
            app.world()
                .get::<WingsLoadable<Vec<u32>>>(e)
                .unwrap()
                .is_loading()
        );
        run_until_done(&mut app, e);
        assert_eq!(
            app.world().get::<WingsLoadable<Vec<u32>>>(e).unwrap().0,
            Loadable::Ready(vec![1, 2, 3])
        );
        assert_eq!(app.world().resource::<Finished>().0, vec![true]);
    }

    /// A failed refresh keeps the previous value.
    #[test]
    fn load_failure_keeps_previous() {
        let mut app = app();
        let e = app
            .world_mut()
            .spawn(WingsLoadable(Loadable::Ready(vec![7u32])))
            .id();
        app.world_mut()
            .commands()
            .entity(e)
            .load(|| Err::<Vec<u32>, _>(DomainError::Unavailable("offline".into())));
        app.world_mut().flush();
        run_until_done(&mut app, e);
        let l = &app.world().get::<WingsLoadable<Vec<u32>>>(e).unwrap().0;
        assert_eq!(l.value(), Some(&vec![7]));
        assert!(l.error().is_some());
        assert_eq!(app.world().resource::<Finished>().0, vec![false]);
    }
}
