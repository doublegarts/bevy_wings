//! Design skins of the feedback widgets: progress indicators and dialogs.
//!
//! | Widget | Material | iOS | macOS | Fluent |
//! | --- | --- | --- | --- | --- |
//! | Linear progress | 4 tall | 4 tall capsule | 6 tall | 3 tall |
//! | Circular progress (indeterminate) | arc | 8 spoke spinner | 12 spoke spinner | arc |
//! | Dialog | basic dialog | alert, capsule buttons side by side | alert, centered | content dialog with a footer |

use bevy::prelude::*;
use bevy::text::TextLayout;

use bevy_wings_ui_animation::prelude::WingsProperty;
#[cfg(feature = "shaders")]
use bevy_wings_ui_animation::prelude::{AnimationSpec, Repeat, WingsAnimateExt, WingsEasing};
use bevy_wings_ui_theme::prelude::{
    ColorRole, ShapeRole, TypeRole, WingsBackground, WingsBorderPaint, WingsDesign,
    WingsElevationLevel, WingsPaint, WingsShape, WingsTypeStyle,
};

use crate::button::{WingsButtonOverride, WingsButtonPalette, WingsButtonVariant};
use crate::feedback::{
    WingsDialog, WingsDialogActions, WingsDialogButton, WingsDialogMessage, WingsDialogSurface,
    WingsDialogTitle, WingsLinearProgress,
};
use crate::skin::{WingsSkin, skin_targets};
#[cfg(feature = "shaders")]
use crate::{
    feedback::WingsProgress,
    skin::{WingsSkinPart, clear_skin_parts, pick},
};

// ---------------------------------------------------------------------------------------------
// Linear progress
// ---------------------------------------------------------------------------------------------

/// Skins linear progress tracks.
pub fn skin_linear_progress(
    skin: Res<WingsSkin>,
    added: Query<Entity, Added<WingsLinearProgress>>,
    all: Query<Entity, With<WingsLinearProgress>>,
    mut tracks: Query<(&mut Node, &mut WingsBackground), With<WingsLinearProgress>>,
) {
    let (height, track) = match skin.design {
        WingsDesign::Material => (4.0, ColorRole::SecondaryContainer),
        WingsDesign::Ios => (4.0, ColorRole::Fill),
        WingsDesign::MacOs => (6.0, ColorRole::Fill),
        WingsDesign::Fluent => (3.0, ColorRole::SecondaryFill),
    };
    for entity in skin_targets(&skin, added, all) {
        if let Ok((mut node, mut bg)) = tracks.get_mut(entity) {
            node.height = px(height);
            bg.set_if_neq(WingsBackground(WingsPaint::Role(track)));
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Spinner
// ---------------------------------------------------------------------------------------------

/// Marks a circular progress indicator drawn as an Apple style spoke spinner instead of the
/// arc shader.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct WingsSpinner;

/// Rotation of a spinner in turns (`0..1`), animated with stepped easing so the spokes jump
/// like the platform activity indicator.
pub struct SpinnerTurnProperty;

impl WingsProperty for SpinnerTurnProperty {
    type Target = UiTransform;
    type Value = f32;
    fn get(t: &UiTransform) -> f32 {
        t.rotation.as_radians() / core::f32::consts::TAU
    }
    fn set(t: &mut UiTransform, v: f32) {
        t.rotation = Rot2::radians(v * core::f32::consts::TAU);
    }
}

/// Spoke geometry: `(center x, center y, width, height, angle, opacity)` for each spoke of a
/// spinner of diameter `size`.
pub fn spinner_spokes(size: f32, count: usize) -> Vec<(f32, f32, f32, f32, f32, f32)> {
    let width = (size * 0.09).max(1.5);
    let height = size * 0.27;
    let radius = size * 0.5 - height * 0.5;
    (0..count)
        .map(|i| {
            let angle = i as f32 / count as f32 * core::f32::consts::TAU;
            let x = size * 0.5 + radius * angle.sin();
            let y = size * 0.5 - radius * angle.cos();
            // The newest spoke is opaque, the trail fades.
            let opacity = 0.25 + 0.75 * (i as f32 / (count - 1).max(1) as f32);
            (x, y, width, height, angle, opacity)
        })
        .collect()
}

/// Skins circular progress indicators: Apple designs show an indeterminate indicator as a
/// spoke spinner, every other case uses the arc shader.
#[cfg(feature = "shaders")]
pub fn skin_circular_progress(
    mut commands: Commands,
    skin: Res<WingsSkin>,
    added: Query<
        Entity,
        (
            With<crate::feedback::WingsCircularProgress>,
            Or<(
                Added<crate::feedback::WingsCircularProgress>,
                Changed<WingsProgress>,
            )>,
        ),
    >,
    all: Query<Entity, With<crate::feedback::WingsCircularProgress>>,
    indicators: Query<(&WingsProgress, &ComputedNode, &Node, Has<WingsSpinner>)>,
    children: Query<&Children>,
    parts: Query<(), With<WingsSkinPart>>,
) {
    use bevy_wings_ui_shaders::WingsCircularProgressMaterial as M;
    let spokes = pick(skin.design, 0, 8, 12, 0);
    for entity in skin_targets(&skin, added, all) {
        let Ok((progress, _, node, is_spinner)) = indicators.get(entity) else {
            continue;
        };
        let want_spinner = spokes > 0 && progress.is_indeterminate();
        if want_spinner == is_spinner && !skin.is_changed() {
            continue;
        }
        clear_skin_parts(&mut commands, entity, &children, &parts);
        if !want_spinner {
            commands
                .entity(entity)
                .remove::<WingsSpinner>()
                .stop_animation::<SpinnerTurnProperty>()
                .insert(UiTransform::default());
            continue;
        }
        let size = match node.width {
            Val::Px(px) => px,
            _ => 32.0,
        };
        commands
            .entity(entity)
            .remove::<MaterialNode<M>>()
            .insert(WingsSpinner)
            .animate_from_to::<SpinnerTurnProperty>(
                0.0,
                1.0,
                AnimationSpec::millis(1000).with_easing(WingsEasing::Steps(spokes as u32)),
                Repeat::infinite(),
            );
        for (x, y, w, h, angle, opacity) in spinner_spokes(size, spokes) {
            commands.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(x - w * 0.5),
                    top: px(y - h * 0.5),
                    width: px(w),
                    height: px(h),
                    ..default()
                },
                UiTransform::from_rotation(Rot2::radians(angle)),
                WingsShape(ShapeRole::Full),
                WingsBackground(WingsPaint::RoleAlpha(ColorRole::OnSurfaceVariant, opacity)),
                Pickable::IGNORE,
                WingsSkinPart,
                ChildOf(entity),
            ));
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Dialog
// ---------------------------------------------------------------------------------------------

/// Dialog layout of a design.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WingsDialogStyle {
    /// Minimum and maximum surface width.
    pub width: (f32, f32),
    /// Surface padding.
    pub padding: f32,
    /// Gap between title, message and buttons.
    pub gap: f32,
    /// Surface shape.
    pub shape: ShapeRole,
    /// Surface fill.
    pub surface: ColorRole,
    /// Title role.
    pub title: TypeRole,
    /// Message role.
    pub message: TypeRole,
    /// Center the texts.
    pub centered: bool,
    /// Buttons share the width.
    pub grow_buttons: bool,
    /// Scrim opacity.
    pub scrim: f32,
}

impl WingsDialogStyle {
    /// Style of a design.
    pub fn for_design(design: WingsDesign) -> Self {
        match design {
            WingsDesign::Material => Self {
                width: (280.0, 560.0),
                padding: 24.0,
                gap: 16.0,
                shape: ShapeRole::ExtraLarge,
                surface: ColorRole::SurfaceContainerHigh,
                title: TypeRole::HeadlineSmall,
                message: TypeRole::BodyMedium,
                centered: false,
                grow_buttons: false,
                scrim: 0.32,
            },
            WingsDesign::Ios => Self {
                width: (270.0, 318.0),
                padding: 18.0,
                gap: 8.0,
                shape: ShapeRole::ExtraLarge,
                surface: ColorRole::SurfaceContainerLowest,
                title: TypeRole::TitleMedium,
                message: TypeRole::LabelMedium,
                centered: false,
                grow_buttons: true,
                scrim: 0.2,
            },
            WingsDesign::MacOs => Self {
                width: (260.0, 260.0),
                padding: 20.0,
                gap: 8.0,
                shape: ShapeRole::Large,
                surface: ColorRole::SurfaceContainerLowest,
                title: TypeRole::TitleLarge,
                message: TypeRole::LabelMedium,
                centered: true,
                grow_buttons: true,
                scrim: 0.0,
            },
            WingsDesign::Fluent => Self {
                width: (320.0, 548.0),
                padding: 24.0,
                gap: 12.0,
                shape: ShapeRole::Large,
                surface: ColorRole::SurfaceContainerLowest,
                title: TypeRole::HeadlineMedium,
                message: TypeRole::BodyMedium,
                centered: false,
                grow_buttons: true,
                scrim: 0.3,
            },
        }
    }

    /// The action button override for a design.
    pub fn button(design: WingsDesign, button: WingsDialogButton) -> WingsButtonOverride {
        use ColorRole::*;
        let palette = |container, content, border| {
            Some(WingsButtonPalette {
                container,
                content,
                border,
                elevation: 0,
            })
        };
        let t = WingsPaint::Transparent;
        let confirm_fill = if button.destructive { Error } else { Primary };
        let confirm_on = if button.destructive {
            OnError
        } else {
            OnPrimary
        };
        match design {
            WingsDesign::Material => WingsButtonOverride {
                palette: palette(
                    t,
                    if button.destructive && button.confirm {
                        Error
                    } else {
                        Primary
                    },
                    t,
                ),
                min_height: None,
                grow: false,
            },
            WingsDesign::Ios => WingsButtonOverride {
                palette: palette(
                    WingsPaint::Role(SecondaryFill),
                    match (button.confirm, button.destructive) {
                        (false, _) => OnSurface,
                        (true, true) => Error,
                        (true, false) => Primary,
                    },
                    t,
                ),
                min_height: Some(48.0),
                grow: true,
            },
            WingsDesign::MacOs => WingsButtonOverride {
                palette: if button.confirm {
                    palette(WingsPaint::Role(confirm_fill), confirm_on, t)
                } else {
                    palette(WingsPaint::Role(SecondaryFill), OnSurface, t)
                },
                min_height: Some(28.0),
                grow: true,
            },
            WingsDesign::Fluent => WingsButtonOverride {
                palette: if button.confirm {
                    palette(WingsPaint::Role(confirm_fill), confirm_on, t)
                } else {
                    palette(
                        WingsPaint::Role(SurfaceContainerLowest),
                        OnSurface,
                        WingsPaint::Role(OutlineVariant),
                    )
                },
                min_height: Some(32.0),
                grow: true,
            },
        }
    }
}

type DialogParts<'a> = (
    Option<&'a mut WingsTypeStyle>,
    Option<&'a mut TextLayout>,
    Has<WingsDialogTitle>,
    Has<WingsDialogMessage>,
);

/// Skins dialogs. Runs before the button skin so the action buttons pick up their overrides
/// in the same frame.
pub fn skin_dialogs(
    mut commands: Commands,
    skin: Res<WingsSkin>,
    added: Query<Entity, Added<WingsDialog>>,
    all: Query<Entity, With<WingsDialog>>,
    children: Query<&Children>,
    mut scrims: Query<&mut WingsBackground, (With<WingsDialog>, Without<WingsDialogSurface>)>,
    mut surfaces: Query<
        (
            &mut Node,
            &mut WingsShape,
            &mut WingsBackground,
            &mut WingsBorderPaint,
            &mut WingsElevationLevel,
        ),
        (With<WingsDialogSurface>, Without<WingsDialog>),
    >,
    mut texts: Query<DialogParts, Or<(With<WingsDialogTitle>, With<WingsDialogMessage>)>>,
    mut rows: Query<
        &mut Node,
        (
            With<WingsDialogActions>,
            Without<WingsDialogSurface>,
            Without<WingsDialog>,
        ),
    >,
    buttons: Query<&WingsDialogButton>,
    mut variants: Query<&mut WingsButtonVariant>,
) {
    let style = WingsDialogStyle::for_design(skin.design);
    for entity in skin_targets(&skin, added, all) {
        if let Ok(mut scrim) = scrims.get_mut(entity) {
            scrim.set_if_neq(WingsBackground(
                WingsPaint::Role(ColorRole::Scrim).with_alpha(style.scrim),
            ));
        }
        for d in children.iter_descendants(entity) {
            if let Ok((mut node, mut shape, mut bg, mut border, mut elevation)) =
                surfaces.get_mut(d)
            {
                node.min_width = px(style.width.0);
                node.max_width = px(style.width.1);
                node.width = if style.width.0 == style.width.1 {
                    px(style.width.0)
                } else {
                    Val::Auto
                };
                node.padding = UiRect::all(px(style.padding));
                node.row_gap = px(style.gap);
                node.border = UiRect::all(px(if skin.design == WingsDesign::Material {
                    0.0
                } else {
                    1.0
                }));
                node.align_items = if style.centered {
                    AlignItems::Center
                } else {
                    AlignItems::Stretch
                };
                shape.set_if_neq(WingsShape(style.shape));
                bg.set_if_neq(WingsBackground(WingsPaint::Role(style.surface)));
                border.set_if_neq(WingsBorderPaint(if skin.design == WingsDesign::Material {
                    WingsPaint::Transparent
                } else {
                    WingsPaint::Role(ColorRole::OutlineVariant)
                }));
                elevation.set_if_neq(WingsElevationLevel(
                    if skin.design == WingsDesign::Material {
                        3
                    } else {
                        4
                    },
                ));
            } else if let Ok((role, layout, is_title, is_message)) = texts.get_mut(d) {
                debug_assert!(is_title || is_message);
                if let Some(mut role) = role {
                    role.set_if_neq(WingsTypeStyle(if is_title {
                        style.title
                    } else {
                        style.message
                    }));
                }
                if let Some(mut layout) = layout {
                    layout.justify = if style.centered {
                        bevy::text::Justify::Center
                    } else {
                        bevy::text::Justify::Left
                    };
                }
            } else if let Ok(mut row) = rows.get_mut(d) {
                row.justify_content = if style.grow_buttons {
                    JustifyContent::Stretch
                } else {
                    JustifyContent::End
                };
                row.margin.top = px(if skin.design == WingsDesign::Material {
                    0.0
                } else {
                    8.0
                });
                row.width = if style.grow_buttons {
                    percent(100)
                } else {
                    Val::Auto
                };
            } else if let Ok(button) = buttons.get(d) {
                commands
                    .entity(d)
                    .insert(WingsDialogStyle::button(skin.design, *button));
                if let Ok(mut variant) = variants.get_mut(d) {
                    // Re-run the button skin for this button.
                    variant.set_changed();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Spokes sit on a circle inside the indicator and fade towards the oldest.
    #[test]
    fn spokes_fit() {
        let spokes = spinner_spokes(20.0, 8);
        assert_eq!(spokes.len(), 8);
        for (x, y, w, h, _, o) in &spokes {
            assert!(*x >= 0.0 && *x <= 20.0 && *y >= 0.0 && *y <= 20.0);
            assert!(*w > 0.0 && *h > 0.0 && *o > 0.0 && *o <= 1.0);
        }
        assert!(spokes[7].5 > spokes[0].5);
    }

    /// iOS alerts put the destructive action in red on a gray capsule.
    #[test]
    fn ios_alert_buttons() {
        let b = WingsDialogStyle::button(
            WingsDesign::Ios,
            WingsDialogButton {
                confirm: true,
                destructive: true,
            },
        );
        assert_eq!(b.palette.unwrap().content, ColorRole::Error);
        assert!(b.grow);
        assert_eq!(
            WingsDialogStyle::for_design(WingsDesign::Ios).width.1,
            318.0
        );
    }
}
