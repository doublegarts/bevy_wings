//! Click to retarget a spring while preserving its velocity.
mod support;
use bevy::prelude::*;
use bevy_wings::prelude::*;

#[derive(Component, Default, Clone)]
struct AnimatedCard;
fn main() {
    support::run("Wings: animation", configure);
}
fn configure(app: &mut App) {
    app.add_systems(Startup, setup);
}
fn setup(mut commands: Commands) {
    commands.spawn_scene(bsn! {
        @WingsScaffold
        Children [
            @WingsColumn { @padding: 48.0, @gap: 32.0 }
            Children [
                @WingsText { @text: "Spring retargeting", @role: TypeRole::HeadlineMedium }
                --
                @WingsCard AnimatedCard UiTransform
                Children [ @WingsText { @text: "I keep my velocity when you click again." } ]
                --
                @WingsButton { @text: "Toggle scale" }
                on(|_: On<Activate>, mut commands: Commands, cards: Query<(Entity, &UiTransform), With<AnimatedCard>>| {
                    for (entity, transform) in &cards {
                        let scale = if transform.scale.x > 1.1 { 1.0 } else { 1.25 };
                        commands.entity(entity).animate::<ScaleProperty>(Vec2::splat(scale), AnimationSpec::spring());
                    }
                })
            ]
        ]
    });
}

#[test]
fn spring_reaches_target() {
    let mut app = support::app("animation", true);
    configure(&mut app);
    app.update();
    let card = app
        .world_mut()
        .query_filtered::<Entity, With<AnimatedCard>>()
        .single(app.world())
        .unwrap();
    app.world_mut()
        .commands()
        .entity(card)
        .animate::<ScaleProperty>(Vec2::splat(1.25), AnimationSpec::spring());
    for _ in 0..240 {
        app.update();
    }
    assert!((app.world().get::<UiTransform>(card).unwrap().scale.x - 1.25).abs() < 0.01);
}
