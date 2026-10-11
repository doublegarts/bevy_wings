//! Text and icons.

use bevy::prelude::*;
use bevy::ui::widget::ImageNode;

use dga_bevy_wings_ui_theme::prelude::{
    ColorRole, TypeRole, WingsForeground, WingsInheritContentColor, WingsPaint, WingsTypeStyle,
};

/// Themed text.
///
/// ```ignore
/// bsn! { @WingsText { @text: "Hello", @role: TypeRole::HeadlineMedium } }
/// ```
///
/// Without a color the text inherits the content color of the closest container (for example
/// a button label is automatically readable on the button fill).
#[derive(SceneComponent, Default, Clone)]
#[scene(WingsTextProps)]
pub struct WingsText;

/// Props of [`WingsText`].
#[derive(Default)]
pub struct WingsTextProps {
    /// The text.
    pub text: String,
    /// Type scale role.
    pub role: TypeRole,
    /// Explicit color role. `None` inherits the content color.
    pub color: Option<ColorRole>,
}

impl WingsText {
    fn scene(props: WingsTextProps) -> impl Scene {
        let color: Box<dyn Scene> = match props.color {
            Some(role) => Box::new(bsn! { WingsForeground({WingsPaint::Role(role)}) }),
            None => Box::new(bsn! {}),
        };
        bsn! {
            Text({props.text})
            WingsTypeStyle({props.role})
            WingsInheritContentColor
            @{color}
        }
    }
}

/// Builder style shorthand: `wings_text("Hello", TypeRole::BodyLarge)`.
pub fn wings_text(text: impl Into<String>, role: TypeRole) -> impl Scene {
    let text = text.into();
    bsn! { @WingsText { @text: text, @role: role } }
}

/// An image icon tinted with the inherited content color.
///
/// ```ignore
/// bsn! { @WingsIcon { @image: "icons/home.png", @size: 24.0 } }
/// ```
#[derive(SceneComponent, Default, Clone)]
#[scene(WingsIconProps)]
pub struct WingsIcon;

/// Props of [`WingsIcon`].
pub struct WingsIconProps {
    /// Asset path of the icon image.
    pub image: String,
    /// Edge length in pixels.
    pub size: f32,
}

impl Default for WingsIconProps {
    fn default() -> Self {
        Self {
            image: String::new(),
            size: 24.0,
        }
    }
}

/// Copies the resolved content color into the icon image tint.
#[derive(Component, Debug, Default, Clone, Copy)]
#[require(WingsInheritContentColor)]
pub struct WingsIconTint;

impl WingsIcon {
    fn scene(props: WingsIconProps) -> impl Scene {
        let size = props.size;
        bsn! {
            Node { width: px(size), height: px(size) }
            ImageNode { image: {props.image} }
            WingsIconTint
        }
    }
}

/// Keeps icon tints in sync with their (theme resolved) content color.
pub fn icon_tint_system(
    mut query: Query<(&TextColor, &mut ImageNode), (With<WingsIconTint>, Changed<TextColor>)>,
) {
    for (color, mut image) in &mut query {
        image.color = color.0;
    }
}
