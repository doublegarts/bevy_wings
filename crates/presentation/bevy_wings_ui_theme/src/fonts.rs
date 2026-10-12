//! The bundled application font. Inter is distributed under the SIL Open Font License.

use bevy::asset::{AssetId, Assets};
use bevy::prelude::{Font, ResMut};

/// Inter's variable weight and optical-size font, embedded without an asset-directory dependency.
pub const DEFAULT_FONT_DATA: &[u8] = include_bytes!("fonts/Inter-Variable.ttf");

/// Replaces Bevy's implicit font before any UI is laid out. Apps using explicit font handles
/// retain their font assets. Headless apps without font assets need no special setup.
pub(crate) fn install_default_font(fonts: Option<ResMut<Assets<Font>>>) {
    if let Some(mut fonts) = fonts {
        fonts
            .insert(
                AssetId::default(),
                Font::from_bytes(DEFAULT_FONT_DATA.to_vec()),
            )
            .expect("the default font uses a stable asset ID");
    }
}
