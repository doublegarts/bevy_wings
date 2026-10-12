//! Animated appearance and disappearance of UI subtrees.
//!
//! Insert [`WingsAnimatedVisibility`] on a node and flip its `visible` flag: Wings plays the
//! configured enter or exit [`WingsTransition`] (fade, scale, slide or any combination) and
//! toggles [`Display::None`] at the right moment so hidden nodes do not take layout space.

use bevy::prelude::*;
use bevy::ui::{Display, Node, UiTransform};

use bevy_wings_ui_core::WingsOpacity;

use crate::property::{
    OpacityProperty, ScaleProperty, TranslationProperty, WingsAnimator, request_on,
};
use crate::spec::{AnimationSpec, Repeat};

/// A combinable enter or exit transition.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WingsTransition {
    /// Fade opacity from (enter) or to (exit) zero.
    pub fade: bool,
    /// Scale from (enter) or to (exit) this factor.
    pub scale: Option<f32>,
    /// Slide from (enter) or to (exit) this pixel offset.
    pub slide: Option<Vec2>,
    /// Timing.
    pub spec: AnimationSpec,
}

impl Default for WingsTransition {
    fn default() -> Self {
        Self::fade()
    }
}

impl WingsTransition {
    /// No visual transition, the node simply appears or disappears.
    pub fn none() -> Self {
        Self {
            fade: false,
            scale: None,
            slide: None,
            spec: AnimationSpec::Snap,
        }
    }

    /// Fade in or out.
    pub fn fade() -> Self {
        Self {
            fade: true,
            scale: None,
            slide: None,
            spec: AnimationSpec::medium(),
        }
    }

    /// Scale from or to `factor`.
    pub fn scale(factor: f32) -> Self {
        Self {
            fade: false,
            scale: Some(factor),
            slide: None,
            spec: AnimationSpec::medium(),
        }
    }

    /// Slide from or to the given pixel offset.
    pub fn slide(offset: Vec2) -> Self {
        Self {
            fade: false,
            scale: None,
            slide: Some(offset),
            spec: AnimationSpec::medium(),
        }
    }

    /// Slide vertically from or to `dy` pixels.
    pub fn slide_vertically(dy: f32) -> Self {
        Self::slide(Vec2::new(0.0, dy))
    }

    /// Slide horizontally from or to `dx` pixels.
    pub fn slide_horizontally(dx: f32) -> Self {
        Self::slide(Vec2::new(dx, 0.0))
    }

    /// Combines two transitions (the timing of `self` wins).
    pub fn and(self, other: Self) -> Self {
        Self {
            fade: self.fade || other.fade,
            scale: self.scale.or(other.scale),
            slide: self.slide.or(other.slide),
            spec: self.spec,
        }
    }

    /// Replaces the timing.
    pub fn with_spec(mut self, spec: AnimationSpec) -> Self {
        self.spec = spec;
        self
    }

    /// Starts this transition as an *enter* on the entity.
    pub fn play_enter(&self, entity: &mut EntityWorldMut) {
        if self.fade {
            request_on::<OpacityProperty>(entity, Some(0.0), 1.0, self.spec, Repeat::ONCE);
        }
        if let Some(s) = self.scale {
            request_on::<ScaleProperty>(
                entity,
                Some(Vec2::splat(s)),
                Vec2::ONE,
                self.spec,
                Repeat::ONCE,
            );
        }
        if let Some(o) = self.slide {
            request_on::<TranslationProperty>(entity, Some(o), Vec2::ZERO, self.spec, Repeat::ONCE);
        }
    }

    /// Starts this transition as an *exit* on the entity.
    pub fn play_exit(&self, entity: &mut EntityWorldMut) {
        if self.fade {
            request_on::<OpacityProperty>(entity, None, 0.0, self.spec, Repeat::ONCE);
        }
        if let Some(s) = self.scale {
            request_on::<ScaleProperty>(entity, None, Vec2::splat(s), self.spec, Repeat::ONCE);
        }
        if let Some(o) = self.slide {
            request_on::<TranslationProperty>(entity, None, o, self.spec, Repeat::ONCE);
        }
    }
}

/// Animated visibility for a UI subtree.
#[derive(Component, Debug, Clone, PartialEq)]
#[require(Node, WingsOpacity, UiTransform)]
pub struct WingsAnimatedVisibility {
    /// Whether the subtree should be visible.
    pub visible: bool,
    /// Transition played when becoming visible.
    pub enter: WingsTransition,
    /// Transition played when becoming hidden.
    pub exit: WingsTransition,
}

impl Default for WingsAnimatedVisibility {
    fn default() -> Self {
        Self {
            visible: true,
            enter: WingsTransition::fade(),
            exit: WingsTransition::fade(),
        }
    }
}

impl WingsAnimatedVisibility {
    /// Visible with fade transitions.
    pub fn new(visible: bool) -> Self {
        Self {
            visible,
            ..Default::default()
        }
    }

    /// Sets the enter transition.
    pub fn with_enter(mut self, enter: WingsTransition) -> Self {
        self.enter = enter;
        self
    }

    /// Sets the exit transition.
    pub fn with_exit(mut self, exit: WingsTransition) -> Self {
        self.exit = exit;
        self
    }
}

/// Internal bookkeeping for [`WingsAnimatedVisibility`].
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct WingsVisibilityState {
    /// The display mode to restore when shown.
    pub shown_display: Display,
    /// True while an exit transition plays.
    pub hiding: bool,
    /// Last applied `visible` value.
    pub applied: bool,
}

/// Reacts to visibility flips and starts transitions.
pub fn start_visibility_transitions(
    mut commands: Commands,
    mut query: Query<
        (
            Entity,
            &WingsAnimatedVisibility,
            &mut Node,
            Option<&mut WingsVisibilityState>,
        ),
        Changed<WingsAnimatedVisibility>,
    >,
) {
    for (entity, vis, mut node, state) in &mut query {
        match state {
            None => {
                // First sight: apply the initial state without animation.
                let shown_display = if node.display == Display::None {
                    Display::Flex
                } else {
                    node.display
                };
                if !vis.visible {
                    node.display = Display::None;
                }
                commands.entity(entity).insert(WingsVisibilityState {
                    shown_display,
                    hiding: false,
                    applied: vis.visible,
                });
            }
            Some(mut state) => {
                if state.applied == vis.visible {
                    continue;
                }
                state.applied = vis.visible;
                if vis.visible {
                    state.hiding = false;
                    node.display = state.shown_display;
                    let enter = vis.enter;
                    commands.entity(entity).queue(move |mut e: EntityWorldMut| {
                        enter.play_enter(&mut e);
                    });
                } else {
                    state.hiding = true;
                    let exit = vis.exit;
                    commands.entity(entity).queue(move |mut e: EntityWorldMut| {
                        exit.play_exit(&mut e);
                    });
                }
            }
        }
    }
}

/// Collapses nodes whose exit transition completed.
pub fn finish_visibility_transitions(
    mut query: Query<(
        &mut WingsVisibilityState,
        &mut Node,
        Option<&WingsAnimator<OpacityProperty>>,
        Option<&WingsAnimator<ScaleProperty>>,
        Option<&WingsAnimator<TranslationProperty>>,
    )>,
) {
    for (mut state, mut node, a, b, c) in &mut query {
        if !state.hiding {
            continue;
        }
        let busy = a.is_some_and(|a| a.is_active())
            || b.is_some_and(|b| b.is_active())
            || c.is_some_and(|c| c.is_active());
        if !busy {
            state.hiding = false;
            node.display = Display::None;
        }
    }
}
