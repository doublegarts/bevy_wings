//! Shared window and headless setup for the focused UI examples.

use bevy::prelude::*;
use dga_bevy_wings_framework::prelude::*;

/// Runs a window, or 120 fixed frames when `--headless` is passed.
pub fn run(title: &str, configure: impl FnOnce(&mut App)) {
    let headless = std::env::args().any(|arg| arg == "--headless");
    let mut app = app(title, headless);
    configure(&mut app);
    if headless {
        for _ in 0..120 {
            app.update();
        }
        println!("{title}: 120 headless frames completed");
    } else {
        app.run();
    }
}

/// Builds the same app used by interactive runs and automated smoke tests.
pub fn app(title: &str, headless: bool) -> App {
    let mut app = App::new();
    if headless {
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            bevy::scene::ScenePlugin,
            bevy::input::InputPlugin,
        ))
        .init_asset::<Image>()
        .add_plugins((
            bevy::ui_widgets::ButtonPlugin,
            bevy::ui_widgets::CheckboxPlugin,
            bevy::ui_widgets::RadioGroupPlugin,
            bevy::ui_widgets::SliderPlugin,
        ))
        .add_plugins((
            WingsCorePlugin,
            WingsAnimationPlugin,
            WingsThemePlugin::default(),
            WingsResponsivePlugin::default(),
            WingsModifierPlugin,
            WingsWidgetsPlugin::headless(),
        ))
        .insert_resource(WingsViewportOverride {
            width: 960.0,
            height: 720.0,
        })
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_millis(16),
        ));
    } else {
        app.add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: title.into(),
                resolution: (960, 720).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(WingsUiPlugins)
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera2d);
        });
    }
    app
}
