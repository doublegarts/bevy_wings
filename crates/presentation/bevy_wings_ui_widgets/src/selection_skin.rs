//! Design skins of the selection controls: [`WingsCheckbox`], [`WingsSwitch`],
//! [`WingsRadioButton`], [`WingsSlider`] and [`WingsChip`].
//!
//! | Control | Material | iOS | macOS | Fluent |
//! | --- | --- | --- | --- | --- |
//! | Checkbox | 18 square | 22 round checkmark | 14 square | 20 square |
//! | Switch | 52 x 32 | 61 x 28, green | 54 x 24 | 40 x 20 |
//! | Radio | 20 ring | trailing checkmark row | 14 ring | 20 ring |
//! | Slider thumb | 20 dot | 38 x 24 pill | 20 x 14 pill | 20 ring with accent dot |
//! | Chip | 32, small corners | 32 capsule | 22 capsule | 32 toggle button |

use bevy::prelude::*;

use bevy_wings_ui_theme::prelude::{
    ColorRole, ShapeRole, TypeRole, WingsBackground, WingsBorderPaint, WingsDesign,
    WingsElevationLevel, WingsPaint, WingsShape, WingsTypeStyle,
};

use crate::selection::{
    WingsCheckMarkProgress, WingsCheckbox, WingsCheckboxBox, WingsCheckedPart, WingsCheckedRefresh,
    WingsChip, WingsRadioButton, WingsRadioDot, WingsRadioRing, WingsSlider, WingsSliderFill,
    WingsSliderThumbSize, WingsSliderTrack, WingsSwitch, WingsSwitchThumb, WingsSwitchTrack,
    check_mark_scene,
};
use crate::skin::{WingsSkin, WingsSkinPart, clear_skin_parts, pick, skin_targets};
use bevy::ui_widgets::SliderThumb;

use ColorRole::*;

fn role(r: ColorRole) -> WingsPaint {
    WingsPaint::Role(r)
}

const T: WingsPaint = WingsPaint::Transparent;
const WHITE: WingsPaint = WingsPaint::Fixed(Color::WHITE);

/// Sets the gap of a toggle like control root.
fn toggle_gap(design: WingsDesign) -> f32 {
    pick(design, 12.0, 12.0, 6.0, 8.0)
}

// ---------------------------------------------------------------------------------------------
// Checkbox
// ---------------------------------------------------------------------------------------------

/// Checkbox box and mark per design: `(edge, border, shape, part, mark scale)`.
pub fn checkbox_style(design: WingsDesign) -> (f32, f32, ShapeRole, WingsCheckedPart, f32) {
    let part = |unchecked_fill, unchecked_border| WingsCheckedPart {
        background: Some((role(Primary), unchecked_fill)),
        border: Some((role(Primary), unchecked_border)),
        ..default()
    };
    match design {
        WingsDesign::Material => (
            18.0,
            2.0,
            ShapeRole::ExtraSmall,
            part(T, role(OnSurfaceVariant)),
            1.0,
        ),
        WingsDesign::Ios => (22.0, 1.5, ShapeRole::Full, part(T, role(Outline)), 0.85),
        WingsDesign::MacOs => (
            14.0,
            1.0,
            ShapeRole::ExtraSmall,
            part(role(SurfaceContainerLowest), role(Outline)),
            0.7,
        ),
        WingsDesign::Fluent => (
            20.0,
            1.0,
            ShapeRole::Small,
            part(role(TertiaryFill), role(Outline)),
            0.85,
        ),
    }
}

/// Skins checkboxes.
pub fn skin_checkboxes(
    mut commands: Commands,
    skin: Res<WingsSkin>,
    added: Query<Entity, Added<WingsCheckbox>>,
    all: Query<Entity, With<WingsCheckbox>>,
    children: Query<&Children>,
    mut roots: Query<&mut Node, (With<WingsCheckbox>, Without<WingsCheckboxBox>)>,
    mut boxes: Query<
        (&mut Node, &mut WingsShape, &mut WingsCheckedPart),
        (With<WingsCheckboxBox>, Without<WingsCheckbox>),
    >,
    mut marks: Query<&mut UiTransform, With<WingsCheckMarkProgress>>,
) {
    let (edge, border, shape, part, mark_scale) = checkbox_style(skin.design);
    for entity in skin_targets(&skin, added, all) {
        if let Ok(mut node) = roots.get_mut(entity) {
            node.column_gap = px(toggle_gap(skin.design));
        }
        for d in children.iter_descendants(entity) {
            if let Ok((mut node, mut s, mut p)) = boxes.get_mut(d) {
                node.width = px(edge);
                node.height = px(edge);
                node.border = UiRect::all(px(border));
                s.set_if_neq(WingsShape(shape));
                p.set_if_neq(part);
            } else if let Ok(mut t) = marks.get_mut(d) {
                t.scale = Vec2::splat(mark_scale);
            }
        }
        commands.entity(entity).insert(WingsCheckedRefresh);
    }
}

// ---------------------------------------------------------------------------------------------
// Switch
// ---------------------------------------------------------------------------------------------

/// Switch geometry and colors of a design.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WingsSwitchStyle {
    /// Track size.
    pub track: Vec2,
    /// Track border width.
    pub border: f32,
    /// Track padding.
    pub padding: f32,
    /// Thumb size.
    pub thumb: Vec2,
    /// Thumb scale when off.
    pub off_scale: f32,
    /// Track paints.
    pub track_part: WingsCheckedPart,
    /// Thumb paints (translation is filled in from the sizes).
    pub thumb_part: WingsCheckedPart,
    /// Thumb shadow.
    pub thumb_elevation: u8,
}

impl WingsSwitchStyle {
    /// Style of a design.
    pub fn for_design(design: WingsDesign) -> Self {
        let track = |on, off, border_on, border_off| WingsCheckedPart {
            background: Some((on, off)),
            border: Some((border_on, border_off)),
            ..default()
        };
        let thumb = |on, off| WingsCheckedPart {
            background: Some((on, off)),
            ..default()
        };
        let mut style = match design {
            WingsDesign::Material => Self {
                track: Vec2::new(52.0, 32.0),
                border: 2.0,
                padding: 2.0,
                thumb: Vec2::splat(24.0),
                off_scale: 0.67,
                track_part: track(
                    role(Primary),
                    role(SurfaceContainerHighest),
                    role(Primary),
                    role(Outline),
                ),
                thumb_part: thumb(role(OnPrimary), role(Outline)),
                thumb_elevation: 0,
            },
            WingsDesign::Ios => Self {
                track: Vec2::new(61.0, 28.0),
                border: 0.0,
                padding: 2.0,
                thumb: Vec2::new(37.0, 24.0),
                off_scale: 1.0,
                track_part: track(role(Success), role(Fill), T, T),
                thumb_part: thumb(WHITE, WHITE),
                thumb_elevation: 1,
            },
            WingsDesign::MacOs => Self {
                track: Vec2::new(54.0, 24.0),
                border: 0.0,
                padding: 2.0,
                thumb: Vec2::new(32.0, 20.0),
                off_scale: 1.0,
                track_part: track(role(Primary), role(Fill), T, T),
                thumb_part: thumb(WHITE, WHITE),
                thumb_elevation: 1,
            },
            WingsDesign::Fluent => Self {
                track: Vec2::new(40.0, 20.0),
                border: 1.0,
                padding: 3.0,
                thumb: Vec2::splat(12.0),
                off_scale: 1.0,
                track_part: track(
                    role(Primary),
                    role(TertiaryFill),
                    role(Primary),
                    role(Outline),
                ),
                thumb_part: thumb(role(OnPrimary), role(OnSurfaceVariant)),
                thumb_elevation: 0,
            },
        };
        let travel = style.track.x - 2.0 * (style.border + style.padding) - style.thumb.x;
        style.thumb_part.translate = Some((Vec2::new(travel, 0.0), Vec2::ZERO));
        style.thumb_part.scale = Some((Vec2::ONE, Vec2::splat(style.off_scale)));
        style
    }
}

/// Skins switches.
pub fn skin_switches(
    mut commands: Commands,
    skin: Res<WingsSkin>,
    added: Query<Entity, Added<WingsSwitch>>,
    all: Query<Entity, With<WingsSwitch>>,
    children: Query<&Children>,
    mut roots: Query<
        &mut Node,
        (
            With<WingsSwitch>,
            Without<WingsSwitchTrack>,
            Without<WingsSwitchThumb>,
        ),
    >,
    mut tracks: Query<
        (&mut Node, &mut WingsCheckedPart),
        (
            With<WingsSwitchTrack>,
            Without<WingsSwitchThumb>,
            Without<WingsSwitch>,
        ),
    >,
    mut thumbs: Query<
        (&mut Node, &mut WingsCheckedPart, &mut WingsElevationLevel),
        (
            With<WingsSwitchThumb>,
            Without<WingsSwitchTrack>,
            Without<WingsSwitch>,
        ),
    >,
) {
    let style = WingsSwitchStyle::for_design(skin.design);
    for entity in skin_targets(&skin, added, all) {
        if let Ok(mut node) = roots.get_mut(entity) {
            node.column_gap = px(toggle_gap(skin.design));
        }
        for d in children.iter_descendants(entity) {
            if let Ok((mut node, mut part)) = tracks.get_mut(d) {
                node.width = px(style.track.x);
                node.height = px(style.track.y);
                node.border = UiRect::all(px(style.border));
                node.padding = UiRect::all(px(style.padding));
                part.set_if_neq(style.track_part);
            } else if let Ok((mut node, mut part, mut elevation)) = thumbs.get_mut(d) {
                node.width = px(style.thumb.x);
                node.height = px(style.thumb.y);
                part.set_if_neq(style.thumb_part);
                elevation.set_if_neq(WingsElevationLevel(style.thumb_elevation));
            }
        }
        commands.entity(entity).insert(WingsCheckedRefresh);
    }
}

// ---------------------------------------------------------------------------------------------
// Radio button
// ---------------------------------------------------------------------------------------------

/// Skins radio buttons. iOS has no radio buttons: the option becomes a row with a trailing
/// checkmark, like an inline `Picker`.
pub fn skin_radio_buttons(
    mut commands: Commands,
    skin: Res<WingsSkin>,
    added: Query<Entity, Added<WingsRadioButton>>,
    all: Query<Entity, With<WingsRadioButton>>,
    children: Query<&Children>,
    parts: Query<(), With<WingsSkinPart>>,
    mut roots: Query<
        &mut Node,
        (
            With<WingsRadioButton>,
            Without<WingsRadioRing>,
            Without<WingsRadioDot>,
        ),
    >,
    mut rings: Query<
        (&mut Node, &mut WingsCheckedPart),
        (
            With<WingsRadioRing>,
            Without<WingsRadioDot>,
            Without<WingsRadioButton>,
        ),
    >,
    mut dots: Query<
        (&mut Node, &mut WingsBackground),
        (
            With<WingsRadioDot>,
            Without<WingsRadioRing>,
            Without<WingsRadioButton>,
        ),
    >,
) {
    let (edge, border, dot, ring_part, dot_paint) = match skin.design {
        WingsDesign::Material => (
            20.0,
            2.0,
            10.0,
            WingsCheckedPart {
                border: Some((role(Primary), role(OnSurfaceVariant))),
                background: Some((T, T)),
                ..default()
            },
            role(Primary),
        ),
        WingsDesign::MacOs => (
            14.0,
            1.0,
            6.0,
            WingsCheckedPart {
                border: Some((role(Primary), role(Outline))),
                background: Some((role(Primary), role(SurfaceContainerLowest))),
                ..default()
            },
            WHITE,
        ),
        _ => (
            20.0,
            1.0,
            10.0,
            WingsCheckedPart {
                border: Some((role(Primary), role(Outline))),
                background: Some((role(Primary), role(TertiaryFill))),
                ..default()
            },
            WHITE,
        ),
    };
    for entity in skin_targets(&skin, added, all) {
        clear_skin_parts(&mut commands, entity, &children, &parts);
        if let Ok(mut node) = roots.get_mut(entity) {
            node.column_gap = px(toggle_gap(skin.design));
            if skin.ios() {
                node.justify_content = JustifyContent::SpaceBetween;
                node.min_height = px(44);
            } else {
                node.justify_content = JustifyContent::Start;
                node.min_height = Val::Auto;
            }
        }
        for d in children.iter_descendants(entity) {
            if let Ok((mut node, mut part)) = rings.get_mut(d) {
                node.display = if skin.ios() {
                    Display::None
                } else {
                    Display::Flex
                };
                node.width = px(edge);
                node.height = px(edge);
                node.border = UiRect::all(px(border));
                part.set_if_neq(ring_part);
            } else if let Ok((mut node, mut bg)) = dots.get_mut(d) {
                node.width = px(dot);
                node.height = px(dot);
                bg.set_if_neq(WingsBackground(dot_paint));
            }
        }
        if skin.ios() {
            commands.spawn_scene(bsn! {
                @{check_mark_scene(Primary)}
                WingsSkinPart
                ChildOf(entity)
            });
        }
        commands.entity(entity).insert(WingsCheckedRefresh);
    }
}

// ---------------------------------------------------------------------------------------------
// Slider
// ---------------------------------------------------------------------------------------------

/// Slider geometry of a design: `(height, track thickness, thumb size, thumb fill, thumb
/// border, thumb elevation)`.
pub fn slider_style(design: WingsDesign) -> (f32, f32, Vec2, WingsPaint, WingsPaint, u8) {
    match design {
        WingsDesign::Material => (44.0, 4.0, Vec2::splat(20.0), role(Primary), T, 0),
        WingsDesign::Ios => (44.0, 6.0, Vec2::new(38.0, 24.0), WHITE, T, 2),
        WingsDesign::MacOs => (
            20.0,
            4.0,
            Vec2::new(20.0, 14.0),
            WHITE,
            role(OutlineVariant),
            1,
        ),
        WingsDesign::Fluent => (
            32.0,
            4.0,
            Vec2::splat(20.0),
            role(SurfaceContainerLowest),
            role(OutlineVariant),
            1,
        ),
    }
}

/// Skins sliders.
pub fn skin_sliders(
    mut commands: Commands,
    skin: Res<WingsSkin>,
    added: Query<Entity, Added<WingsSlider>>,
    all: Query<Entity, With<WingsSlider>>,
    children: Query<&Children>,
    parts: Query<(), With<WingsSkinPart>>,
    mut roots: Query<(&mut Node, &mut WingsSliderThumbSize), With<WingsSlider>>,
    mut tracks: Query<
        (&mut Node, &mut WingsBackground),
        (
            With<WingsSliderTrack>,
            Without<WingsSlider>,
            Without<SliderThumb>,
        ),
    >,
    mut thumbs: Query<
        (
            Entity,
            &mut Node,
            &mut WingsBackground,
            &mut WingsBorderPaint,
            &mut WingsElevationLevel,
        ),
        (
            With<SliderThumb>,
            Without<WingsSlider>,
            Without<WingsSliderTrack>,
        ),
    >,
    fills: Query<(), With<WingsSliderFill>>,
) {
    let (height, thickness, thumb, fill, border, elevation) = slider_style(skin.design);
    for entity in skin_targets(&skin, added, all) {
        if let Ok((mut node, mut size)) = roots.get_mut(entity) {
            node.height = px(height);
            size.set_if_neq(WingsSliderThumbSize(thumb));
        }
        for d in children.iter_descendants(entity) {
            if fills.contains(d) {
                continue;
            }
            if let Ok((mut node, mut bg)) = tracks.get_mut(d) {
                node.height = px(thickness);
                node.left = px(thumb.x / 2.0);
                node.right = px(thumb.x / 2.0);
                let track = if skin.design == WingsDesign::Material {
                    SurfaceContainerHighest
                } else {
                    Fill
                };
                bg.set_if_neq(WingsBackground(role(track)));
            } else if let Ok((thumb_entity, mut node, mut bg, mut b, mut e)) = thumbs.get_mut(d) {
                node.width = px(thumb.x);
                node.height = px(thumb.y);
                node.border = UiRect::all(px(if border == T { 0.0 } else { 1.0 }));
                node.justify_content = JustifyContent::Center;
                node.align_items = AlignItems::Center;
                bg.set_if_neq(WingsBackground(fill));
                b.set_if_neq(WingsBorderPaint(border));
                e.set_if_neq(WingsElevationLevel(elevation));
                clear_skin_parts(&mut commands, thumb_entity, &children, &parts);
                if skin.fluent() {
                    // Fluent thumbs carry an accent dot.
                    commands.spawn((
                        Node {
                            width: px(12),
                            height: px(12),
                            ..default()
                        },
                        WingsShape(ShapeRole::Full),
                        WingsBackground(role(Primary)),
                        Pickable::IGNORE,
                        WingsSkinPart,
                        ChildOf(thumb_entity),
                    ));
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Chip
// ---------------------------------------------------------------------------------------------

/// Chip geometry and paints of a design: `(height, padding, shape, part, text role)`.
pub fn chip_style(design: WingsDesign) -> (f32, f32, ShapeRole, WingsCheckedPart, TypeRole) {
    let part = |bg: (WingsPaint, WingsPaint),
                border: (WingsPaint, WingsPaint),
                content: (ColorRole, ColorRole)| {
        WingsCheckedPart {
            background: Some(bg),
            border: Some(border),
            content: Some((role(content.0), role(content.1))),
            ..default()
        }
    };
    match design {
        WingsDesign::Material => (
            32.0,
            16.0,
            ShapeRole::Small,
            part(
                (role(SecondaryContainer), T),
                (T, role(Outline)),
                (OnSecondaryContainer, OnSurfaceVariant),
            ),
            TypeRole::LabelLarge,
        ),
        WingsDesign::Ios => (
            32.0,
            14.0,
            ShapeRole::Full,
            part(
                (role(PrimaryContainer), role(SecondaryFill)),
                (T, T),
                (OnPrimaryContainer, OnSurface),
            ),
            TypeRole::LabelMedium,
        ),
        WingsDesign::MacOs => (
            22.0,
            10.0,
            ShapeRole::Full,
            part(
                (role(PrimaryContainer), role(SecondaryFill)),
                (T, T),
                (OnPrimaryContainer, OnSurface),
            ),
            TypeRole::LabelLarge,
        ),
        WingsDesign::Fluent => (
            32.0,
            12.0,
            ShapeRole::Small,
            part(
                (role(Primary), role(SurfaceContainerLowest)),
                (role(Primary), role(OutlineVariant)),
                (OnPrimary, OnSurface),
            ),
            TypeRole::LabelLarge,
        ),
    }
}

/// Skins chips.
pub fn skin_chips(
    mut commands: Commands,
    skin: Res<WingsSkin>,
    added: Query<Entity, Added<WingsChip>>,
    all: Query<Entity, With<WingsChip>>,
    children: Query<&Children>,
    mut chips: Query<(&mut Node, &mut WingsShape, &mut WingsCheckedPart), With<WingsChip>>,
    mut texts: Query<&mut WingsTypeStyle>,
) {
    let (height, padding, shape, part, text) = chip_style(skin.design);
    for entity in skin_targets(&skin, added, all) {
        if let Ok((mut node, mut s, mut p)) = chips.get_mut(entity) {
            node.height = px(height);
            node.padding = UiRect::horizontal(px(padding));
            s.set_if_neq(WingsShape(shape));
            p.set_if_neq(part);
        }
        for d in children.iter_descendants(entity) {
            if let Ok(mut t) = texts.get_mut(d) {
                t.set_if_neq(WingsTypeStyle(text));
            }
        }
        commands.entity(entity).insert(WingsCheckedRefresh);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Switch thumbs travel to the far end of the track in every design.
    #[test]
    fn switch_travel() {
        for d in WingsDesign::ALL {
            let s = WingsSwitchStyle::for_design(d);
            let (on, _) = s.thumb_part.translate.unwrap();
            let inner = s.track.x - 2.0 * (s.border + s.padding);
            assert!((on.x + s.thumb.x - inner).abs() < 1e-4, "{d:?}");
        }
        assert_eq!(
            WingsSwitchStyle::for_design(WingsDesign::Ios).track,
            Vec2::new(61.0, 28.0)
        );
    }

    /// iOS switches turn green, macOS and Fluent use the accent.
    #[test]
    fn switch_colors() {
        let on = |d| {
            WingsSwitchStyle::for_design(d)
                .track_part
                .background
                .unwrap()
                .0
        };
        assert_eq!(on(WingsDesign::Ios), role(Success));
        assert_eq!(on(WingsDesign::MacOs), role(Primary));
        assert_eq!(on(WingsDesign::Fluent), role(Primary));
    }
}
