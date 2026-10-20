//! Feedback widgets: progress indicators, skeletons, badges, snackbars and dialogs.

use bevy::prelude::*;
use bevy::ui_widgets::Activate;

use bevy_wings_ui_animation::prelude::OpacityProperty;
use bevy_wings_ui_animation::prelude::{
    AnimationSpec, Repeat, WingsAnimateExt, WingsAnimatedVisibility, WingsEasing, WingsProperty,
    WingsTransition,
};
use bevy_wings_ui_core::WingsOpacity;
use bevy_wings_ui_theme::prelude::{
    ColorRole, ShapeRole, TypeRole, WingsBackground, WingsBorderPaint, WingsContentColor,
    WingsDesign, WingsElevationLevel, WingsPaint, WingsShape,
};

use crate::button::{WingsButton, WingsButtonOverride, WingsButtonPalette, WingsButtonVariant};
use crate::skin::WingsSkin;
use crate::text::WingsText;

// ---------------------------------------------------------------------------------------------
// Linear progress
// ---------------------------------------------------------------------------------------------

/// Progress of a [`WingsLinearProgress`] or `WingsCircularProgress` in `0..=1`. Negative values
/// mean indeterminate. Change it at runtime and the indicator follows.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct WingsProgress(pub f32);

impl Default for WingsProgress {
    fn default() -> Self {
        Self(-1.0)
    }
}

impl WingsProgress {
    /// Indeterminate progress.
    pub const INDETERMINATE: Self = Self(-1.0);

    /// True when no progress value is known.
    pub fn is_indeterminate(self) -> bool {
        self.0 < 0.0
    }

    /// The clamped progress, `None` when indeterminate.
    pub fn fraction(self) -> Option<f32> {
        (!self.is_indeterminate()).then(|| self.0.clamp(0.0, 1.0))
    }
}

/// Horizontal translation of a [`UiTransform`] in percent of the node's own width, used by the
/// indeterminate sweep.
pub struct SweepProperty;

impl WingsProperty for SweepProperty {
    type Target = UiTransform;
    type Value = f32;
    fn get(t: &UiTransform) -> f32 {
        match t.translation.x {
            Val::Percent(p) => p,
            _ => 0.0,
        }
    }
    fn set(t: &mut UiTransform, v: f32) {
        t.translation.x = Val::Percent(v);
    }
}

/// Marks the indicator bar of a [`WingsLinearProgress`].
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct WingsProgressBar;

/// A thin horizontal progress bar.
///
/// ```ignore
/// bsn! { @WingsLinearProgress { @progress: 0.4 } }
/// bsn! { @WingsLinearProgress }          // indeterminate
/// ```
#[derive(SceneComponent, Default, Clone)]
#[scene(WingsProgressProps)]
pub struct WingsLinearProgress;

/// Props of the progress indicators.
#[derive(Clone)]
pub struct WingsProgressProps {
    /// Initial progress, negative for indeterminate.
    pub progress: f32,
    /// Width (linear) or diameter (circular).
    pub size: Val,
}

impl Default for WingsProgressProps {
    fn default() -> Self {
        Self {
            progress: -1.0,
            size: Val::Percent(100.0),
        }
    }
}

impl WingsLinearProgress {
    fn scene(props: WingsProgressProps) -> impl Scene {
        let progress = WingsProgress(props.progress);
        let width = props.size;
        bsn! {
            Node { width: width, height: px(4), overflow: Overflow::clip() }
            progress
            WingsShape(ShapeRole::Full)
            WingsBackground({WingsPaint::Role(ColorRole::SecondaryContainer)})
            Children [
                Node { height: percent(100) }
                WingsProgressBar
                UiTransform
                WingsShape(ShapeRole::Full)
                WingsBackground({WingsPaint::Role(ColorRole::Primary)})
            ]
        }
    }
}

/// Width of the indeterminate sweep, in percent of the track.
pub const SWEEP_WIDTH: f32 = 30.0;

/// Updates bar widths and starts or stops the indeterminate sweep.
pub fn linear_progress_system(
    mut commands: Commands,
    tracks: Query<(&WingsProgress, &Children), (With<WingsLinearProgress>, Changed<WingsProgress>)>,
    mut bars: Query<(Entity, &mut Node, &mut UiTransform), With<WingsProgressBar>>,
) {
    for (progress, children) in &tracks {
        for child in children.iter() {
            let Ok((bar, mut node, mut transform)) = bars.get_mut(child) else {
                continue;
            };
            match progress.fraction() {
                Some(f) => {
                    commands.entity(bar).stop_animation::<SweepProperty>();
                    transform.translation.x = Val::Px(0.0);
                    commands
                        .entity(bar)
                        .animate::<crate::feedback::BarWidthProperty>(
                            f * 100.0,
                            AnimationSpec::short(),
                        );
                    if node.width == Val::Auto {
                        node.width = percent(f * 100.0);
                    }
                }
                None => {
                    node.width = percent(SWEEP_WIDTH);
                    commands.entity(bar).animate_from_to::<SweepProperty>(
                        -100.0,
                        100.0 / SWEEP_WIDTH * 100.0,
                        AnimationSpec::millis(1400).with_easing(WingsEasing::EaseInOut),
                        Repeat::infinite(),
                    );
                }
            }
        }
    }
}

/// Width of a [`Node`] in percent (smoothly animates determinate progress).
pub struct BarWidthProperty;

impl WingsProperty for BarWidthProperty {
    type Target = Node;
    type Value = f32;
    fn get(n: &Node) -> f32 {
        match n.width {
            Val::Percent(p) => p,
            _ => 0.0,
        }
    }
    fn set(n: &mut Node, v: f32) {
        n.width = Val::Percent(v);
    }
}

// ---------------------------------------------------------------------------------------------
// Circular progress (shader)
// ---------------------------------------------------------------------------------------------

/// A circular progress indicator drawn by a custom UI shader (anti aliased arc, rotating
/// indeterminate mode). Requires the `shaders` feature.
#[cfg(feature = "shaders")]
#[derive(SceneComponent, Default, Clone)]
#[scene(WingsCircularProgressProps)]
pub struct WingsCircularProgress;

/// Props of [`WingsCircularProgress`].
#[cfg(feature = "shaders")]
#[derive(Clone)]
pub struct WingsCircularProgressProps {
    /// Initial progress, negative for indeterminate.
    pub progress: f32,
    /// Diameter in pixels.
    pub size: f32,
}

#[cfg(feature = "shaders")]
impl Default for WingsCircularProgressProps {
    fn default() -> Self {
        Self {
            progress: -1.0,
            size: 48.0,
        }
    }
}

#[cfg(feature = "shaders")]
impl WingsCircularProgress {
    fn scene(props: WingsCircularProgressProps) -> impl Scene {
        let progress = WingsProgress(props.progress);
        let size = props.size;
        bsn! {
            Node { width: px(size), height: px(size) }
            progress
        }
    }
}

/// Creates materials for new circular indicators and keeps their colors and progress current.
#[cfg(feature = "shaders")]
pub fn circular_progress_system(
    mut commands: Commands,
    theme: Res<bevy_wings_ui_theme::prelude::WingsTheme>,
    materials: Option<ResMut<Assets<bevy_wings_ui_shaders::WingsCircularProgressMaterial>>>,
    added: Query<
        (Entity, &WingsProgress),
        (
            With<WingsCircularProgress>,
            Without<MaterialNode<bevy_wings_ui_shaders::WingsCircularProgressMaterial>>,
            Without<crate::feedback_skin::WingsSpinner>,
        ),
    >,
    changed: Query<
        (
            &WingsProgress,
            &MaterialNode<bevy_wings_ui_shaders::WingsCircularProgressMaterial>,
        ),
        Changed<WingsProgress>,
    >,
    all: Query<&MaterialNode<bevy_wings_ui_shaders::WingsCircularProgressMaterial>>,
) {
    use bevy_wings_ui_shaders::WingsCircularProgressMaterial as M;
    let Some(mut materials) = materials else {
        return;
    };
    let indicator = theme.color(ColorRole::Primary);
    let track = theme.color(ColorRole::SecondaryContainer);
    for (entity, progress) in &added {
        let mut m = match progress.fraction() {
            Some(f) => M::determinate(indicator, f),
            None => M::indeterminate(indicator),
        };
        m.track = track.into();
        commands
            .entity(entity)
            .insert(MaterialNode(materials.add(m)));
    }
    for (progress, handle) in &changed {
        if let Some(mut m) = materials.get_mut(&handle.0) {
            m.set_progress(progress.0);
        }
    }
    if theme.is_changed() {
        for handle in &all {
            if let Some(mut m) = materials.get_mut(&handle.0) {
                m.indicator = indicator.into();
                m.track = track.into();
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Skeleton
// ---------------------------------------------------------------------------------------------

/// Loading placeholder. With the `shaders` feature it renders an animated shimmer, otherwise it
/// pulses its opacity.
#[derive(SceneComponent, Default, Clone)]
#[scene(WingsSkeletonProps)]
pub struct WingsSkeleton;

/// Props of [`WingsSkeleton`].
#[derive(Clone)]
pub struct WingsSkeletonProps {
    /// Width.
    pub width: Val,
    /// Height.
    pub height: Val,
    /// Corner shape.
    pub shape: ShapeRole,
}

impl Default for WingsSkeletonProps {
    fn default() -> Self {
        Self {
            width: Val::Percent(100.0),
            height: Val::Px(16.0),
            shape: ShapeRole::Small,
        }
    }
}

impl WingsSkeleton {
    fn scene(props: WingsSkeletonProps) -> impl Scene {
        let (width, height, shape) = (props.width, props.height, props.shape);
        bsn! {
            Node { width: width, height: height }
            WingsShape(shape)
            WingsOpacity
        }
    }
}

/// Starts the skeleton effect on new skeletons and after design changes, and keeps shimmer
/// colors on theme. Material and Fluent shimmer (with the `shaders` feature); the Apple designs
/// pulse a gray placeholder like SwiftUI's `.redacted(reason: .placeholder)`. Without shader
/// materials (headless apps, `shaders` disabled) every design pulses.
pub fn skeleton_system(
    mut commands: Commands,
    skin: Res<WingsSkin>,
    added: Query<Entity, Added<WingsSkeleton>>,
    every: Query<Entity, With<WingsSkeleton>>,
    #[cfg(feature = "shaders")] theme: Res<bevy_wings_ui_theme::prelude::WingsTheme>,
    #[cfg(feature = "shaders")] materials: Option<
        ResMut<Assets<bevy_wings_ui_shaders::WingsShimmerMaterial>>,
    >,
    #[cfg(feature = "shaders")] all: Query<
        &MaterialNode<bevy_wings_ui_shaders::WingsShimmerMaterial>,
    >,
) {
    let targets = crate::skin::skin_targets(&skin, added, every);
    #[cfg(feature = "shaders")]
    if let Some(mut materials) = materials {
        let base = theme.color(ColorRole::SurfaceContainerHighest);
        let highlight = theme.color(ColorRole::SurfaceContainerLow);
        if !skin.apple() {
            for entity in targets {
                let m = bevy_wings_ui_shaders::WingsShimmerMaterial::new(base, highlight);
                commands
                    .entity(entity)
                    .stop_animation::<OpacityProperty>()
                    .insert((
                        WingsOpacity(1.0),
                        WingsBackground(WingsPaint::Transparent),
                        MaterialNode(materials.add(m)),
                    ));
            }
            if theme.is_changed() {
                for handle in &all {
                    if let Some(mut m) = materials.get_mut(&handle.0) {
                        m.base = base.into();
                        m.highlight = highlight.into();
                    }
                }
            }
            return;
        }
        for entity in &targets {
            commands
                .entity(*entity)
                .remove::<MaterialNode<bevy_wings_ui_shaders::WingsShimmerMaterial>>();
        }
    }
    let fill = if skin.apple() {
        ColorRole::Fill
    } else {
        ColorRole::SurfaceContainerHighest
    };
    for entity in targets {
        commands
            .entity(entity)
            .insert(WingsBackground(WingsPaint::Role(fill)))
            .animate_from_to::<OpacityProperty>(
                1.0,
                0.45,
                AnimationSpec::millis(900).with_easing(WingsEasing::EaseInOut),
                Repeat::ping_pong(),
            );
    }
}

// ---------------------------------------------------------------------------------------------
// Badge
// ---------------------------------------------------------------------------------------------

/// A small status marker, usually placed in the corner of an icon. A count of zero renders a
/// dot.
///
/// ```ignore
/// bsn! {
///     @WingsBox
///     Children [ @WingsIcon { @image: "mail.png" } -- @WingsBadge { @count: 3 } ]
/// }
/// ```
#[derive(SceneComponent, Default, Clone)]
#[scene(WingsBadgeProps)]
pub struct WingsBadge;

/// Props of [`WingsBadge`].
#[derive(Default, Clone)]
pub struct WingsBadgeProps {
    /// Count to show. `0` renders a dot.
    pub count: u32,
}

/// Formats a badge count (`999+` above 999).
pub fn badge_label(count: u32) -> String {
    if count > 999 {
        "999+".to_string()
    } else {
        count.to_string()
    }
}

impl WingsBadge {
    fn scene(props: WingsBadgeProps) -> impl Scene {
        let (node, label): (Node, Box<dyn Scene>) = match props.count {
            0 => (
                Node {
                    width: px(6),
                    height: px(6),
                    position_type: PositionType::Absolute,
                    top: px(0),
                    right: px(0),
                    ..default()
                },
                Box::new(bsn! {}),
            ),
            c => {
                let text = badge_label(c);
                (
                    Node {
                        min_width: px(16),
                        height: px(16),
                        padding: UiRect::horizontal(px(4)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        position_type: PositionType::Absolute,
                        top: px(-4),
                        right: px(-8),
                        ..default()
                    },
                    Box::new(bsn! {
                        Children [ @WingsText { @text: text, @role: TypeRole::LabelSmall } ]
                    }),
                )
            }
        };
        bsn! {
            node
            WingsShape(ShapeRole::Full)
            WingsBackground({WingsPaint::Role(ColorRole::Error)})
            WingsContentColor({WingsPaint::Role(ColorRole::OnError)})
            @{label}
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Snackbar
// ---------------------------------------------------------------------------------------------

/// Where snackbars appear. Place one near the root of a screen (the scaffold adds one).
#[derive(Component, Debug, Default, Clone, Copy)]
#[require(Node)]
pub struct WingsSnackbarHost;

/// Message that queues a snackbar on every [`WingsSnackbarHost`].
///
/// ```ignore
/// fn saved(mut snack: MessageWriter<WingsShowSnackbar>) {
///     snack.write(WingsShowSnackbar::new("Saved"));
/// }
/// ```
#[derive(Message, Debug, Clone, PartialEq)]
pub struct WingsShowSnackbar {
    /// Message text.
    pub text: String,
    /// Optional action label. Clicking it triggers [`WingsSnackbarAction`].
    pub action: Option<String>,
    /// Seconds before the snackbar hides itself.
    pub duration: f32,
}

impl WingsShowSnackbar {
    /// A snackbar shown for 4 seconds.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            action: None,
            duration: 4.0,
        }
    }

    /// Adds an action button.
    pub fn with_action(mut self, label: impl Into<String>) -> Self {
        self.action = Some(label.into());
        self
    }

    /// Sets the display duration in seconds.
    pub fn with_duration(mut self, seconds: f32) -> Self {
        self.duration = seconds;
        self
    }
}

/// Triggered on a snackbar entity when its action button was clicked.
#[derive(EntityEvent, Debug, Clone)]
pub struct WingsSnackbarAction {
    /// The snackbar.
    pub entity: Entity,
    /// The action label.
    pub label: String,
}

/// Countdown of a visible snackbar.
#[derive(Component, Debug, Clone, Copy)]
pub struct WingsSnackbar {
    /// Remaining visible time in seconds.
    pub remaining: f32,
    /// Remaining time of the exit transition, once hiding.
    pub exiting: Option<f32>,
}

fn snackbar_scene(msg: &WingsShowSnackbar, design: WingsDesign) -> impl Scene + use<> {
    let text = msg.text.clone();
    let apple = design.is_apple();
    let action_color = match design {
        WingsDesign::Material => ColorRole::InversePrimary,
        _ => ColorRole::Primary,
    };
    let action: Box<dyn Scene> = match &msg.action {
        None => Box::new(bsn! {}),
        Some(label) => {
            let label = label.clone();
            let action_label = label.clone();
            let action_override = WingsButtonOverride {
                palette: Some(WingsButtonPalette {
                    container: WingsPaint::Transparent,
                    content: action_color,
                    border: WingsPaint::Transparent,
                    elevation: 0,
                }),
                min_height: None,
                grow: false,
            };
            Box::new(bsn! {
                Children [
                    @WingsButton { @text: label, @variant: WingsButtonVariant::Text }
                    action_override
                    on(move |ev: On<Activate>, parents: Query<&ChildOf>, mut commands: Commands| {
                        if let Ok(parent) = parents.get(ev.event_target()) {
                            commands.trigger(WingsSnackbarAction {
                                entity: parent.parent(),
                                label: action_label.clone(),
                            });
                        }
                    })
                ]
            })
        }
    };
    let snackbar = WingsSnackbar {
        remaining: msg.duration,
        exiting: None,
    };
    let visibility = WingsAnimatedVisibility {
        visible: true,
        enter: WingsTransition::slide_vertically(24.0).and(WingsTransition::fade()),
        exit: WingsTransition::fade(),
    };
    // Material: inverse surface bar. Apple: a floating capsule toast. Fluent: an info bar.
    let (shape, container, content, border, min_width, height, pad) = match design {
        WingsDesign::Material => (
            ShapeRole::ExtraSmall,
            ColorRole::InverseSurface,
            ColorRole::InverseOnSurface,
            WingsPaint::Transparent,
            288.0,
            48.0,
            16.0,
        ),
        WingsDesign::Fluent => (
            ShapeRole::Large,
            ColorRole::SurfaceContainerHigh,
            ColorRole::OnSurface,
            WingsPaint::Role(ColorRole::OutlineVariant),
            320.0,
            48.0,
            16.0,
        ),
        _ => (
            ShapeRole::Full,
            ColorRole::SurfaceContainerLowest,
            ColorRole::OnSurface,
            WingsPaint::Role(ColorRole::OutlineVariant),
            200.0,
            if design == WingsDesign::MacOs {
                36.0
            } else {
                48.0
            },
            20.0,
        ),
    };
    let node = Node {
        min_width: px(min_width),
        max_width: px(560),
        min_height: px(height),
        padding: UiRect::axes(px(pad), px(4)),
        column_gap: px(8),
        border: UiRect::all(px(if border == WingsPaint::Transparent {
            0.0
        } else {
            1.0
        })),
        align_items: AlignItems::Center,
        justify_content: if apple {
            JustifyContent::Center
        } else {
            JustifyContent::SpaceBetween
        },
        ..default()
    };
    let glass: Box<dyn Scene> = if apple {
        Box::new(bsn! { crate::glass::WingsGlass })
    } else {
        Box::new(bsn! {})
    };
    bsn! {
        node
        snackbar
        visibility
        @{glass}
        WingsShape(shape)
        WingsElevationLevel(3)
        WingsBackground({WingsPaint::Role(container)})
        WingsBorderPaint({border})
        WingsContentColor({WingsPaint::Role(content)})
        Children [ @WingsText { @text: text, @role: TypeRole::BodyMedium } ]
        @{action}
    }
}

/// Spawns snackbars for queued messages. A host shows one snackbar at a time; a newer message
/// replaces the visible one.
pub fn show_snackbars(
    mut commands: Commands,
    skin: Res<WingsSkin>,
    mut messages: MessageReader<WingsShowSnackbar>,
    hosts: Query<(Entity, Option<&Children>), With<WingsSnackbarHost>>,
    mut visible: Query<(&mut WingsSnackbar, &mut WingsAnimatedVisibility)>,
) {
    let Some(msg) = messages.read().last() else {
        return;
    };
    for (host, children) in &hosts {
        for child in children.into_iter().flatten() {
            if let Ok((mut snack, mut vis)) = visible.get_mut(*child) {
                snack.remaining = 0.0;
                snack.exiting = Some(0.0);
                vis.visible = false;
            }
        }
        commands.spawn_scene(bsn! { @{snackbar_scene(msg, skin.design)} ChildOf(host) });
    }
}

/// Counts snackbars down, hides them and despawns them after their exit transition.
pub fn tick_snackbars(
    mut commands: Commands,
    time: Res<Time>,
    mut snacks: Query<(Entity, &mut WingsSnackbar, &mut WingsAnimatedVisibility)>,
) {
    let dt = time.delta_secs();
    for (entity, mut snack, mut vis) in &mut snacks {
        match snack.exiting {
            None => {
                snack.remaining -= dt;
                if snack.remaining <= 0.0 {
                    snack.exiting = Some(0.6);
                    vis.visible = false;
                }
            }
            Some(left) => {
                let left = left - dt;
                snack.exiting = Some(left);
                if left <= 0.0 {
                    commands.entity(entity).despawn();
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Dialog
// ---------------------------------------------------------------------------------------------

/// A modal dialog on a dimming scrim. Show and hide it by flipping
/// [`WingsAnimatedVisibility::visible`] on the dialog entity (for example through a view model
/// binding). Clicking a button triggers [`WingsDialogResult`] and hides the dialog.
///
/// ```ignore
/// bsn! {
///     #ConfirmDelete
///     @WingsDialog { @title: "Delete file?", @text: "This cannot be undone.", @confirm: "Delete" }
///     on(|r: On<WingsDialogResult>| if r.confirmed { /* delete */ })
/// }
/// ```
#[derive(SceneComponent, Default, Clone)]
#[scene(WingsDialogProps)]
pub struct WingsDialog;

/// Props of [`WingsDialog`].
#[derive(Clone)]
pub struct WingsDialogProps {
    /// Headline.
    pub title: String,
    /// Supporting text.
    pub text: String,
    /// Confirm button label.
    pub confirm: String,
    /// Dismiss button label. Empty for no dismiss button.
    pub dismiss: String,
    /// Start visible.
    pub open: bool,
    /// The confirm action destroys something (drawn in the error color).
    pub destructive: bool,
}

impl Default for WingsDialogProps {
    fn default() -> Self {
        Self {
            title: String::new(),
            text: String::new(),
            confirm: "OK".into(),
            dismiss: "Cancel".into(),
            open: false,
            destructive: false,
        }
    }
}

/// Triggered on a [`WingsDialog`] when one of its buttons was clicked.
#[derive(EntityEvent, Debug, Clone, Copy)]
pub struct WingsDialogResult {
    /// The dialog.
    pub entity: Entity,
    /// `true` for the confirm button.
    pub confirmed: bool,
}

/// Remembers which dialog button an entity is.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct WingsDialogButton {
    /// `true` for the confirm button.
    pub confirm: bool,
    /// `true` when confirming destroys something (drawn in the error color).
    pub destructive: bool,
}

/// The surface (panel) of a [`WingsDialog`].
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct WingsDialogSurface;

/// The title of a [`WingsDialog`].
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct WingsDialogTitle;

/// The message of a [`WingsDialog`].
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct WingsDialogMessage;

/// The button row of a [`WingsDialog`].
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct WingsDialogActions;

/// Dismisses open dialogs on back (Escape, gamepad B), like their dismiss button.
pub fn dismiss_dialogs_on_back(
    mut back: ResMut<bevy_wings_ui_core::back::WingsBack>,
    mut dialogs: Query<(Entity, &mut WingsAnimatedVisibility), With<WingsDialog>>,
    mut commands: Commands,
) {
    if !dialogs.iter().any(|(_, v)| v.visible) || !back.consume() {
        return;
    }
    for (entity, mut visibility) in &mut dialogs {
        if visibility.visible {
            visibility.visible = false;
            commands.trigger(WingsDialogResult {
                entity,
                confirmed: false,
            });
        }
    }
}

/// Observer: forwards dialog button clicks as [`WingsDialogResult`] and closes the dialog.
pub fn dialog_button_clicked(
    ev: On<Activate>,
    buttons: Query<&WingsDialogButton>,
    parents: Query<&ChildOf>,
    mut dialogs: Query<&mut WingsAnimatedVisibility, With<WingsDialog>>,
    mut commands: Commands,
) {
    let Ok(button) = buttons.get(ev.event_target()) else {
        return;
    };
    for ancestor in bevy_wings_ui_core::hierarchy::ancestors(ev.event_target(), &parents) {
        if let Ok(mut vis) = dialogs.get_mut(ancestor) {
            vis.visible = false;
            commands.trigger(WingsDialogResult {
                entity: ancestor,
                confirmed: button.confirm,
            });
            return;
        }
    }
}

impl WingsDialog {
    fn scene(props: WingsDialogProps) -> impl Scene {
        let title = props.title;
        let text = props.text;
        let confirm = props.confirm;
        let dismiss: Box<dyn Scene> = if props.dismiss.is_empty() {
            Box::new(bsn! {})
        } else {
            let label = props.dismiss;
            Box::new(bsn! {
                Children [
                    @WingsButton { @text: label, @variant: WingsButtonVariant::Text }
                    WingsDialogButton { confirm: false, destructive: false }
                    on(dialog_button_clicked)
                ]
            })
        };
        let visibility = WingsAnimatedVisibility {
            visible: props.open,
            enter: WingsTransition::fade(),
            exit: WingsTransition::fade(),
        };
        let confirm_button = WingsDialogButton {
            confirm: true,
            destructive: props.destructive,
        };
        let surface_motion = WingsAnimatedVisibility {
            visible: true,
            enter: WingsTransition::scale(0.9),
            exit: WingsTransition::none(),
        };
        bsn! {
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                right: px(0),
                top: px(0),
                bottom: px(0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
            }
            visibility
            GlobalZIndex(100)
            bevy_wings_ui_responsive::WingsSafeAreaPadding
            WingsBackground({WingsPaint::Role(ColorRole::Scrim).with_alpha(0.32)})
            Children [
                Node {
                    min_width: px(280),
                    max_width: px(560),
                    padding: px(24),
                    row_gap: px(16),
                    flex_direction: FlexDirection::Column,
                }
                surface_motion
                WingsDialogSurface
                WingsShape(ShapeRole::ExtraLarge)
                WingsElevationLevel(3)
                WingsBackground({WingsPaint::Role(ColorRole::SurfaceContainerHigh)})
                WingsBorderPaint
                WingsContentColor({WingsPaint::Role(ColorRole::OnSurface)})
                Children [
                    @WingsText { @text: title, @role: TypeRole::HeadlineSmall }
                    WingsDialogTitle
                    TextLayout
                    --
                    @WingsText { @text: text, @role: TypeRole::BodyMedium, @color: ColorRole::OnSurfaceVariant }
                    WingsDialogMessage
                    TextLayout
                    --
                    Node { justify_content: JustifyContent::End, column_gap: px(8) }
                    WingsDialogActions
                    @{dismiss}
                    Children [
                        @WingsButton { @text: confirm, @variant: WingsButtonVariant::Text }
                        confirm_button
                        on(dialog_button_clicked)
                    ]
                ]
            ]
        }
    }
}

/// Replays the surface pop in animation every time a dialog opens.
pub fn dialog_open_system(
    dialogs: Query<
        (&WingsAnimatedVisibility, &Children),
        (With<WingsDialog>, Changed<WingsAnimatedVisibility>),
    >,
    mut surfaces: Query<&mut WingsAnimatedVisibility, Without<WingsDialog>>,
) {
    for (vis, children) in &dialogs {
        for child in children.iter() {
            if let Ok(mut surface) = surfaces.get_mut(child) {
                // Toggle so the enter transition plays again in sync with the scrim.
                surface.visible = vis.visible;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Progress helpers.
    #[test]
    fn progress_fraction() {
        assert!(WingsProgress::default().is_indeterminate());
        assert_eq!(WingsProgress(0.25).fraction(), Some(0.25));
        assert_eq!(WingsProgress(3.0).fraction(), Some(1.0));
        assert_eq!(WingsProgress::INDETERMINATE.fraction(), None);
    }

    /// Badge counts are capped.
    #[test]
    fn badge_labels() {
        assert_eq!(badge_label(7), "7");
        assert_eq!(badge_label(1000), "999+");
    }

    /// Snackbar builder.
    #[test]
    fn snackbar_builder() {
        let s = WingsShowSnackbar::new("Saved")
            .with_action("Undo")
            .with_duration(2.0);
        assert_eq!(s.action.as_deref(), Some("Undo"));
        assert_eq!(s.duration, 2.0);
    }
}
