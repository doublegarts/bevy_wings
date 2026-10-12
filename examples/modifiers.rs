//! Compose layout, theme paint, shape, elevation and hover scale on a node.
mod support;
use bevy::prelude::*;
use bevy_wings::prelude::*;

#[derive(Component, Default, Clone)]
struct ModifiedCard;
fn main() {
    support::run("Wings: modifiers", configure);
}
fn configure(app: &mut App) {
    app.add_systems(Startup, setup);
}
fn setup(mut commands: Commands) {
    let style = modifier()
        .width(360.0)
        .padding(32.0)
        .background(ColorRole::PrimaryContainer)
        .shape(ShapeRole::Large)
        .elevation(3)
        .hover_scale(1.08);
    commands.spawn_scene(bsn! {
        @WingsScaffold
        Children [
            @WingsColumn { @padding: 48.0, @gap: 24.0 }
            Children [
                @WingsText { @text: "Hover the styled card", @role: TypeRole::HeadlineMedium }
                --
                Node style ModifiedCard bevy::picking::hover::Hovered
                Children [ @WingsText { @text: "A modifier chain becomes ordinary ECS components." } ]
            ]
        ]
    });
}

#[test]
fn modifier_applies_layout() {
    let mut app = support::app("modifiers", true);
    configure(&mut app);
    app.update();
    let node = app
        .world_mut()
        .query_filtered::<&Node, With<ModifiedCard>>()
        .single(app.world())
        .unwrap();
    assert_eq!(node.width, px(360));
    assert_eq!(node.padding, UiRect::all(px(32)));
}
