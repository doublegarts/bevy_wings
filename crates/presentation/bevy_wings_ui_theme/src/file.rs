//! Theme files with hot reload.
//!
//! A theme file has the same format as `wings.toml` (see [`config`](crate::config)) and the
//! extension `.wings.toml`. Load it as an asset and the theme follows the file: edit and save
//! it while the app runs and every widget repaints, no rebuild needed.
//!
//! ```ignore
//! // assets/ui/brand.wings.toml
//! app.add_plugins(WingsThemeFilePlugin::new("ui/brand.wings.toml"));
//! ```
//!
//! Hot reload needs Bevy's `file_watcher` feature (on by default in `bevy/dev`). Without it the
//! file still loads once at startup. A file with an error keeps the current theme and logs
//! the problem.

use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, LoadContext};
use bevy::prelude::*;

use crate::config::{WingsConfig, WingsConfigError};
use crate::theme::WingsTheme;

/// A loaded `.wings.toml` theme file.
#[derive(Asset, TypePath, Debug, Clone, PartialEq)]
pub struct WingsThemeFile(pub WingsConfig);

/// Why a theme file failed to load.
#[derive(Debug)]
pub enum WingsThemeFileError {
    /// Reading the file failed.
    Io(std::io::Error),
    /// The file is not valid UTF-8.
    Utf8(std::string::FromUtf8Error),
    /// The file is not a valid theme.
    Config(WingsConfigError),
}

impl core::fmt::Display for WingsThemeFileError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "could not read theme file: {e}"),
            Self::Utf8(e) => write!(f, "theme file is not UTF-8: {e}"),
            Self::Config(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for WingsThemeFileError {}

/// Loads `.wings.toml` files.
#[derive(Default, TypePath)]
pub struct WingsThemeFileLoader;

impl AssetLoader for WingsThemeFileLoader {
    type Asset = WingsThemeFile;
    type Settings = ();
    type Error = WingsThemeFileError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        _load_context: &mut LoadContext<'_>,
    ) -> Result<WingsThemeFile, WingsThemeFileError> {
        let mut bytes = Vec::new();
        reader
            .read_to_end(&mut bytes)
            .await
            .map_err(WingsThemeFileError::Io)?;
        let text = String::from_utf8(bytes).map_err(WingsThemeFileError::Utf8)?;
        WingsConfig::from_toml(&text)
            .map(WingsThemeFile)
            .map_err(WingsThemeFileError::Config)
    }

    fn extensions(&self) -> &[&str] {
        &["wings.toml"]
    }
}

/// The theme file the app's theme follows.
#[derive(Resource, Debug, Clone)]
pub struct WingsThemeSource {
    /// The file.
    pub handle: Handle<WingsThemeFile>,
    applied: Option<WingsConfig>,
}

impl WingsThemeSource {
    /// Follows a theme file asset.
    pub fn new(handle: Handle<WingsThemeFile>) -> Self {
        Self {
            handle,
            applied: None,
        }
    }
}

/// Loads a theme file at startup and keeps the theme in sync with it.
pub struct WingsThemeFilePlugin {
    /// Asset path of the `.wings.toml` file.
    pub path: String,
}

impl WingsThemeFilePlugin {
    /// Follows the theme file at an asset path.
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }
}

impl Plugin for WingsThemeFilePlugin {
    fn build(&self, app: &mut App) {
        if !app.world().contains_resource::<Assets<WingsThemeFile>>() {
            app.init_asset::<WingsThemeFile>()
                .register_asset_loader(WingsThemeFileLoader);
        }
        let path = self.path.clone();
        app.add_systems(
            PreStartup,
            move |mut commands: Commands, server: Res<AssetServer>| {
                commands.insert_resource(WingsThemeSource::new(server.load(path.clone())));
            },
        )
        .add_systems(PreUpdate, apply_theme_file);
    }
}

/// Rebuilds the theme from a theme file. The running appearance (light, dark, system) is
/// kept unless the file changed it, so toggling dark mode survives edits to colors.
pub fn theme_from_file(
    current: &WingsTheme,
    previous: Option<&WingsConfig>,
    file: &WingsConfig,
) -> WingsTheme {
    let mut theme = file.theme();
    if previous.is_some_and(|p| p.appearance == file.appearance) {
        theme.mode = current.mode;
    }
    theme
}

/// Applies the theme file when it loads or changes.
pub fn apply_theme_file(
    mut events: MessageReader<AssetEvent<WingsThemeFile>>,
    files: Res<Assets<WingsThemeFile>>,
    source: Option<ResMut<WingsThemeSource>>,
    theme: Option<ResMut<WingsTheme>>,
) {
    let (Some(mut source), Some(mut theme)) = (source, theme) else {
        events.clear();
        return;
    };
    let id = source.handle.id();
    let changed = events.read().any(|e| match e {
        AssetEvent::Added { id: i }
        | AssetEvent::Modified { id: i }
        | AssetEvent::LoadedWithDependencies { id: i } => *i == id,
        _ => false,
    });
    if !changed {
        return;
    }
    let Some(file) = files.get(id) else { return };
    if source.applied.as_ref() == Some(&file.0) {
        return;
    }
    *theme = theme_from_file(&theme, source.applied.as_ref(), &file.0);
    source.applied = Some(file.0.clone());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheme::ColorRole;
    use crate::theme::WingsThemeMode;

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<WingsThemeFile>()
            .add_systems(PreUpdate, apply_theme_file)
            .insert_resource(WingsTheme::default());
        app
    }

    fn file(text: &str) -> WingsThemeFile {
        WingsThemeFile(WingsConfig::from_toml(text).unwrap())
    }

    /// The theme follows the file when it is added and every time it changes.
    #[test]
    fn theme_follows_file() {
        let mut app = app();
        let handle = app
            .world_mut()
            .resource_mut::<Assets<WingsThemeFile>>()
            .add(file(
                "ui = \"custom\"\nappearance = \"light\"\n[custom]\ndesign = \"material\"\n[custom.light]\nprimary = \"#FF0000\"\n",
            ));
        app.insert_resource(WingsThemeSource::new(handle.clone()));
        // Asset events go out at the end of a frame and are read in the next one.
        app.update();
        app.update();
        let primary = app
            .world()
            .resource::<WingsTheme>()
            .light
            .get(ColorRole::Primary);
        assert_eq!(primary, Color::srgb(1.0, 0.0, 0.0));

        // The app switched to dark at runtime; editing a color keeps it dark.
        app.world_mut().resource_mut::<WingsTheme>().mode = WingsThemeMode::Dark;
        let _ = app
            .world_mut()
            .resource_mut::<Assets<WingsThemeFile>>()
            .insert(handle.id(), file(
                "ui = \"custom\"\nappearance = \"light\"\n[custom]\ndesign = \"material\"\n[custom.light]\nprimary = \"#00FF00\"\n",
            ));
        app.update();
        app.update();
        let theme = app.world().resource::<WingsTheme>();
        assert_eq!(
            theme.light.get(ColorRole::Primary),
            Color::srgb(0.0, 1.0, 0.0)
        );
        assert_eq!(theme.mode, WingsThemeMode::Dark);
    }

    /// Changing the appearance in the file wins over the running mode.
    #[test]
    fn appearance_change_applies() {
        let current = WingsTheme::default();
        let before = WingsConfig::from_toml("appearance = \"light\"").unwrap();
        let after = WingsConfig::from_toml("appearance = \"dark\"").unwrap();
        assert_eq!(
            theme_from_file(&current, Some(&before), &after).mode,
            WingsThemeMode::Dark
        );
    }
}
