//! Touch ripple rendered with the custom `WingsRippleMaterial` UI shader.
//!
//! Every [`WingsInteractive`](crate::interaction::WingsInteractive) widget gets
//! [`WingsRipple`] automatically when the `shaders` feature is enabled. A press spawns a
//! short lived child node that expands a circle from the pointer position, clipped to the
//! widget's rounded corners, then fades out.
//!
//! Ripples are a Material effect: the iOS, macOS and Fluent skins give feedback without them.

use bevy::picking::events::PointerPress;
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;

use bevy_wings_ui_shaders::WingsRippleMaterial;
use bevy_wings_ui_theme::prelude::{WingsStateLayer, WingsTheme};

/// Enables the press ripple on a widget.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct WingsRipple {
    /// Expansion time in seconds.
    pub duration: f32,
    /// Peak opacity of the ripple color.
    pub opacity: f32,
}

impl Default for WingsRipple {
    fn default() -> Self {
        Self {
            duration: 0.45,
            opacity: 0.16,
        }
    }
}

/// A running ripple node.
#[derive(Component, Debug, Clone, Copy)]
pub struct WingsRippleWave {
    /// Seconds since the press.
    pub elapsed: f32,
    /// Expansion time in seconds.
    pub duration: f32,
}

impl WingsRippleWave {
    /// Fraction of the expansion, `0..=1`.
    pub fn progress(&self) -> f32 {
        (self.elapsed / self.duration).clamp(0.0, 1.0)
    }

    /// Fade multiplier: fully opaque while expanding, then fading over half the duration.
    pub fn alpha(&self) -> f32 {
        let fade = (self.elapsed - self.duration) / (self.duration * 0.5);
        1.0 - fade.clamp(0.0, 1.0)
    }

    /// True once the ripple fully faded.
    pub fn is_done(&self) -> bool {
        self.elapsed >= self.duration * 1.5
    }
}

/// Observer: spawns a ripple under the pointer on press.
pub fn spawn_ripple(
    press: On<PointerPress>,
    mut commands: Commands,
    theme: Res<WingsTheme>,
    skin: Res<crate::skin::WingsSkin>,
    materials: Option<ResMut<Assets<WingsRippleMaterial>>>,
    widgets: Query<(&WingsRipple, &Node, Option<&WingsStateLayer>), Without<InteractionDisabled>>,
) {
    let Some(mut materials) = materials else {
        return;
    };
    if skin.design != bevy_wings_ui_theme::prelude::WingsDesign::Material {
        return;
    }
    let target = press.event_target();
    let Ok((ripple, node, layer)) = widgets.get(target) else {
        return;
    };
    let hit = press
        .hit
        .position
        .map(|p| p.truncate())
        .unwrap_or(Vec2::ZERO);
    let color = layer
        .map(|l| l.color.resolve(theme.colors(), theme.is_dark()))
        .unwrap_or(Color::WHITE)
        .with_alpha(ripple.opacity);
    let material = materials.add(WingsRippleMaterial::new(
        color,
        WingsRippleMaterial::center_from_hit(hit),
    ));
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: px(0),
            right: px(0),
            top: px(0),
            bottom: px(0),
            border_radius: node.border_radius,
            ..default()
        },
        MaterialNode(material),
        Pickable::IGNORE,
        WingsRippleWave {
            elapsed: 0.0,
            duration: ripple.duration,
        },
        ChildOf(target),
    ));
}

/// Advances ripples and despawns finished ones.
pub fn tick_ripples(
    mut commands: Commands,
    time: Res<Time>,
    materials: Option<ResMut<Assets<WingsRippleMaterial>>>,
    mut waves: Query<(
        Entity,
        &mut WingsRippleWave,
        &MaterialNode<WingsRippleMaterial>,
    )>,
) {
    let Some(mut materials) = materials else {
        return;
    };
    let dt = time.delta_secs();
    for (entity, mut wave, handle) in &mut waves {
        wave.elapsed += dt;
        if wave.is_done() {
            commands.entity(entity).despawn();
            continue;
        }
        if let Some(mut m) = materials.get_mut(&handle.0) {
            m.progress = wave.progress();
            m.alpha = wave.alpha();
        }
    }
}

/// Adds [`WingsRipple`] to every new interactive widget that does not opt out.
pub fn add_default_ripples(
    mut commands: Commands,
    added: Query<
        Entity,
        (
            Added<crate::interaction::WingsInteractive>,
            Without<WingsRipple>,
            Without<WingsNoRipple>,
        ),
    >,
) {
    for entity in &added {
        commands.entity(entity).insert(WingsRipple::default());
    }
}

/// Opts a widget out of the automatic ripple.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct WingsNoRipple;

#[cfg(test)]
mod tests {
    use super::*;

    /// A wave expands, holds full opacity, then fades and finishes.
    #[test]
    fn wave_lifecycle() {
        let mut w = WingsRippleWave {
            elapsed: 0.0,
            duration: 0.4,
        };
        assert_eq!(w.progress(), 0.0);
        assert_eq!(w.alpha(), 1.0);
        w.elapsed = 0.4;
        assert_eq!(w.progress(), 1.0);
        assert_eq!(w.alpha(), 1.0);
        w.elapsed = 0.5;
        assert!(w.alpha() < 1.0 && w.alpha() > 0.0);
        w.elapsed = 0.6;
        assert!(w.is_done());
    }
}
