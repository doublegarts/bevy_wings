//! Toggle light/dark mode and choose a seeded color scheme.
mod support;
use bevy::prelude::*;
use bevy_wings::prelude::*;

fn main() {
    support::run("Wings: theme", configure);
}
fn configure(app: &mut App) {
    app.add_systems(Startup, setup);
}
fn setup(mut commands: Commands) {
    commands.spawn_scene(bsn! {
        @WingsScaffold
        Children [
            @WingsColumn { @padding: 32.0, @gap: 16.0 }
            Children [
                @WingsText { @text: "Theme roles update together", @role: TypeRole::HeadlineMedium }
                --
                @WingsButton { @text: "Toggle light / dark" }
                on(|_: On<Activate>, mut theme: ResMut<WingsTheme>| theme.toggle_dark())
                --
                @WingsButton { @text: "Use a green seed", @variant: WingsButtonVariant::Outlined }
                on(|_: On<Activate>, mut theme: ResMut<WingsTheme>| {
                    let mode = theme.mode;
                    *theme = WingsTheme::from_seed(Color::srgb(0.15, 0.65, 0.35));
                    theme.mode = mode;
                })
                --
                @WingsCard Children [ @WingsText { @text: "Surface, content, border and typography come from roles." } ]
            ]
        ]
    });
}

#[test]
fn theme_repaints_after_toggle() {
    let mut app = support::app("theme", true);
    configure(&mut app);
    app.update();
    let before = app
        .world_mut()
        .query::<&BackgroundColor>()
        .iter(app.world())
        .map(|c| c.0)
        .collect::<Vec<_>>();
    app.world_mut().resource_mut::<WingsTheme>().toggle_dark();
    for _ in 0..120 {
        app.update();
    }
    let after = app
        .world_mut()
        .query::<&BackgroundColor>()
        .iter(app.world())
        .map(|c| c.0)
        .collect::<Vec<_>>();
    assert!(!before.is_empty());
    assert_ne!(before, after);
}
