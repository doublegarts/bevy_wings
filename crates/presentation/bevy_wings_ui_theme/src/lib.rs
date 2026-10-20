//! # bevy_wings_ui_theme
//!
//! ECS native theming for Bevy UI.
//!
//! * [`WingsTheme`]: one resource holding the light and dark
//!   [`WingsColorScheme`](scheme::WingsColorScheme), typography, shapes, spacing, elevation
//!   and motion tokens.
//! * Paint components ([`WingsBackground`](paint::WingsBackground),
//!   [`WingsForeground`](paint::WingsForeground), ...) reference semantic
//!   [`ColorRole`](scheme::ColorRole)s instead of raw colors.
//! * The incremental [`paint_system`](paint::paint_system) recolors only what changed, and the
//!   whole tree in a single pass when the theme flips, with an animated cross fade.
//! * [`WingsLocalTheme`](theme::WingsLocalTheme) forces a subtree light or dark.
//! * `WingsThemeMode::System` follows the operating system preference.
//! * [`WingsDesign`](design::WingsDesign) selects Material or the Apple (SwiftUI) look. The
//!   default theme uses the platform's own design.
//!
//! ## Example: twenty buttons, one switch
//!
//! ```ignore
//! fn setup(mut commands: Commands) {
//!     for _ in 0..20 {
//!         // White in the light scheme, near black in the dark scheme.
//!         commands.spawn((Node::default(), WingsBackground::role(ColorRole::Surface)));
//!     }
//! }
//!
//! fn switch(keys: Res<ButtonInput<KeyCode>>, mut theme: ResMut<WingsTheme>) {
//!     if keys.just_pressed(KeyCode::KeyT) {
//!         theme.toggle_dark(); // every surface recolors next frame
//!     }
//! }
//! ```

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/doublegarts/bevy_wings/main/assets/branding/bevy-wings-icon.png",
    html_favicon_url = "https://raw.githubusercontent.com/doublegarts/bevy_wings/main/assets/branding/bevy-wings-icon.png"
)]

pub mod config;
pub mod design;
pub mod file;
pub mod fonts;
pub mod paint;
pub mod scheme;
pub mod theme;
pub mod tokens;

use bevy::prelude::*;
use bevy::window::{PrimaryWindow, Window, WindowTheme};

use bevy_wings_ui_animation::WingsAnimationPlugin;
use bevy_wings_ui_core::{WingsUiSystems, ensure_core};

use theme::{WingsTheme, WingsThemeChanged, WingsThemeMode};

/// Adds theming. Inserts [`WingsTheme`] (the provided one or the default) when missing.
#[derive(Default)]
pub struct WingsThemePlugin {
    /// Initial theme. `None` keeps an existing resource or inserts the default theme.
    pub theme: Option<WingsTheme>,
}

impl WingsThemePlugin {
    /// Plugin with an initial theme.
    pub fn with_theme(theme: WingsTheme) -> Self {
        Self { theme: Some(theme) }
    }

    /// Plugin with the theme described by a `wings.toml` configuration (see
    /// [`wings_config!`](crate::wings_config)).
    pub fn from_config(config: config::WingsConfig) -> Self {
        Self::with_theme(config.theme())
    }
}

impl Plugin for WingsThemePlugin {
    fn build(&self, app: &mut App) {
        ensure_core(app);
        if !app.is_plugin_added::<WingsAnimationPlugin>() {
            app.add_plugins(WingsAnimationPlugin);
        }
        match &self.theme {
            Some(t) => {
                app.insert_resource(t.clone());
            }
            None => {
                app.init_resource::<WingsTheme>();
            }
        }
        app.add_systems(PreStartup, fonts::install_default_font)
            .add_systems(PostUpdate, sync_clear_color);
        app.add_message::<WingsThemeChanged>().add_systems(
            Update,
            (
                follow_system_theme,
                (
                    paint::paint_system,
                    paint::typography_system,
                    paint::shape_system,
                    paint::elevation_system,
                ),
            )
                .chain()
                .in_set(WingsUiSystems::Theme),
        );
    }
}

/// Keeps Bevy's [`ClearColor`] on the theme's background, so areas no UI covers (window edges
/// during resizes, letterboxing) match the app.
pub fn sync_clear_color(theme: Res<WingsTheme>, clear: Option<ResMut<ClearColor>>) {
    if let Some(mut clear) = clear
        && theme.is_changed()
    {
        let background = theme.color(scheme::ColorRole::Background);
        if clear.0 != background {
            clear.0 = background;
        }
    }
}

/// Keeps [`WingsTheme::system_prefers_dark`] in sync with the primary window's theme.
/// Only marks the theme as changed when the preference actually flips.
pub fn follow_system_theme(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut theme: ResMut<WingsTheme>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let prefers_dark = window.window_theme == Some(WindowTheme::Dark);
    if theme.system_prefers_dark != prefers_dark {
        if theme.mode == WingsThemeMode::System {
            theme.system_prefers_dark = prefers_dark;
        } else {
            // Nothing visible changes, so avoid triggering a full repaint.
            theme.bypass_change_detection().system_prefers_dark = prefers_dark;
        }
    }
}

/// Commonly used items.
pub mod prelude {
    pub use crate::WingsThemePlugin;
    pub use crate::config::{WingsAppearance, WingsConfig, WingsCustomUi, WingsUiMode};
    pub use crate::design::WingsDesign;
    pub use crate::file::{WingsThemeFile, WingsThemeFilePlugin, WingsThemeSource};
    pub use crate::paint::{
        WingsBackground, WingsBorderPaint, WingsColorTransition, WingsContentColor,
        WingsElevationLevel, WingsForeground, WingsInheritContentColor, WingsPaint, WingsShape,
        WingsStateLayer, WingsTypeOverride, WingsTypeStyle,
    };
    pub use crate::scheme::{AppleSystemColors, ColorRole, WingsColorScheme, contrast_ratio};
    pub use crate::theme::{WingsLocalTheme, WingsTheme, WingsThemeChanged, WingsThemeMode};
    pub use crate::tokens::{
        ShapeRole, TypeRole, WingsElevation, WingsMotion, WingsShapes, WingsSpacing,
        WingsTextStyle, WingsTypography,
    };
    pub use crate::wings_config;
}

#[cfg(test)]
mod tests {
    use super::prelude::*;
    use bevy::prelude::*;
    use bevy::text::{FontSize, TextColor, TextFont};
    use bevy::time::TimeUpdateStrategy;
    use bevy::ui::{BackgroundColor, BorderRadius, BoxShadow, Val};
    use bevy::window::{PrimaryWindow, WindowTheme};
    use bevy_wings_ui_core::WingsOpacity;
    use core::time::Duration;

    fn app(theme: WingsTheme) -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, WingsThemePlugin::with_theme(theme)));
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            50,
        )));
        app.update();
        app
    }

    fn instant_theme() -> WingsTheme {
        let mut t = WingsTheme::material();
        t.motion.theme_transition = None;
        t
    }

    fn bg(app: &App, e: Entity) -> Color {
        app.world().get::<BackgroundColor>(e).unwrap().0
    }

    /// The headline scenario: twenty surface buttons are light in the light scheme and all
    /// switch to the dark surface color after one theme toggle.
    #[test]
    fn twenty_buttons_switch_to_dark() {
        let mut app = app(instant_theme());
        let buttons: Vec<Entity> = (0..20)
            .map(|_| {
                app.world_mut()
                    .spawn(WingsBackground::role(ColorRole::Surface))
                    .id()
            })
            .collect();
        app.update();
        let light = app
            .world()
            .resource::<WingsTheme>()
            .light
            .get(ColorRole::Surface);
        let dark = app
            .world()
            .resource::<WingsTheme>()
            .dark
            .get(ColorRole::Surface);
        for b in &buttons {
            assert_eq!(bg(&app, *b), light);
        }
        app.world_mut().resource_mut::<WingsTheme>().toggle_dark();
        app.update();
        for b in &buttons {
            assert_eq!(bg(&app, *b), dark);
        }
    }

    /// With a theme transition configured, already painted entities cross fade instead of
    /// jumping, and reach the dark color once the transition completes.
    #[test]
    fn theme_switch_cross_fades() {
        let mut theme = WingsTheme::material();
        theme.motion.theme_transition =
            Some(bevy_wings_ui_animation::prelude::AnimationSpec::millis(200));
        let mut app = app(theme);
        let e = app
            .world_mut()
            .spawn(WingsBackground::role(ColorRole::Surface))
            .id();
        app.update();
        let light = bg(&app, e);
        app.world_mut().resource_mut::<WingsTheme>().toggle_dark();
        app.update();
        app.update();
        let mid = bg(&app, e);
        assert_ne!(mid, light);
        let dark = app
            .world()
            .resource::<WingsTheme>()
            .dark
            .get(ColorRole::Surface);
        assert_ne!(mid, dark);
        for _ in 0..10 {
            app.update();
        }
        let end = bg(&app, e).to_srgba();
        let want = dark.to_srgba();
        assert!((end.red - want.red).abs() < 1e-3 && (end.blue - want.blue).abs() < 1e-3);
    }

    /// A frame without changes does not touch any color component (zero cost idle frames).
    #[test]
    fn idle_frames_do_not_write() {
        let mut app = app(instant_theme());
        let e = app
            .world_mut()
            .spawn(WingsBackground::role(ColorRole::Primary))
            .id();
        app.update();
        let tick = app
            .world()
            .entity(e)
            .get_ref::<BackgroundColor>()
            .unwrap()
            .last_changed();
        app.update();
        app.update();
        let tick2 = app
            .world()
            .entity(e)
            .get_ref::<BackgroundColor>()
            .unwrap()
            .last_changed();
        assert_eq!(tick, tick2);
    }

    /// A local theme keeps its subtree dark while the app is light.
    #[test]
    fn local_theme_overrides_subtree() {
        let mut app = app(instant_theme());
        let root = app.world_mut().spawn(WingsLocalTheme::DARK).id();
        let child = app
            .world_mut()
            .spawn((ChildOf(root), WingsBackground::role(ColorRole::Surface)))
            .id();
        app.update();
        let dark = app
            .world()
            .resource::<WingsTheme>()
            .dark
            .get(ColorRole::Surface);
        assert_eq!(bg(&app, child), dark);
    }

    /// Text inherits the content color of the closest provider and follows its changes.
    #[test]
    fn text_inherits_content_color() {
        let mut app = app(instant_theme());
        let button = app
            .world_mut()
            .spawn(WingsContentColor(WingsPaint::Role(ColorRole::OnPrimary)))
            .id();
        let label = app
            .world_mut()
            .spawn((ChildOf(button), WingsInheritContentColor))
            .id();
        app.update();
        let theme = app.world().resource::<WingsTheme>().clone();
        assert_eq!(
            app.world().get::<TextColor>(label).unwrap().0,
            theme.light.get(ColorRole::OnPrimary)
        );
        app.world_mut()
            .get_mut::<WingsContentColor>(button)
            .unwrap()
            .0 = WingsPaint::Role(ColorRole::Error);
        app.update();
        assert_eq!(
            app.world().get::<TextColor>(label).unwrap().0,
            theme.light.get(ColorRole::Error)
        );
    }

    /// Subtree opacity multiplies painted alpha.
    #[test]
    fn opacity_multiplies_alpha() {
        let mut app = app(instant_theme());
        let root = app.world_mut().spawn(WingsOpacity(0.5)).id();
        let child = app
            .world_mut()
            .spawn((
                ChildOf(root),
                WingsBackground(WingsPaint::Fixed(Color::WHITE)),
            ))
            .id();
        app.update();
        assert!((bg(&app, child).alpha() - 0.5).abs() < 1e-6);
        app.world_mut().get_mut::<WingsOpacity>(root).unwrap().0 = 0.25;
        app.update();
        assert!((bg(&app, child).alpha() - 0.25).abs() < 1e-6);
    }

    /// The state layer tints the fill.
    #[test]
    fn state_layer_tints_fill() {
        let mut app = app(instant_theme());
        let e = app
            .world_mut()
            .spawn((
                WingsBackground(WingsPaint::Fixed(Color::WHITE)),
                WingsStateLayer {
                    color: WingsPaint::Fixed(Color::BLACK),
                    opacity: 0.5,
                },
            ))
            .id();
        app.update();
        let c = bg(&app, e).to_linear();
        assert!((c.red - 0.5).abs() < 1e-4);
    }

    /// Typography, shape and elevation roles are applied from the theme.
    #[test]
    fn tokens_are_applied() {
        let mut app = app(instant_theme());
        let text = app
            .world_mut()
            .spawn(WingsTypeStyle(TypeRole::HeadlineLarge))
            .id();
        let card = app
            .world_mut()
            .spawn((WingsShape(ShapeRole::Large), WingsElevationLevel(3)))
            .id();
        app.update();
        let font = app.world().get::<TextFont>(text).unwrap();
        assert!(matches!(font.font_size, FontSize::Px(s) if s == 32.0));
        assert_eq!(
            app.world().get::<Node>(card).unwrap().border_radius,
            BorderRadius::all(Val::Px(16.0))
        );
        assert_eq!(app.world().get::<BoxShadow>(card).unwrap().0.len(), 1);
        // Changing the theme shapes updates existing nodes.
        app.world_mut().resource_mut::<WingsTheme>().shapes.radii[4] = 20.0;
        app.update();
        assert_eq!(
            app.world().get::<Node>(card).unwrap().border_radius,
            BorderRadius::all(Val::Px(20.0))
        );
    }

    #[derive(Resource, Default)]
    struct Flips(Vec<bool>);

    /// `WingsThemeChanged` is sent on every light or dark flip.
    #[test]
    fn theme_changed_message() {
        let mut app = app(instant_theme());
        app.init_resource::<Flips>();
        app.add_systems(
            Last,
            |mut r: MessageReader<WingsThemeChanged>, mut f: ResMut<Flips>| {
                f.0.extend(r.read().map(|m| m.dark));
            },
        );
        app.world_mut().resource_mut::<WingsTheme>().toggle_dark();
        app.update();
        app.world_mut().resource_mut::<WingsTheme>().toggle_dark();
        app.update();
        assert_eq!(app.world().resource::<Flips>().0, vec![true, false]);
    }

    /// System mode follows the primary window theme.
    #[test]
    fn follows_window_theme() {
        let mut app = app(instant_theme().with_mode(WingsThemeMode::System));
        let window = app
            .world_mut()
            .spawn((
                Window {
                    window_theme: Some(WindowTheme::Dark),
                    ..default()
                },
                PrimaryWindow,
            ))
            .id();
        let e = app
            .world_mut()
            .spawn(WingsBackground::role(ColorRole::Surface))
            .id();
        app.update();
        let theme = app.world().resource::<WingsTheme>().clone();
        assert!(theme.is_dark());
        assert_eq!(bg(&app, e), theme.dark.get(ColorRole::Surface));
        app.world_mut()
            .get_mut::<Window>(window)
            .unwrap()
            .window_theme = Some(WindowTheme::Light);
        app.update();
        assert_eq!(bg(&app, e), theme.light.get(ColorRole::Surface));
    }

    /// Font changes reach existing and newly spawned themed text.
    #[test]
    fn typography_font_updates_existing_text() {
        use bevy::text::FontSource;
        let mut theme = instant_theme();
        theme.typography.font = FontSource::family("First family");
        let mut app = app(theme);
        let text = app
            .world_mut()
            .spawn(WingsTypeStyle(TypeRole::BodyMedium))
            .id();
        app.update();
        assert_eq!(
            app.world().get::<TextFont>(text).unwrap().font,
            FontSource::family("First family")
        );
        app.world_mut().resource_mut::<WingsTheme>().typography.font =
            FontSource::family("Second family");
        app.update();
        assert_eq!(
            app.world().get::<TextFont>(text).unwrap().font,
            FontSource::family("Second family")
        );
        let added = app
            .world_mut()
            .spawn(WingsTypeStyle(TypeRole::LabelLarge))
            .id();
        app.update();
        assert_eq!(
            app.world().get::<TextFont>(added).unwrap().font,
            FontSource::family("Second family")
        );
    }

    /// Installing the embedded font replaces the implicit asset and preserves custom fonts.
    #[test]
    fn embedded_inter_replaces_default_font() {
        use bevy::asset::AssetId;
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Font>();
        let custom = app
            .world_mut()
            .resource_mut::<Assets<Font>>()
            .add(Font::from_bytes(vec![1, 2, 3]));
        app.add_plugins(WingsThemePlugin::default());
        app.update();
        let fonts = app.world().resource::<Assets<Font>>();
        let default = fonts.get(AssetId::default()).unwrap();
        assert_eq!(default.data.as_ref(), crate::fonts::DEFAULT_FONT_DATA);
        assert_eq!(fonts.get(&custom).unwrap().data.as_ref(), &[1, 2, 3]);
    }
}
