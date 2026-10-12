//! Load on a background task, keep previous data while refreshing and show failures.
mod support;
use bevy::prelude::*;
use bevy_wings::prelude::*;

#[derive(Component, Default, Clone)]
struct ResultLabel;

fn main() {
    support::run("Wings: background loading", configure);
}
fn configure(app: &mut App) {
    app.register_loadable::<String>()
        .add_systems(Startup, setup)
        .add_systems(Update, render);
}
fn setup(mut commands: Commands) {
    commands.spawn_scene(bsn! {
        @WingsScaffold
        Children [
            @WingsColumn { @padding: 32.0, @gap: 16.0 }
            Children [
                @WingsText { @text: "Background loading", @role: TypeRole::HeadlineMedium }
                --
                @WingsText { @text: "Press Load" } ResultLabel
                --
                @WingsButton { @text: "Load / refresh" }
                on(|_: On<Activate>, labels: Query<Entity, With<ResultLabel>>, mut commands: Commands| {
                    for label in &labels {
                        commands.entity(label).load(|| {
                            std::thread::sleep(std::time::Duration::from_millis(500));
                            Ok("Loaded on the compute pool".to_owned())
                        });
                    }
                })
                --
                @WingsButton { @text: "Simulate failure", @variant: WingsButtonVariant::Outlined }
                on(|_: On<Activate>, labels: Query<Entity, With<ResultLabel>>, mut commands: Commands| {
                    for label in &labels {
                        commands.entity(label).load(|| Err::<String, _>(DomainError::Unavailable("Offline".into())));
                    }
                })
            ]
        ]
    });
}
fn render(mut labels: Query<(&WingsLoadable<String>, &mut Text), Changed<WingsLoadable<String>>>) {
    for (load, mut text) in &mut labels {
        text.0 = format!(
            "{}{}{}",
            load.value().map(String::as_str).unwrap_or("No data"),
            if load.is_loading() {
                " (refreshing...)"
            } else {
                ""
            },
            load.error()
                .map(|error| format!("; {error}"))
                .unwrap_or_default(),
        );
    }
}

#[test]
fn completed_load_updates_label() {
    let mut app = support::app("loading", true);
    configure(&mut app);
    app.update();
    let label = app
        .world_mut()
        .query_filtered::<Entity, With<ResultLabel>>()
        .single(app.world())
        .unwrap();
    app.world_mut()
        .commands()
        .entity(label)
        .load(|| Ok("Loaded".to_owned()));
    for _ in 0..200 {
        app.update();
        if app.world().get::<Text>(label).unwrap().0 == "Loaded" {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    panic!("background result never reached the text binding");
}
