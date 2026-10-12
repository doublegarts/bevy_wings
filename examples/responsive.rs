//! Resize the window: the grid grows from one column to four.
mod support;
use bevy::prelude::*;
use bevy_wings::prelude::*;

#[derive(Component, Default, Clone)]
struct ResponsiveGrid;
fn main() {
    support::run("Wings: responsive layout", configure);
}
fn configure(app: &mut App) {
    app.add_systems(Startup, setup);
}
fn setup(mut commands: Commands) {
    let columns = WingsResponsiveColumns(WingsResponsive::new(1).medium(2).expanded(3).large(4));
    commands.spawn_scene(bsn! {
        @WingsScaffold
        Children [
            @WingsColumn { @padding: 24.0, @gap: 16.0 }
            Children [
                @WingsText { @text: "Resize me: <600 / 600 / 840 / 1200 px", @role: TypeRole::TitleLarge }
                --
                @WingsGrid { @gap: 16.0 } columns ResponsiveGrid
                Children [
                    @WingsCard Children [ @WingsText { @text: "One" } ]
                    --
                    @WingsCard Children [ @WingsText { @text: "Two" } ]
                    --
                    @WingsCard Children [ @WingsText { @text: "Three" } ]
                    --
                    @WingsCard Children [ @WingsText { @text: "Four" } ]
                ]
            ]
        ]
    });
}

#[test]
fn grid_changes_with_viewport() {
    let mut app = support::app("responsive", true);
    configure(&mut app);
    app.update();
    let grid = app
        .world_mut()
        .query_filtered::<Entity, With<ResponsiveGrid>>()
        .single(app.world())
        .unwrap();
    let wide = app
        .world()
        .get::<Node>(grid)
        .unwrap()
        .grid_template_columns
        .clone();
    app.world_mut()
        .resource_mut::<WingsViewportOverride>()
        .width = 400.0;
    app.update();
    assert_eq!(
        app.world().resource::<WingsViewport>().width_class,
        WidthClass::Compact
    );
    assert_ne!(
        wide,
        app.world().get::<Node>(grid).unwrap().grid_template_columns
    );
}
