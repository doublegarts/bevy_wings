//! Segmented controls and tabs: [`WingsSegmented`] and [`WingsTabs`].
//!
//! Both show mutually exclusive options in a row and trigger [`WingsSegmentChanged`] when the
//! selection changes. The selection lives in [`WingsSelectedIndex`] on the control, so a view
//! model can also drive it.
//!
//! | Design | Segmented | Tabs |
//! | --- | --- | --- |
//! | Material | outlined segmented buttons | primary tabs with an underline indicator |
//! | iOS | gray capsule track, white selected capsule (`Picker(.segmented)`) | same as segmented |
//! | macOS | gray track, accent filled selection | same as segmented |
//! | Fluent | bordered group, subtle selection with an accent bar | pivot tabs with an accent bar |
//!
//! ```ignore
//! bsn! {
//!     @WingsSegmented { @options: vec!["Day".into(), "Week".into(), "Month".into()] }
//!     on(|ev: On<WingsSegmentChanged>| info!("now {}", ev.index))
//! }
//! ```

use bevy::input_focus::tab_navigation::TabIndex;
use bevy::picking::cursor::EntityCursor;
use bevy::prelude::*;
use bevy::ui::Checked;
use bevy::ui_widgets::{Activate, Button};
use bevy::window::SystemCursorIcon;

use bevy_wings_ui_theme::prelude::{
    ColorRole, ShapeRole, TypeRole, WingsBackground, WingsBorderPaint, WingsContentColor,
    WingsDesign, WingsElevationLevel, WingsPaint, WingsShape, WingsTypeStyle,
};

use crate::interaction::WingsInteractive;
use crate::selection::{WingsCheckedOwner, WingsCheckedPart, WingsCheckedRefresh};
use crate::skin::{WingsSkin, WingsSkinPart, clear_skin_parts, skin_targets};
use crate::text::WingsText;

/// The selected option of a [`WingsSegmented`] or [`WingsTabs`].
#[derive(Component, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct WingsSelectedIndex(pub usize);

/// One option of a segmented control or tab row.
#[derive(Component, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct WingsSegment {
    /// Position among the options.
    pub index: usize,
}

/// Triggered on a [`WingsSegmented`] or [`WingsTabs`] when the user picks another option.
#[derive(EntityEvent, Debug, Clone, Copy)]
pub struct WingsSegmentChanged {
    /// The control.
    pub entity: Entity,
    /// The selected option.
    pub index: usize,
}

/// Visual kind of a segmented row.
#[derive(Component, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum WingsSegmentedKind {
    /// Segmented control.
    #[default]
    Segmented,
    /// Tabs.
    Tabs,
}

/// A segmented control.
#[derive(SceneComponent, Default, Clone)]
#[scene(WingsSegmentedProps)]
pub struct WingsSegmented;

/// Tabs above content.
#[derive(SceneComponent, Default, Clone)]
#[scene(WingsSegmentedProps)]
pub struct WingsTabs;

/// Props of [`WingsSegmented`] and [`WingsTabs`].
#[derive(Clone)]
pub struct WingsSegmentedProps {
    /// Option labels.
    pub options: Vec<String>,
    /// Initially selected option.
    pub selected: usize,
    /// Width of the control.
    pub width: Val,
}

impl Default for WingsSegmentedProps {
    fn default() -> Self {
        Self {
            options: Vec::new(),
            selected: 0,
            width: Val::Auto,
        }
    }
}

fn segments_scene(props: &WingsSegmentedProps) -> Vec<impl Scene + use<>> {
    props
        .options
        .iter()
        .enumerate()
        .map(|(index, label)| {
            let label = label.clone();
            let checked: Box<dyn Scene> = if index == props.selected {
                Box::new(bsn! { Checked })
            } else {
                Box::new(bsn! {})
            };
            let segment = WingsSegment { index };
            bsn! {
                Node {
                    flex_grow: 1.0,
                    flex_basis: px(0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    padding: UiRect::horizontal(px(12)),
                }
                segment
                Button
                WingsInteractive
                WingsCheckedOwner
                WingsCheckedPart
                TabIndex(0)
                EntityCursor::System(SystemCursorIcon::Pointer)
                WingsBackground
                WingsBorderPaint
                WingsContentColor
                WingsShape(ShapeRole::Full)
                WingsElevationLevel
                on(segment_activated)
                @{checked}
                Children [ @WingsText { @text: label, @role: TypeRole::LabelLarge } ]
            }
        })
        .collect()
}

fn root_scene(props: WingsSegmentedProps, kind: WingsSegmentedKind) -> impl Scene {
    let width = props.width;
    let selected = WingsSelectedIndex(props.selected);
    let segments = segments_scene(&props);
    bsn! {
        Node { width: width, align_items: AlignItems::Stretch, flex_shrink: 0.0 }
        kind
        selected
        WingsBackground
        WingsBorderPaint
        WingsShape(ShapeRole::Full)
        Children [ {segments} ]
    }
}

impl WingsSegmented {
    fn scene(props: WingsSegmentedProps) -> impl Scene {
        root_scene(props, WingsSegmentedKind::Segmented)
    }
}

impl WingsTabs {
    fn scene(props: WingsSegmentedProps) -> impl Scene {
        root_scene(props, WingsSegmentedKind::Tabs)
    }
}

/// Observer: selects the activated segment.
pub fn segment_activated(
    ev: On<Activate>,
    segments: Query<(&WingsSegment, &ChildOf)>,
    mut controls: Query<&mut WingsSelectedIndex>,
    mut commands: Commands,
) {
    let Ok((segment, parent)) = segments.get(ev.event_target()) else {
        return;
    };
    let control = parent.parent();
    if let Ok(mut selected) = controls.get_mut(control)
        && selected.0 != segment.index
    {
        selected.0 = segment.index;
        commands.trigger(WingsSegmentChanged {
            entity: control,
            index: segment.index,
        });
    }
}

/// Moves [`Checked`] to the selected segment when the selection changes.
pub fn segment_selection_system(
    mut commands: Commands,
    controls: Query<(&WingsSelectedIndex, &Children), Changed<WingsSelectedIndex>>,
    segments: Query<(&WingsSegment, Has<Checked>)>,
) {
    for (selected, children) in &controls {
        for child in children.iter() {
            let Ok((segment, checked)) = segments.get(child) else {
                continue;
            };
            match (segment.index == selected.0, checked) {
                (true, false) => {
                    commands.entity(child).insert(Checked);
                }
                (false, true) => {
                    commands.entity(child).remove::<Checked>();
                }
                _ => {}
            }
        }
    }
}

/// Look of a segmented row in a design.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WingsSegmentedStyle {
    /// Height.
    pub height: f32,
    /// Track padding.
    pub padding: f32,
    /// Track fill.
    pub track: WingsPaint,
    /// Track border.
    pub track_border: WingsPaint,
    /// Track corners.
    pub track_shape: ShapeRole,
    /// Segment corners.
    pub segment_shape: ShapeRole,
    /// Segment paints.
    pub segment: WingsCheckedPart,
    /// Selected segment shadow.
    pub selected_elevation: u8,
    /// Show an accent bar under the selected segment.
    pub indicator: bool,
    /// Label role.
    pub label: TypeRole,
}

impl WingsSegmentedStyle {
    /// Style of a design and kind.
    pub fn for_design(design: WingsDesign, kind: WingsSegmentedKind) -> Self {
        use ColorRole::*;
        let t = WingsPaint::Transparent;
        let r = WingsPaint::Role;
        let part =
            |bg: (WingsPaint, WingsPaint), content: (ColorRole, ColorRole)| WingsCheckedPart {
                background: Some(bg),
                content: Some((r(content.0), r(content.1))),
                ..default()
            };
        match (design, kind) {
            (WingsDesign::Material, WingsSegmentedKind::Segmented) => Self {
                height: 40.0,
                padding: 0.0,
                track: t,
                track_border: r(Outline),
                track_shape: ShapeRole::Full,
                segment_shape: ShapeRole::None,
                segment: part(
                    (r(SecondaryContainer), t),
                    (OnSecondaryContainer, OnSurface),
                ),
                selected_elevation: 0,
                indicator: false,
                label: TypeRole::LabelLarge,
            },
            (WingsDesign::Material, WingsSegmentedKind::Tabs) => Self {
                height: 48.0,
                padding: 0.0,
                track: t,
                track_border: t,
                track_shape: ShapeRole::None,
                segment_shape: ShapeRole::None,
                segment: part((t, t), (Primary, OnSurfaceVariant)),
                selected_elevation: 0,
                indicator: true,
                label: TypeRole::TitleSmall,
            },
            (WingsDesign::Ios, _) => Self {
                height: 32.0,
                padding: 2.0,
                track: r(TertiaryFill),
                track_border: t,
                track_shape: ShapeRole::Full,
                segment_shape: ShapeRole::Full,
                segment: part((r(SurfaceContainerLowest), t), (OnSurface, OnSurface)),
                selected_elevation: 1,
                indicator: false,
                label: TypeRole::LabelMedium,
            },
            (WingsDesign::MacOs, _) => Self {
                height: 24.0,
                padding: 2.0,
                track: r(SecondaryFill),
                track_border: t,
                track_shape: ShapeRole::Small,
                segment_shape: ShapeRole::ExtraSmall,
                segment: part((r(Primary), t), (OnPrimary, OnSurface)),
                selected_elevation: 0,
                indicator: false,
                label: TypeRole::BodyMedium,
            },
            (WingsDesign::Fluent, WingsSegmentedKind::Segmented) => Self {
                height: 32.0,
                padding: 2.0,
                track: r(SurfaceContainerLowest),
                track_border: r(OutlineVariant),
                track_shape: ShapeRole::Small,
                segment_shape: ShapeRole::ExtraSmall,
                segment: part((r(SecondaryFill), t), (OnSurface, OnSurfaceVariant)),
                selected_elevation: 0,
                indicator: true,
                label: TypeRole::BodyMedium,
            },
            (WingsDesign::Fluent, WingsSegmentedKind::Tabs) => Self {
                height: 40.0,
                padding: 0.0,
                track: t,
                track_border: t,
                track_shape: ShapeRole::None,
                segment_shape: ShapeRole::Small,
                segment: part((t, t), (OnSurface, OnSurfaceVariant)),
                selected_elevation: 0,
                indicator: true,
                label: TypeRole::TitleMedium,
            },
        }
    }
}

/// Skins segmented controls and tabs.
pub fn skin_segmented(
    mut commands: Commands,
    skin: Res<WingsSkin>,
    added: Query<Entity, Added<WingsSegmentedKind>>,
    all: Query<Entity, With<WingsSegmentedKind>>,
    mut roots: Query<
        (
            &WingsSegmentedKind,
            &Children,
            &mut Node,
            &mut WingsBackground,
            &mut WingsBorderPaint,
            &mut WingsShape,
        ),
        Without<WingsSegment>,
    >,
    mut segments: Query<
        (&mut Node, &mut WingsShape, &mut WingsCheckedPart),
        (With<WingsSegment>, Without<WingsSegmentedKind>),
    >,
    children: Query<&Children>,
    parts: Query<(), With<WingsSkinPart>>,
    mut texts: Query<&mut WingsTypeStyle>,
) {
    for entity in skin_targets(&skin, added, all) {
        let Ok((kind, kids, mut node, mut bg, mut border, mut shape)) = roots.get_mut(entity)
        else {
            continue;
        };
        let style = WingsSegmentedStyle::for_design(skin.design, *kind);
        node.height = px(style.height);
        node.padding = UiRect::all(px(style.padding));
        node.border = UiRect::all(px(if style.track_border == WingsPaint::Transparent {
            0.0
        } else {
            1.0
        }));
        node.column_gap = px(if skin.design == WingsDesign::Material {
            0.0
        } else {
            2.0
        });
        bg.set_if_neq(WingsBackground(style.track));
        border.set_if_neq(WingsBorderPaint(style.track_border));
        shape.set_if_neq(WingsShape(style.track_shape));
        let count = kids.len();
        for (i, segment) in kids.iter().enumerate() {
            let Ok((mut seg_node, mut seg_shape, mut part)) = segments.get_mut(segment) else {
                continue;
            };
            // Material segmented buttons divide segments with the outline.
            seg_node.border = UiRect {
                left: px(
                    if skin.design == WingsDesign::Material
                        && *kind == WingsSegmentedKind::Segmented
                        && i > 0
                    {
                        1.0
                    } else {
                        0.0
                    },
                ),
                ..default()
            };
            seg_shape.set_if_neq(WingsShape(style.segment_shape));
            // Material segments share the outline: the end segments take its rounded ends.
            if skin.design == WingsDesign::Material && *kind == WingsSegmentedKind::Segmented {
                let r = px(style.height / 2.0);
                let zero = px(0);
                seg_node.border_radius = BorderRadius {
                    top_left: if i == 0 { r } else { zero }.into(),
                    bottom_left: if i == 0 { r } else { zero }.into(),
                    top_right: if i + 1 == count { r } else { zero }.into(),
                    bottom_right: if i + 1 == count { r } else { zero }.into(),
                };
            }
            let mut p = style.segment;
            p.border = Some((
                WingsPaint::Role(ColorRole::Outline),
                WingsPaint::Role(ColorRole::Outline),
            ));
            part.set_if_neq(p);
            commands
                .entity(segment)
                .insert(WingsElevationLevel(0))
                .insert(WingsCheckedRefresh);
            clear_skin_parts(&mut commands, segment, &children, &parts);
            if style.indicator {
                commands.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        bottom: px(0),
                        left: percent(if *kind == WingsSegmentedKind::Tabs {
                            20
                        } else {
                            35
                        }),
                        right: percent(if *kind == WingsSegmentedKind::Tabs {
                            20
                        } else {
                            35
                        }),
                        height: px(3),
                        ..default()
                    },
                    WingsShape(ShapeRole::Full),
                    WingsBackground(WingsPaint::Role(ColorRole::Primary)),
                    WingsCheckedPart {
                        visible_when_checked: true,
                        ..default()
                    },
                    Pickable::IGNORE,
                    WingsSkinPart,
                    ChildOf(segment),
                ));
            }
            for d in children.iter_descendants(segment) {
                if let Ok(mut t) = texts.get_mut(d) {
                    t.set_if_neq(WingsTypeStyle(style.label));
                }
            }
        }
        let _ = style.selected_elevation;
    }
}

/// Gives the selected segment its shadow (iOS lifts it off the track).
pub fn segment_elevation_system(
    skin: Res<WingsSkin>,
    mut segments: Query<(&ChildOf, Has<Checked>, &mut WingsElevationLevel), With<WingsSegment>>,
    kinds: Query<&WingsSegmentedKind>,
) {
    for (parent, checked, mut elevation) in &mut segments {
        let Ok(kind) = kinds.get(parent.parent()) else {
            continue;
        };
        let style = WingsSegmentedStyle::for_design(skin.design, *kind);
        let wanted = if checked { style.selected_elevation } else { 0 };
        elevation.set_if_neq(WingsElevationLevel(wanted));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// iOS lifts a white capsule off a gray track; macOS fills the selection with the accent.
    #[test]
    fn styles() {
        let ios = WingsSegmentedStyle::for_design(WingsDesign::Ios, WingsSegmentedKind::Segmented);
        assert_eq!(ios.height, 32.0);
        assert_eq!(ios.selected_elevation, 1);
        let mac = WingsSegmentedStyle::for_design(WingsDesign::MacOs, WingsSegmentedKind::Tabs);
        assert_eq!(
            mac.segment.background.unwrap().0,
            WingsPaint::Role(ColorRole::Primary)
        );
        assert!(
            WingsSegmentedStyle::for_design(WingsDesign::Material, WingsSegmentedKind::Tabs)
                .indicator
        );
    }
}
