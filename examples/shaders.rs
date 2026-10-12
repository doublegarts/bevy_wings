//! Embedded WESL materials: aurora, glass, shimmer, ripple and circular progress.
mod support;
use bevy::prelude::*;
use bevy::ui_render::prelude::MaterialNode;
use bevy_wings::prelude::*;

fn main() {
    support::run("Wings: shaders", configure);
}
fn configure(app: &mut App) {
    // Headless mode still constructs the materials, without initializing a GPU.
    if !app
        .world()
        .contains_resource::<Assets<WingsAuroraMaterial>>()
    {
        app.init_asset::<WingsAuroraMaterial>();
    }
    if !app
        .world()
        .contains_resource::<Assets<WingsGlassMaterial>>()
    {
        app.init_asset::<WingsGlassMaterial>();
    }
    app.add_systems(Startup, setup);
}
fn setup(
    mut commands: Commands,
    mut auroras: ResMut<Assets<WingsAuroraMaterial>>,
    mut glasses: ResMut<Assets<WingsGlassMaterial>>,
) {
    let aurora = MaterialNode(auroras.add(WingsAuroraMaterial::new(
        Color::srgb(0.3, 0.2, 0.8),
        Color::srgb(0.1, 0.8, 0.5),
        Color::srgb(0.03, 0.04, 0.09),
    )));
    let glass =
        MaterialNode(glasses.add(WingsGlassMaterial::new(Color::srgba(0.8, 0.9, 1.0, 0.3))));
    let root = commands.spawn_scene(bsn! { @WingsScaffold }).id();
    let column = commands
        .spawn_scene(bsn! {
            @WingsColumn { @padding: 24.0, @gap: 24.0 } ChildOf(root)
        })
        .id();
    commands.spawn_scene(bsn! {
        @WingsText { @text: "Embedded WESL materials", @role: TypeRole::HeadlineMedium } ChildOf(column)
    });
    // Material handles are inserted as ordinary components, outside the scene template.
    commands.spawn((
        Node {
            width: px(500),
            height: px(150),
            ..default()
        },
        aurora,
        ChildOf(column),
    ));
    let panel = commands
        .spawn((
            Node {
                width: px(500),
                height: px(80),
                ..default()
            },
            glass,
            ChildOf(column),
        ))
        .id();
    commands.spawn_scene(bsn! { @WingsText { @text: "Procedural frosted glass" } ChildOf(panel) });
    commands
        .spawn_scene(bsn! { @WingsSkeleton { @width: px(320), @height: px(48) } ChildOf(column) });
    commands.spawn_scene(
        bsn! { @WingsCircularProgress { @progress: 0.7, @size: 64.0 } ChildOf(column) },
    );
    commands.spawn_scene(bsn! { @WingsButton { @text: "Click for a ripple" } ChildOf(column) });
}

#[test]
fn materials_construct_headless() {
    let mut app = support::app("shaders", true);
    configure(&mut app);
    app.update();
    assert_eq!(
        app.world().resource::<Assets<WingsAuroraMaterial>>().len(),
        1
    );
    assert_eq!(
        app.world().resource::<Assets<WingsGlassMaterial>>().len(),
        1
    );
}
