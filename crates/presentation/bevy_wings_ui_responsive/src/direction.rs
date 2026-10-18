//! Right to left layout.
//!
//! Set [`WingsLayoutDirection::RightToLeft`] (Arabic, Hebrew, Persian, Urdu) and the whole UI
//! mirrors: rows run from the right, leading padding and margins move to the right, sidebars
//! and back buttons swap sides and directional glyphs such as chevrons point the other way.
//! Text and images are flipped back so they stay readable; mark other content that must not
//! mirror (logos, media, clocks) with [`WingsNoMirror`].
//!
//! Mirroring is a transform on UI roots, so layout code stays in left to right terms and
//! picking keeps working.

use bevy::prelude::*;
use bevy::ui::widget::ImageNode;

/// Reading direction of the UI.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
pub enum WingsLayoutDirection {
    /// Left to right.
    #[default]
    LeftToRight,
    /// Right to left.
    RightToLeft,
}

impl WingsLayoutDirection {
    /// The usual direction of a language tag (`ar`, `he-IL`, `fa`, `ur` are right to left).
    pub fn for_language(tag: &str) -> Self {
        let lang = tag
            .split(['-', '_'])
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        match lang.as_str() {
            "ar" | "he" | "iw" | "fa" | "ur" | "ps" | "sd" | "yi" | "dv" | "ug" | "ckb" => {
                Self::RightToLeft
            }
            _ => Self::LeftToRight,
        }
    }

    /// True for right to left.
    pub fn is_rtl(self) -> bool {
        self == Self::RightToLeft
    }
}

/// Keeps a node (and its subtree) from being flipped back inside a mirrored UI. Use it for
/// content that must keep its own orientation.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct WingsNoMirror;

/// Marks nodes Wings mirrored, so switching back restores them.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct WingsMirrored;

fn flip(transform: &mut UiTransform) {
    transform.scale.x = -transform.scale.x;
}

/// Mirrors UI roots and flips text and images back, following [`WingsLayoutDirection`].
pub fn apply_layout_direction(
    mut commands: Commands,
    direction: Res<WingsLayoutDirection>,
    roots: Query<(Entity, Has<WingsMirrored>), (With<Node>, Without<ChildOf>)>,
    readable: Query<
        (Entity, Has<WingsMirrored>),
        (
            Or<(With<Text>, With<ImageNode>, With<WingsNoMirror>)>,
            With<ChildOf>,
        ),
    >,
    added: Query<
        (),
        Or<(
            Added<Node>,
            Added<Text>,
            Added<ImageNode>,
            Added<WingsNoMirror>,
        )>,
    >,
    mut transforms: Query<&mut UiTransform>,
) {
    if !direction.is_changed() && added.is_empty() {
        return;
    }
    let rtl = direction.is_rtl();
    for (entity, mirrored) in roots.iter().chain(readable.iter()) {
        if mirrored == rtl {
            continue;
        }
        let Ok(mut transform) = transforms.get_mut(entity) else {
            continue;
        };
        flip(&mut transform);
        if rtl {
            commands.entity(entity).insert(WingsMirrored);
        } else {
            commands.entity(entity).remove::<WingsMirrored>();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Languages map to directions.
    #[test]
    fn languages() {
        assert!(WingsLayoutDirection::for_language("ar-EG").is_rtl());
        assert!(WingsLayoutDirection::for_language("he").is_rtl());
        assert!(!WingsLayoutDirection::for_language("en-US").is_rtl());
    }

    /// Roots mirror, text flips back, and everything returns when switching back.
    #[test]
    fn mirrors_and_restores() {
        let mut app = App::new();
        app.init_resource::<WingsLayoutDirection>()
            .add_systems(Update, apply_layout_direction);
        let root = app.world_mut().spawn(Node::default()).id();
        let text = app
            .world_mut()
            .spawn((Node::default(), Text::new("hi"), ChildOf(root)))
            .id();
        let plain = app.world_mut().spawn((Node::default(), ChildOf(root))).id();
        app.update();
        let sx = |app: &App, e| app.world().get::<UiTransform>(e).unwrap().scale.x;
        assert_eq!(sx(&app, root), 1.0);
        *app.world_mut().resource_mut::<WingsLayoutDirection>() = WingsLayoutDirection::RightToLeft;
        app.update();
        assert_eq!(sx(&app, root), -1.0);
        assert_eq!(sx(&app, text), -1.0);
        assert_eq!(sx(&app, plain), 1.0);
        *app.world_mut().resource_mut::<WingsLayoutDirection>() = WingsLayoutDirection::LeftToRight;
        app.update();
        assert_eq!(sx(&app, root), 1.0);
        assert_eq!(sx(&app, text), 1.0);
    }
}
