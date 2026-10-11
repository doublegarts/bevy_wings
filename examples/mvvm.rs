//! A component view model, change-driven text bindings and button intents.
mod support;
use bevy::prelude::*;
use dga_bevy_wings_framework::prelude::*;

#[derive(Component, Default, Clone)]
struct Counter {
    count: u32,
}
impl WingsViewModel for Counter {}
#[derive(Component, Default, Clone)]
struct IncrementButton;
fn main() {
    support::run("Wings: MVVM", configure);
}
fn configure(app: &mut App) {
    app.register_view_model::<Counter>()
        .add_systems(Startup, setup);
}
fn setup(mut commands: Commands) {
    let label = bind_text(|vm: &Counter| format!("Count: {}", vm.count));
    commands.spawn_scene(bsn! {
        @WingsScaffold Counter
        Children [
            @WingsColumn { @padding: 32.0, @gap: 16.0 }
            Children [
                @WingsText { @role: TypeRole::HeadlineMedium } label
                --
                @WingsButton { @text: "Increment" } IncrementButton
                on(on_activate(|vm: &mut Counter| vm.count += 1))
                --
                @WingsButton { @text: "Reset", @variant: WingsButtonVariant::Outlined }
                on(on_activate(|vm: &mut Counter| vm.count = 0))
            ]
        ]
    });
}

#[test]
fn intent_updates_bound_text() {
    let mut app = support::app("mvvm", true);
    configure(&mut app);
    app.update();
    let button = app
        .world_mut()
        .query_filtered::<Entity, With<IncrementButton>>()
        .iter(app.world())
        .next()
        .unwrap();
    app.world_mut().trigger(Activate { entity: button });
    app.update();
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|t| t.0 == "Count: 1")
    );
}
