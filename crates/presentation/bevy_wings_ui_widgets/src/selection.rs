//! Selection controls: [`WingsCheckbox`], [`WingsSwitch`], [`WingsRadioGroup`],
//! [`WingsRadioButton`], [`WingsSlider`] and [`WingsChip`].
//!
//! All controls wrap the headless widgets of `bevy_ui_widgets`, so they are keyboard
//! accessible and emit [`ValueChange`](bevy::ui_widgets::ValueChange) events. By default they
//! update themselves (uncontrolled). To drive them from a view model, bind
//! [`Checked`] with `WingsBindChecked` from the MVVM crate, or observe `ValueChange` and write
//! the state yourself.
//!
//! ## Checked parts
//!
//! Visual reaction to the checked state is data driven. Any descendant of a control (or the
//! control itself) can carry a [`WingsCheckedPart`] describing what changes when the owning
//! control toggles: fill, border, visibility, translation and scale. One system,
//! [`checked_parts_system`], walks only the controls whose state changed this frame. Custom
//! controls can reuse the same mechanism by adding [`WingsCheckedOwner`].

use bevy::input_focus::tab_navigation::TabIndex;
use bevy::picking::cursor::EntityCursor;
use bevy::prelude::*;
use bevy::ui::{Checked, InteractionDisabled};
use bevy::ui_widgets::{
    Checkbox, RadioButton, RadioGroup, Slider, SliderRange, SliderThumb, SliderValue, TrackClick,
    checkbox_self_update, radio_self_update, slider_self_update,
};
use bevy::window::SystemCursorIcon;

use bevy_wings_ui_animation::prelude::{
    AnimationSpec, ScaleProperty, TranslationProperty, WingsAnimateExt, WingsEasing, WingsProperty,
};
use bevy_wings_ui_theme::prelude::{
    ColorRole, ShapeRole, TypeRole, WingsBackground, WingsBorderPaint, WingsContentColor,
    WingsPaint, WingsShape,
};

use crate::button::disabled_scene;
use crate::interaction::WingsInteractive;
use crate::text::WingsText;

/// Marks an entity whose [`Checked`] state drives [`WingsCheckedPart`]s in its subtree.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct WingsCheckedOwner;

/// Describes how an entity looks in the checked and unchecked state of its closest
/// [`WingsCheckedOwner`]. Every field is optional; `None` leaves that aspect untouched.
#[derive(Component, Debug, Default, Clone, Copy, PartialEq)]
pub struct WingsCheckedPart {
    /// Fill `(checked, unchecked)`.
    pub background: Option<(WingsPaint, WingsPaint)>,
    /// Border `(checked, unchecked)`.
    pub border: Option<(WingsPaint, WingsPaint)>,
    /// Content color for descendants `(checked, unchecked)`.
    pub content: Option<(WingsPaint, WingsPaint)>,
    /// Show only while checked.
    pub visible_when_checked: bool,
    /// Pixel translation `(checked, unchecked)`, animated.
    pub translate: Option<(Vec2, Vec2)>,
    /// Scale `(checked, unchecked)`, animated.
    pub scale: Option<(Vec2, Vec2)>,
}

impl WingsCheckedPart {
    /// Fill paint for a state.
    pub fn background_for(&self, checked: bool) -> Option<WingsPaint> {
        self.background
            .map(|(on, off)| if checked { on } else { off })
    }
    /// Border paint for a state.
    pub fn border_for(&self, checked: bool) -> Option<WingsPaint> {
        self.border.map(|(on, off)| if checked { on } else { off })
    }
    /// Content paint for a state.
    pub fn content_for(&self, checked: bool) -> Option<WingsPaint> {
        self.content.map(|(on, off)| if checked { on } else { off })
    }
    /// Translation for a state.
    pub fn translate_for(&self, checked: bool) -> Option<Vec2> {
        self.translate
            .map(|(on, off)| if checked { on } else { off })
    }
    /// Scale for a state.
    pub fn scale_for(&self, checked: bool) -> Option<Vec2> {
        self.scale.map(|(on, off)| if checked { on } else { off })
    }
}

/// Motion used by checked part transforms.
pub const CHECKED_MOTION: AnimationSpec = AnimationSpec::Spring {
    spring: bevy_wings_ui_animation::prelude::SpringSpec::BOUNCY,
    delay: 0.0,
};

/// Progress along the checkbox's continuous two-segment path.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct WingsCheckMarkProgress(
    /// Fraction of the path to reveal, clamped to `0.0..=1.0` when drawn.
    pub f32,
);

pub(crate) struct CheckMarkProperty;
impl WingsProperty for CheckMarkProperty {
    type Target = WingsCheckMarkProgress;
    type Value = f32;
    fn get(target: &Self::Target) -> f32 {
        target.0
    }
    fn set(target: &mut Self::Target, value: f32) {
        target.0 = value;
    }
}

#[derive(Component, Default, Clone, Copy)]
pub(crate) struct CheckMarkStroke(usize);

const CHECK_POINTS: [Vec2; 3] = [
    Vec2::new(2.0, 7.0),
    Vec2::new(5.0, 10.0),
    Vec2::new(12.0, 3.0),
];

/// Reveals the short downward stroke, then the long upward stroke, at constant path speed.
pub(crate) fn draw_checkmarks(
    marks: Query<(&WingsCheckMarkProgress, &Children), Changed<WingsCheckMarkProgress>>,
    mut strokes: Query<(&CheckMarkStroke, &mut Node, &mut UiTransform)>,
) {
    let lengths = [
        CHECK_POINTS[0].distance(CHECK_POINTS[1]),
        CHECK_POINTS[1].distance(CHECK_POINTS[2]),
    ];
    let total = lengths[0] + lengths[1];
    for (progress, children) in &marks {
        for child in children {
            let Ok((stroke, mut node, mut transform)) = strokes.get_mut(*child) else {
                continue;
            };
            let i = stroke.0;
            let start = CHECK_POINTS[i];
            let direction = (CHECK_POINTS[i + 1] - start) / lengths[i];
            let preceding = if i == 0 { 0.0 } else { lengths[0] };
            let length = (progress.0.clamp(0.0, 1.0) * total - preceding).clamp(0.0, lengths[i]);
            let center = start + direction * length * 0.5;
            // UI transforms rotate around the node center. Move that center so the
            // first endpoint stays fixed while the other endpoint travels along the path.
            node.width = px(length);
            node.left = px(center.x - length * 0.5);
            node.top = px(center.y - 1.0);
            transform.rotation = Rot2::radians(direction.y.atan2(direction.x));
        }
    }
}

/// Applies [`WingsCheckedPart`]s for every owner whose checked state changed (or that was just
/// spawned, in which case transforms snap instead of animating).
pub fn checked_parts_system(
    mut commands: Commands,
    added_owner: Query<Entity, Added<WingsCheckedOwner>>,
    added_checked: Query<Entity, (With<WingsCheckedOwner>, Added<Checked>)>,
    mut removed_checked: RemovedComponents<Checked>,
    owners: Query<Has<Checked>, With<WingsCheckedOwner>>,
    children: Query<&Children>,
    mut parts: Query<(
        &WingsCheckedPart,
        Option<&mut WingsBackground>,
        Option<&mut WingsBorderPaint>,
        Option<&mut WingsContentColor>,
        &mut Node,
        Option<&mut UiTransform>,
        Option<&mut WingsCheckMarkProgress>,
    )>,
) {
    let fresh: Vec<Entity> = added_owner.iter().collect();
    let toggled = added_checked
        .iter()
        .chain(removed_checked.read())
        .filter(|e| !fresh.contains(e));
    let work: Vec<(Entity, bool)> = fresh
        .iter()
        .map(|e| (*e, true))
        .chain(toggled.map(|e| (e, false)))
        .collect();
    for (owner, snap) in work {
        let Ok(checked) = owners.get(owner) else {
            continue;
        };
        let targets = core::iter::once(owner).chain(children.iter_descendants(owner));
        for target in targets {
            let Ok((part, bg, border, content, mut node, transform, progress)) =
                parts.get_mut(target)
            else {
                continue;
            };
            let part = *part;
            if let (Some(paint), Some(mut bg)) = (part.background_for(checked), bg) {
                bg.set_if_neq(WingsBackground(paint));
            }
            if let (Some(paint), Some(mut b)) = (part.border_for(checked), border) {
                b.set_if_neq(WingsBorderPaint(paint));
            }
            if let (Some(paint), Some(mut c)) = (part.content_for(checked), content) {
                c.set_if_neq(WingsContentColor(paint));
            }
            if part.visible_when_checked {
                let display = if checked {
                    Display::Flex
                } else {
                    Display::None
                };
                if node.display != display {
                    node.display = display;
                }
            }
            let translate = part.translate_for(checked);
            let scale = part.scale_for(checked);
            if let Some(mut progress) = progress {
                let target_progress = if checked { 1.0 } else { 0.0 };
                if snap {
                    progress.0 = target_progress;
                } else {
                    commands.entity(target).animate::<CheckMarkProperty>(
                        target_progress,
                        AnimationSpec::millis(220).with_easing(WingsEasing::Linear),
                    );
                }
            }
            if snap {
                if let Some(mut t) = transform {
                    if let Some(v) = translate {
                        t.translation = Val2::px(v.x, v.y);
                    }
                    if let Some(s) = scale {
                        t.scale = s;
                    }
                }
            } else {
                let mut ec = commands.entity(target);
                if let Some(v) = translate {
                    ec.animate::<TranslationProperty>(v, CHECKED_MOTION);
                }
                if let Some(s) = scale {
                    ec.animate::<ScaleProperty>(s, CHECKED_MOTION);
                }
            }
        }
    }
}

fn checked_scene(checked: bool) -> Box<dyn Scene> {
    if checked {
        Box::new(bsn! { Checked })
    } else {
        Box::new(bsn! {})
    }
}

fn label_scene(label: String) -> Box<dyn Scene> {
    if label.is_empty() {
        Box::new(bsn! {})
    } else {
        Box::new(bsn! { Children [ @WingsText { @text: label, @role: TypeRole::BodyLarge } ] })
    }
}

/// Shared props of checkbox like controls.
#[derive(Clone)]
pub struct WingsToggleProps {
    /// Initial state.
    pub checked: bool,
    /// Optional trailing label. Clicking the label toggles the control too.
    pub label: String,
    /// When `false` the control starts disabled.
    pub enabled: bool,
}

impl Default for WingsToggleProps {
    fn default() -> Self {
        Self {
            checked: false,
            label: String::new(),
            enabled: true,
        }
    }
}

fn toggle_root(props: &WingsToggleProps) -> impl Scene + use<> {
    let checked = checked_scene(props.checked);
    let disabled = disabled_scene(props.enabled);
    bsn! {
        Node {
            align_items: AlignItems::Center,
            column_gap: px(12),
            padding: px(4),
        }
        WingsShape(ShapeRole::Small)
        WingsInteractive
        WingsCheckedOwner
        TabIndex(0)
        EntityCursor::System(SystemCursorIcon::Pointer)
        on(checkbox_self_update)
        @{checked}
        @{disabled}
    }
}

/// A themed checkbox with an animated check mark and an optional label.
///
/// ```ignore
/// bsn! {
///     @WingsCheckbox { @label: "Remember me", @checked: true }
///     on(|change: On<ValueChange<bool>>| info!("now {}", change.value))
/// }
/// ```
#[derive(SceneComponent, Default, Clone)]
#[scene(WingsToggleProps)]
pub struct WingsCheckbox;

impl WingsCheckbox {
    fn scene(props: WingsToggleProps) -> impl Scene {
        let root = toggle_root(&props);
        let label = label_scene(props.label);
        let box_part = WingsCheckedPart {
            background: Some((
                WingsPaint::Role(ColorRole::Primary),
                WingsPaint::Transparent,
            )),
            border: Some((
                WingsPaint::Role(ColorRole::Primary),
                WingsPaint::Role(ColorRole::OnSurfaceVariant),
            )),
            ..default()
        };
        bsn! {
            Checkbox
            @{root}
            Children [
                Node {
                    width: px(18),
                    height: px(18),
                    border: px(2),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                }
                WingsShape(ShapeRole::ExtraSmall)
                WingsBackground
                WingsBorderPaint
                box_part
                Children [
                    Node { width: px(14), height: px(14) }
                    WingsCheckMarkProgress
                    WingsCheckedPart
                    Children [
                        Node {
                            position_type: PositionType::Absolute,
                            width: px(0),
                            height: px(2),
                            border_radius: BorderRadius::all(px(1)),
                        }
                        UiTransform
                        WingsBackground({WingsPaint::Role(ColorRole::OnPrimary)})
                        CheckMarkStroke(0)
                        --
                        Node {
                            position_type: PositionType::Absolute,
                            width: px(0),
                            height: px(2),
                            border_radius: BorderRadius::all(px(1)),
                        }
                        UiTransform
                        WingsBackground({WingsPaint::Role(ColorRole::OnPrimary)})
                        CheckMarkStroke(1)
                    ]
                ]
            ]
            @{label}
        }
    }
}

/// A themed on/off switch with an optional label. Emits `ValueChange<bool>`.
#[derive(SceneComponent, Default, Clone)]
#[scene(WingsToggleProps)]
pub struct WingsSwitch;

impl WingsSwitch {
    fn scene(props: WingsToggleProps) -> impl Scene {
        let root = toggle_root(&props);
        let label = label_scene(props.label);
        let track_part = WingsCheckedPart {
            background: Some((
                WingsPaint::Role(ColorRole::Primary),
                WingsPaint::Role(ColorRole::SurfaceContainerHighest),
            )),
            border: Some((
                WingsPaint::Role(ColorRole::Primary),
                WingsPaint::Role(ColorRole::Outline),
            )),
            ..default()
        };
        let thumb_part = WingsCheckedPart {
            background: Some((
                WingsPaint::Role(ColorRole::OnPrimary),
                WingsPaint::Role(ColorRole::Outline),
            )),
            translate: Some((Vec2::new(20.0, 0.0), Vec2::ZERO)),
            scale: Some((Vec2::ONE, Vec2::splat(0.67))),
            ..default()
        };
        bsn! {
            Checkbox
            @{root}
            Children [
                Node {
                    width: px(52),
                    height: px(32),
                    border: px(2),
                    padding: px(2),
                    align_items: AlignItems::Center,
                }
                WingsShape(ShapeRole::Full)
                WingsBackground
                WingsBorderPaint
                track_part
                Children [
                    Node { width: px(24), height: px(24) }
                    WingsShape(ShapeRole::Full)
                    WingsBackground
                    UiTransform
                    thumb_part
                ]
            ]
            @{label}
        }
    }
}

/// Container of mutually exclusive [`WingsRadioButton`]s. Emits `ValueChange<Entity>` with the
/// selected button.
#[derive(SceneComponent, Default, Clone)]
#[scene(WingsRadioGroupProps)]
pub struct WingsRadioGroup;

/// Props of [`WingsRadioGroup`].
#[derive(Default, Clone)]
pub struct WingsRadioGroupProps {
    /// Lay the options out horizontally.
    pub horizontal: bool,
    /// Gap between options in pixels.
    pub gap: f32,
}

impl WingsRadioGroup {
    fn scene(props: WingsRadioGroupProps) -> impl Scene {
        let node = Node {
            flex_direction: if props.horizontal {
                FlexDirection::Row
            } else {
                FlexDirection::Column
            },
            row_gap: px(props.gap),
            column_gap: px(props.gap),
            ..default()
        };
        bsn! {
            node
            RadioGroup
            on(radio_self_update)
        }
    }
}

/// One option of a [`WingsRadioGroup`].
#[derive(SceneComponent, Default, Clone)]
#[scene(WingsToggleProps)]
pub struct WingsRadioButton;

impl WingsRadioButton {
    fn scene(props: WingsToggleProps) -> impl Scene {
        let checked = checked_scene(props.checked);
        let disabled = disabled_scene(props.enabled);
        let label = label_scene(props.label);
        let ring_part = WingsCheckedPart {
            border: Some((
                WingsPaint::Role(ColorRole::Primary),
                WingsPaint::Role(ColorRole::OnSurfaceVariant),
            )),
            ..default()
        };
        let dot_part = WingsCheckedPart {
            visible_when_checked: true,
            scale: Some((Vec2::ONE, Vec2::ZERO)),
            ..default()
        };
        bsn! {
            Node {
                align_items: AlignItems::Center,
                column_gap: px(12),
                padding: px(4),
            }
            RadioButton
            WingsShape(ShapeRole::Small)
            WingsInteractive
            WingsCheckedOwner
            EntityCursor::System(SystemCursorIcon::Pointer)
            @{checked}
            @{disabled}
            Children [
                Node {
                    width: px(20),
                    height: px(20),
                    border: px(2),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                }
                WingsShape(ShapeRole::Full)
                WingsBorderPaint
                ring_part
                Children [
                    Node { width: px(10), height: px(10) }
                    WingsShape(ShapeRole::Full)
                    WingsBackground({WingsPaint::Role(ColorRole::Primary)})
                    UiTransform
                    dot_part
                ]
            ]
            @{label}
        }
    }
}

/// Size of the slider thumb in pixels.
pub const SLIDER_THUMB: f32 = 20.0;

/// Marks the active (filled) portion of a [`WingsSlider`] track.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct WingsSliderFill;

/// A themed slider with drag and click-to-position input. Emits `ValueChange<f32>`.
///
/// ```ignore
/// bsn! { @WingsSlider { @value: 0.5, @min: 0.0, @max: 1.0 } }
/// ```
#[derive(SceneComponent, Default, Clone)]
#[scene(WingsSliderProps)]
pub struct WingsSlider;

/// Props of [`WingsSlider`].
#[derive(Clone)]
pub struct WingsSliderProps {
    /// Initial value.
    pub value: f32,
    /// Lower bound.
    pub min: f32,
    /// Upper bound.
    pub max: f32,
    /// Width of the slider.
    pub width: Val,
    /// When `false` the slider starts disabled.
    pub enabled: bool,
}

impl Default for WingsSliderProps {
    fn default() -> Self {
        Self {
            value: 0.0,
            min: 0.0,
            max: 1.0,
            width: Val::Px(200.0),
            enabled: true,
        }
    }
}

impl WingsSlider {
    fn scene(props: WingsSliderProps) -> impl Scene {
        let range = SliderRange::new(props.min, props.max);
        let value = SliderValue(range.clamp(props.value));
        let disabled = disabled_scene(props.enabled);
        let width = props.width;
        let half = SLIDER_THUMB / 2.0;
        bsn! {
            Node { width: width, height: px(44), align_items: AlignItems::Center }
            Slider { track_click: TrackClick::Snap }
            range
            value
            WingsInteractive
            TabIndex(0)
            EntityCursor::System(SystemCursorIcon::Pointer)
            on(slider_self_update)
            @{disabled}
            Children [
                Node {
                        position_type: PositionType::Absolute,
                        left: px(half),
                        right: px(half),
                        height: px(4),
                    }
                    WingsShape(ShapeRole::Full)
                    WingsBackground({WingsPaint::Role(ColorRole::SurfaceContainerHighest)})
                    Children [
                        Node { height: percent(100) }
                        WingsSliderFill
                        WingsShape(ShapeRole::Full)
                        WingsBackground({WingsPaint::Role(ColorRole::Primary)})
                    ]
                --
                Node {
                    position_type: PositionType::Absolute,
                    width: px(SLIDER_THUMB),
                    height: px(SLIDER_THUMB),
                }
                SliderThumb
                WingsShape(ShapeRole::Full)
                WingsBackground({WingsPaint::Role(ColorRole::Primary)})
            ]
        }
    }
}

/// Normalized position of `value` in `range`, in `0..=1`.
pub fn slider_fraction(range: &SliderRange, value: f32) -> f32 {
    if range.span() <= 0.0 {
        0.0
    } else {
        ((range.clamp(value) - range.start()) / range.span()).clamp(0.0, 1.0)
    }
}

/// Positions the thumb and the fill of sliders whose value or range changed.
pub fn slider_layout_system(
    sliders: Query<
        (Entity, &SliderValue, &SliderRange),
        (
            With<WingsSlider>,
            Or<(Changed<SliderValue>, Changed<SliderRange>)>,
        ),
    >,
    children: Query<&Children>,
    mut thumbs: Query<&mut Node, (With<SliderThumb>, Without<WingsSliderFill>)>,
    mut fills: Query<&mut Node, (With<WingsSliderFill>, Without<SliderThumb>)>,
) {
    for (slider, value, range) in &sliders {
        let t = slider_fraction(range, value.0);
        for d in children.iter_descendants(slider) {
            if let Ok(mut node) = thumbs.get_mut(d) {
                node.left = percent(t * 100.0);
                node.margin.left = px(-t * SLIDER_THUMB);
            } else if let Ok(mut node) = fills.get_mut(d) {
                node.width = percent(t * 100.0);
            }
        }
    }
}

/// A compact, toggleable filter chip. Emits `ValueChange<bool>`.
#[derive(SceneComponent, Default, Clone)]
#[scene(WingsToggleProps)]
pub struct WingsChip;

impl WingsChip {
    fn scene(props: WingsToggleProps) -> impl Scene {
        let checked = checked_scene(props.checked);
        let disabled = disabled_scene(props.enabled);
        let text = props.label;
        let part = WingsCheckedPart {
            background: Some((
                WingsPaint::Role(ColorRole::SecondaryContainer),
                WingsPaint::Transparent,
            )),
            border: Some((
                WingsPaint::Transparent,
                WingsPaint::Role(ColorRole::Outline),
            )),
            content: Some((
                WingsPaint::Role(ColorRole::OnSecondaryContainer),
                WingsPaint::Role(ColorRole::OnSurfaceVariant),
            )),
            ..default()
        };
        bsn! {
            Node {
                height: px(32),
                padding: UiRect::horizontal(px(16)),
                border: px(1),
                align_items: AlignItems::Center,
            }
            Checkbox
            WingsShape(ShapeRole::Small)
            WingsBackground
            WingsBorderPaint
            WingsContentColor
            WingsInteractive
            WingsCheckedOwner
            TabIndex(0)
            EntityCursor::System(SystemCursorIcon::Pointer)
            on(checkbox_self_update)
            part
            @{checked}
            @{disabled}
            Children [ @WingsText { @text: text, @role: TypeRole::LabelLarge } ]
        }
    }
}

/// Re-exported so users can disable controls without importing `bevy::ui`.
pub type WingsDisabled = InteractionDisabled;

#[cfg(test)]
mod tests {
    use super::*;

    /// Fractions are clamped and handle empty ranges.
    #[test]
    fn fraction() {
        let r = SliderRange::new(10.0, 20.0);
        assert_eq!(slider_fraction(&r, 15.0), 0.5);
        assert_eq!(slider_fraction(&r, 0.0), 0.0);
        assert_eq!(slider_fraction(&r, 99.0), 1.0);
        assert_eq!(slider_fraction(&SliderRange::new(1.0, 1.0), 1.0), 0.0);
    }

    /// Part helpers return the checked or unchecked side.
    #[test]
    fn part_sides() {
        let p = WingsCheckedPart {
            translate: Some((Vec2::X, Vec2::ZERO)),
            ..default()
        };
        assert_eq!(p.translate_for(true), Some(Vec2::X));
        assert_eq!(p.translate_for(false), Some(Vec2::ZERO));
        assert_eq!(p.background_for(true), None);
    }
}
