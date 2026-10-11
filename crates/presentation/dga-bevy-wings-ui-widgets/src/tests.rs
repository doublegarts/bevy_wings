//! Integration tests: widgets are spawned from `bsn!` scenes in a headless app and inspected
//! through their components, exactly like gameplay code would.

use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use bevy::ui::{Checked, InteractionDisabled, Pressed};
use bevy::ui_widgets::{Activate, SliderValue, ValueChange};
use core::time::Duration;

use dga_bevy_wings_ui_animation::prelude::WingsAnimatedVisibility;
use dga_bevy_wings_ui_core::WingsOpacity;
use dga_bevy_wings_ui_responsive::{WidthClass, WingsViewport, WingsViewportOverride};
use dga_bevy_wings_ui_theme::prelude::*;

use crate::feedback::{WingsProgressBar, WingsSnackbar};
use crate::prelude::*;
use crate::selection::{WingsCheckMarkProgress, WingsSliderFill};
use crate::text_field::{WingsTextFieldInput, WingsTextValue};

fn test_pointer(position: Vec2) -> bevy::picking::events::Pointer {
    bevy::picking::events::Pointer::new(
        bevy::picking::pointer::PointerId::Mouse,
        bevy::picking::pointer::Location {
            target: bevy::camera::NormalizedRenderTarget::None {
                width: 200,
                height: 44,
            },
            position,
        },
    )
}

/// A headless app with instant theme switches and a fixed 50 ms frame time.
fn app() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        bevy::scene::ScenePlugin,
        bevy::input::InputPlugin,
    ));
    app.init_asset::<Image>();
    // Only the headless widget plugins the tests need (the full group expects windows and IME).
    app.add_plugins((
        bevy::ui_widgets::ButtonPlugin,
        bevy::ui_widgets::CheckboxPlugin,
        bevy::ui_widgets::RadioGroupPlugin,
        bevy::ui_widgets::SliderPlugin,
    ));
    let mut theme = WingsTheme::default();
    theme.motion.theme_transition = None;
    app.add_plugins((
        WingsThemePlugin::with_theme(theme),
        WingsWidgetsPlugin::headless(),
    ));
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        50,
    )));
    app.update();
    app
}

fn spawn(app: &mut App, scene: impl Scene) -> Entity {
    let e = app.world_mut().spawn_scene(scene).unwrap().id();
    app.update();
    app.update();
    e
}

fn settle(app: &mut App, frames: usize) {
    for _ in 0..frames {
        app.update();
    }
}

fn children(app: &App, e: Entity) -> Vec<Entity> {
    app.world()
        .get::<Children>(e)
        .map(|c| c.iter().collect())
        .unwrap_or_default()
}

fn descendants_with<C: Component>(app: &mut App, root: Entity) -> Vec<Entity> {
    let mut out = Vec::new();
    let mut stack = vec![root];
    while let Some(e) = stack.pop() {
        if app.world().get::<C>(e).is_some() {
            out.push(e);
        }
        stack.extend(children(app, e));
    }
    out
}

fn color(app: &App, role: ColorRole) -> Color {
    app.world().resource::<WingsTheme>().color(role)
}

/// A filled button paints the primary role and its label inherits the matching `on` color.
#[test]
fn filled_button_uses_theme_roles() {
    let mut app = app();
    let b = spawn(&mut app, bsn! { @WingsButton { @text: "Save" } });
    assert_eq!(
        app.world().get::<BackgroundColor>(b).unwrap().0,
        color(&app, ColorRole::Primary)
    );
    let label = descendants_with::<Text>(&mut app, b)[0];
    assert_eq!(app.world().get::<Text>(label).unwrap().0, "Save");
    assert_eq!(
        app.world().get::<TextColor>(label).unwrap().0,
        color(&app, ColorRole::OnPrimary)
    );
}

/// The headline scenario: twenty buttons repaint when the theme flips to dark, without any
/// widget specific code.
#[test]
fn twenty_buttons_follow_dark_theme() {
    let mut app = app();
    let buttons: Vec<Entity> = (0..20)
        .map(|i| {
            let text = format!("Button {i}");
            app.world_mut()
                .spawn_scene(
                    bsn! { @WingsButton { @text: text, @variant: WingsButtonVariant::Elevated } },
                )
                .unwrap()
                .id()
        })
        .collect();
    settle(&mut app, 2);
    let light = color(&app, ColorRole::SurfaceContainerLow);
    for b in &buttons {
        assert_eq!(app.world().get::<BackgroundColor>(*b).unwrap().0, light);
    }
    app.world_mut().resource_mut::<WingsTheme>().toggle_dark();
    settle(&mut app, 1);
    let dark = color(&app, ColorRole::SurfaceContainerLow);
    assert_ne!(light, dark);
    for b in &buttons {
        assert_eq!(app.world().get::<BackgroundColor>(*b).unwrap().0, dark);
    }
}

/// Changing the variant component at runtime restyles the button.
#[test]
fn variant_change_restyles() {
    let mut app = app();
    let b = spawn(&mut app, bsn! { @WingsButton { @text: "Delete" } });
    *app.world_mut().get_mut::<WingsButtonVariant>(b).unwrap() = WingsButtonVariant::Danger;
    settle(&mut app, 20);
    assert_eq!(
        app.world().get::<WingsBackground>(b).unwrap().0,
        WingsPaint::Role(ColorRole::Error)
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(b).unwrap().0,
        color(&app, ColorRole::Error)
    );
}

/// Pressed sets the pressed state layer; disabled dims the widget to 38 %.
#[test]
fn interaction_states() {
    let mut app = app();
    let b = spawn(&mut app, bsn! { @WingsButton { @text: "Go" } });
    app.world_mut().entity_mut(b).insert(Pressed);
    app.update();
    assert_eq!(
        app.world().get::<WingsStateLayer>(b).unwrap().opacity,
        WingsStateLayer::PRESSED
    );
    app.world_mut().entity_mut(b).remove::<Pressed>();
    app.update();
    assert_eq!(app.world().get::<WingsStateLayer>(b).unwrap().opacity, 0.0);

    let d = spawn(
        &mut app,
        bsn! { @WingsButton { @text: "Off", @enabled: false } },
    );
    assert!(app.world().get::<InteractionDisabled>(d).is_some());
    assert_eq!(app.world().get::<WingsOpacity>(d).unwrap().0, 0.38);
}

/// A checkbox traces its short stroke before the long stroke and erases in reverse.
#[test]
fn checkbox_toggles_and_shows_mark() {
    let mut app = app();
    let cb = spawn(&mut app, bsn! { @WingsCheckbox { @label: "Remember me" } });
    let mark = descendants_with::<WingsCheckMarkProgress>(&mut app, cb)[0];
    let strokes = children(&app, mark);
    assert_eq!(strokes.len(), 2);
    assert_eq!(
        app.world().get::<WingsCheckMarkProgress>(mark).unwrap().0,
        0.0
    );
    for stroke in &strokes {
        assert_eq!(app.world().get::<Node>(*stroke).unwrap().width, px(0));
        assert_eq!(
            app.world().get::<BackgroundColor>(*stroke).unwrap().0,
            color(&app, ColorRole::OnPrimary)
        );
    }

    // Early progress reveals only the first leg and keeps its starting point fixed.
    app.world_mut()
        .get_mut::<WingsCheckMarkProgress>(mark)
        .unwrap()
        .0 = 0.15;
    app.update();
    let short = app.world().get::<Node>(strokes[0]).unwrap();
    let Val::Px(width) = short.width else {
        panic!("expected pixel width")
    };
    assert!(width > 0.0 && width < Vec2::new(3.0, 3.0).length());
    assert_eq!(app.world().get::<Node>(strokes[1]).unwrap().width, px(0));
    let Val::Px(left) = short.left else {
        panic!("expected pixel left")
    };
    let Val::Px(top) = short.top else {
        panic!("expected pixel top")
    };
    let rotation = app.world().get::<UiTransform>(strokes[0]).unwrap().rotation;
    let start = Vec2::new(left + width * 0.5, top + 1.0) - rotation * Vec2::new(width * 0.5, 0.0);
    assert!(start.distance(Vec2::new(2.0, 7.0)) < 0.001);
    app.world_mut()
        .get_mut::<WingsCheckMarkProgress>(mark)
        .unwrap()
        .0 = 0.0;
    app.update();

    app.world_mut().trigger(ValueChange {
        source: cb,
        value: true,
        is_final: true,
    });
    settle(&mut app, 2);
    assert!(app.world().get::<Checked>(cb).is_some());
    let entering = app.world().get::<WingsCheckMarkProgress>(mark).unwrap().0;
    assert!(
        entering > 0.0 && entering < 1.0,
        "path should still be drawing: {entering}"
    );
    settle(&mut app, 8);
    assert_eq!(
        app.world().get::<WingsCheckMarkProgress>(mark).unwrap().0,
        1.0
    );
    let Val::Px(short) = app.world().get::<Node>(strokes[0]).unwrap().width else {
        panic!("expected pixels")
    };
    let Val::Px(long) = app.world().get::<Node>(strokes[1]).unwrap().width else {
        panic!("expected pixels")
    };
    assert!((short - Vec2::new(3.0, 3.0).length()).abs() < 0.001);
    assert!((long - Vec2::new(7.0, -7.0).length()).abs() < 0.001);

    app.world_mut().trigger(ValueChange {
        source: cb,
        value: false,
        is_final: true,
    });
    settle(&mut app, 2);
    assert!(app.world().get::<Checked>(cb).is_none());
    let leaving = app.world().get::<WingsCheckMarkProgress>(mark).unwrap().0;
    assert!(
        leaving > 0.0 && leaving < 1.0,
        "path should erase gradually: {leaving}"
    );
    // Reverse direction before completion; continue from the current path position.
    app.world_mut().trigger(ValueChange {
        source: cb,
        value: true,
        is_final: true,
    });
    settle(&mut app, 8);
    assert_eq!(
        app.world().get::<WingsCheckMarkProgress>(mark).unwrap().0,
        1.0
    );
    app.world_mut().trigger(ValueChange {
        source: cb,
        value: false,
        is_final: true,
    });
    settle(&mut app, 8);
    for stroke in strokes {
        assert_eq!(app.world().get::<Node>(stroke).unwrap().width, px(0));
    }

    let checked = spawn(&mut app, bsn! { @WingsCheckbox { @checked: true } });
    let mark = descendants_with::<WingsCheckMarkProgress>(&mut app, checked)[0];
    assert_eq!(
        app.world().get::<WingsCheckMarkProgress>(mark).unwrap().0,
        1.0
    );
}

/// A switch spawned checked snaps its thumb to the end without animating.
#[test]
fn switch_thumb_snaps_on_spawn() {
    let mut app = app();
    let sw = spawn(&mut app, bsn! { @WingsSwitch { @checked: true } });
    let thumb = descendants_with::<WingsCheckedPart>(&mut app, sw)
        .into_iter()
        .find(|p| {
            app.world()
                .get::<WingsCheckedPart>(*p)
                .unwrap()
                .translate
                .is_some()
        })
        .unwrap();
    let t = app.world().get::<UiTransform>(thumb).unwrap();
    assert_eq!(t.translation.x, Val::Px(20.0));
    assert_eq!(t.scale, Vec2::ONE);
}

/// Unchecking a switch animates the thumb back to the start.
#[test]
fn switch_thumb_animates_back() {
    let mut app = app();
    let sw = spawn(&mut app, bsn! { @WingsSwitch { @checked: true } });
    app.world_mut().entity_mut(sw).remove::<Checked>();
    settle(&mut app, 60);
    let thumb = descendants_with::<WingsCheckedPart>(&mut app, sw)
        .into_iter()
        .find(|p| {
            app.world()
                .get::<WingsCheckedPart>(*p)
                .unwrap()
                .translate
                .is_some()
        })
        .unwrap();
    let t = app.world().get::<UiTransform>(thumb).unwrap();
    assert_eq!(t.translation.x, Val::Px(0.0));
}

/// The slider fill tracks the value.
#[test]
fn slider_fill_follows_value() {
    let mut app = app();
    let s = spawn(
        &mut app,
        bsn! { @WingsSlider { @value: 25.0, @min: 0.0, @max: 100.0 } },
    );
    let fill = descendants_with::<WingsSliderFill>(&mut app, s)[0];
    assert_eq!(
        app.world().get::<Node>(fill).unwrap().width,
        Val::Percent(25.0)
    );
    app.world_mut().entity_mut(s).insert(SliderValue(75.0));
    app.update();
    assert_eq!(
        app.world().get::<Node>(fill).unwrap().width,
        Val::Percent(75.0)
    );
}

/// Pressing the track moves the value, fill and thumb without a drag; disabled and thumb
/// presses do not jump. Pointer events bubble from the visual track to the slider root.
#[test]
fn slider_track_click_moves_thumb() {
    use bevy::picking::{backend::HitData, events::PointerPress, pointer::PointerButton};
    use bevy::ui::{ComputedNode, ComputedUiRenderTargetInfo, UiGlobalTransform};
    use bevy::ui_widgets::SliderThumb;
    let mut app = app();
    app.init_resource::<bevy::ui::UiScale>();
    let slider = spawn(
        &mut app,
        bsn! { @WingsSlider { @value: 15.0, @min: 10.0, @max: 30.0 } },
    );
    let thumb = descendants_with::<SliderThumb>(&mut app, slider)[0];
    let fill = descendants_with::<WingsSliderFill>(&mut app, slider)[0];
    app.world_mut().entity_mut(slider).insert((
        ComputedNode {
            size: Vec2::new(200.0, 44.0),
            ..default()
        },
        ComputedUiRenderTargetInfo::default(),
        UiGlobalTransform::from_xy(100.0, 22.0),
    ));
    app.world_mut().entity_mut(thumb).insert(ComputedNode {
        size: Vec2::splat(20.0),
        ..default()
    });
    let press = |app: &mut App, target: Entity, x: f32| {
        app.world_mut().trigger(PointerPress {
            entity: target,
            pointer: test_pointer(Vec2::new(x, 22.0)),
            button: PointerButton::Primary,
            hit: HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
            count: 1,
        });
        app.update();
    };
    for (x, value, percent) in [(145.0, 25.0, 75.0), (10.0, 10.0, 0.0), (190.0, 30.0, 100.0)] {
        press(&mut app, fill, x);
        assert_eq!(app.world().get::<SliderValue>(slider).unwrap().0, value);
        assert_eq!(
            app.world().get::<Node>(fill).unwrap().width,
            Val::Percent(percent)
        );
        assert_eq!(
            app.world().get::<Node>(thumb).unwrap().left,
            Val::Percent(percent)
        );
        assert_eq!(
            app.world().get::<Node>(thumb).unwrap().margin.left,
            px(-percent * 0.2)
        );
    }
    press(&mut app, thumb, 55.0);
    assert_eq!(app.world().get::<SliderValue>(slider).unwrap().0, 30.0);
    app.world_mut()
        .entity_mut(slider)
        .insert(InteractionDisabled);
    press(&mut app, fill, 55.0);
    assert_eq!(app.world().get::<SliderValue>(slider).unwrap().0, 30.0);
}

/// Clicking a radio label selects that option and clears the other option in its group.
#[test]
fn radio_group_selects_one_option() {
    use bevy::picking::{backend::HitData, events::PointerClick, pointer::PointerButton};
    let mut app = app();
    let group = spawn(
        &mut app,
        bsn! {
            @WingsRadioGroup { @gap: 8.0 }
            Children [
                @WingsRadioButton { @label: "A", @checked: true }
                --
                @WingsRadioButton { @label: "B" }
            ]
        },
    );
    let options = children(&app, group);
    assert!(app.world().get::<Checked>(options[0]).is_some());
    let label = descendants_with::<Text>(&mut app, options[1])[0];
    app.world_mut().trigger(PointerClick {
        entity: label,
        pointer: test_pointer(Vec2::ZERO),
        button: PointerButton::Primary,
        hit: HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
        duration: Duration::from_millis(10),
        count: 1,
    });
    app.update();
    assert!(app.world().get::<Checked>(options[0]).is_none());
    assert!(app.world().get::<Checked>(options[1]).is_some());
}

#[derive(Resource, Default)]
struct Selected(Option<usize>);

/// Activating a navigation item selects it, deselects the others and reports its index.
#[test]
fn navigation_items_are_exclusive() {
    let mut app = app();
    app.init_resource::<Selected>();
    let bar = spawn(
        &mut app,
        bsn! {
            @WingsNavigationBar
            on(|ev: On<WingsNavigationSelected>, mut s: ResMut<Selected>| s.0 = Some(ev.index))
            Children [
                @WingsNavigationItem { @label: "Home", @selected: true }
                --
                @WingsNavigationItem { @label: "Search" }
                --
                @WingsNavigationItem { @label: "Profile" }
            ]
        },
    );
    let items = children(&app, bar);
    assert!(app.world().get::<Checked>(items[0]).is_some());
    app.world_mut().trigger(Activate { entity: items[2] });
    settle(&mut app, 2);
    assert!(app.world().get::<Checked>(items[0]).is_none());
    assert!(app.world().get::<Checked>(items[2]).is_some());
    assert_eq!(app.world().resource::<Selected>().0, Some(2));
}

/// Navigation bar and rail swap with the window width class.
#[test]
fn navigation_bar_and_rail_are_responsive() {
    let mut app = app();
    app.insert_resource(WingsViewportOverride {
        width: 400.0,
        height: 800.0,
    });
    let bar = spawn(&mut app, bsn! { @WingsNavigationBar });
    let rail = spawn(&mut app, bsn! { @WingsNavigationRail });
    assert_eq!(
        app.world().resource::<WingsViewport>().width_class,
        WidthClass::Compact
    );
    assert_ne!(app.world().get::<Node>(bar).unwrap().display, Display::None);
    assert_eq!(
        app.world().get::<Node>(rail).unwrap().display,
        Display::None
    );

    app.insert_resource(WingsViewportOverride {
        width: 1300.0,
        height: 800.0,
    });
    app.update();
    assert_eq!(app.world().get::<Node>(bar).unwrap().display, Display::None);
    assert_ne!(
        app.world().get::<Node>(rail).unwrap().display,
        Display::None
    );
}

#[derive(Resource, Default)]
struct DialogAnswer(Option<bool>);

/// Clicking the confirm button reports the result and closes the dialog.
#[test]
fn dialog_confirm_closes() {
    let mut app = app();
    app.init_resource::<DialogAnswer>();
    let dialog = spawn(
        &mut app,
        bsn! {
            @WingsDialog { @title: "Delete?", @text: "Gone forever.", @confirm: "Delete", @open: true }
            on(|r: On<WingsDialogResult>, mut a: ResMut<DialogAnswer>| a.0 = Some(r.confirmed))
        },
    );
    let confirm = descendants_with::<crate::feedback::WingsDialogButton>(&mut app, dialog)
        .into_iter()
        .find(|b| {
            app.world()
                .get::<crate::feedback::WingsDialogButton>(*b)
                .unwrap()
                .confirm
        })
        .unwrap();
    app.world_mut().trigger(Activate { entity: confirm });
    settle(&mut app, 30);
    assert_eq!(app.world().resource::<DialogAnswer>().0, Some(true));
    assert!(
        !app.world()
            .get::<WingsAnimatedVisibility>(dialog)
            .unwrap()
            .visible
    );
    assert_eq!(
        app.world().get::<Node>(dialog).unwrap().display,
        Display::None
    );
}

/// A snackbar appears in the scaffold's host and disappears after its duration.
#[test]
fn snackbar_lifecycle() {
    let mut app = app();
    let scaffold = spawn(&mut app, bsn! { @WingsScaffold });
    let host = descendants_with::<WingsSnackbarHost>(&mut app, scaffold)[0];
    app.world_mut()
        .write_message(WingsShowSnackbar::new("Saved").with_duration(0.5));
    settle(&mut app, 2);
    assert_eq!(children(&app, host).len(), 1);
    let snack = children(&app, host)[0];
    assert!(app.world().get::<WingsSnackbar>(snack).is_some());
    settle(&mut app, 40);
    assert!(children(&app, host).is_empty());
}

/// Determinate progress animates to its width; indeterminate switches to the sweep.
#[test]
fn linear_progress_modes() {
    let mut app = app();
    let p = spawn(&mut app, bsn! { @WingsLinearProgress { @progress: 0.5 } });
    settle(&mut app, 30);
    let bar = descendants_with::<WingsProgressBar>(&mut app, p)[0];
    assert_eq!(
        app.world().get::<Node>(bar).unwrap().width,
        Val::Percent(50.0)
    );
    app.world_mut()
        .entity_mut(p)
        .insert(WingsProgress::INDETERMINATE);
    settle(&mut app, 3);
    assert_eq!(
        app.world().get::<Node>(bar).unwrap().width,
        Val::Percent(crate::feedback::SWEEP_WIDTH)
    );
}

/// A text field starts with its value and highlights its outline when focused.
#[test]
fn text_field_value_and_focus() {
    let mut app = app();
    let f = spawn(
        &mut app,
        bsn! { @WingsTextField { @label: "Name", @value: "Ada" } },
    );
    assert_eq!(app.world().get::<WingsTextValue>(f).unwrap().0, "Ada");
    let input = descendants_with::<WingsTextFieldInput>(&mut app, f)[0];
    assert_eq!(
        app.world()
            .get::<bevy::text::EditableText>(input)
            .unwrap()
            .value()
            .to_string(),
        "Ada"
    );
    app.world_mut()
        .resource_mut::<bevy::input_focus::InputFocus>()
        .set(input, bevy::input_focus::FocusCause::Navigated);
    app.update();
    assert_eq!(
        app.world().get::<WingsBorderPaint>(f).unwrap().0,
        WingsPaint::Role(ColorRole::Primary)
    );
}

/// Layout containers translate their props into the node.
#[test]
fn layout_props() {
    let mut app = app();
    let col = spawn(
        &mut app,
        bsn! {
            @WingsColumn { @gap: 8.0, @padding: 4.0 }
            Children [ @WingsText { @text: "a" } -- @WingsText { @text: "b" } ]
        },
    );
    let node = app.world().get::<Node>(col).unwrap();
    assert_eq!(node.flex_direction, FlexDirection::Column);
    assert_eq!(node.row_gap, Val::Px(8.0));
    assert_eq!(node.align_items, AlignItems::Stretch);
    assert_eq!(children(&app, col).len(), 2);

    let row = spawn(&mut app, bsn! { @WingsRow { @gap: 2.0 } });
    assert_eq!(
        app.world().get::<Node>(row).unwrap().align_items,
        AlignItems::Center
    );

    let card = spawn(
        &mut app,
        bsn! { @WingsCard { @style: WingsCardStyle::Outlined } },
    );
    assert_eq!(
        app.world().get::<WingsBorderPaint>(card).unwrap().0,
        WingsPaint::Role(ColorRole::OutlineVariant)
    );
}

/// Without shader materials the skeleton pulses its opacity.
#[test]
fn skeleton_fallback_pulses() {
    let mut app = app();
    let s = spawn(&mut app, bsn! { @WingsSkeleton });
    settle(&mut app, 8);
    let o = app.world().get::<WingsOpacity>(s).unwrap().0;
    assert!(o < 1.0, "opacity {o}");
}

/// Badge labels and dots.
#[test]
fn badges() {
    let mut app = app();
    let b = spawn(&mut app, bsn! { @WingsBadge { @count: 1200 } });
    let t = descendants_with::<Text>(&mut app, b)[0];
    assert_eq!(app.world().get::<Text>(t).unwrap().0, "999+");
    let dot = spawn(&mut app, bsn! { @WingsBadge });
    assert!(descendants_with::<Text>(&mut app, dot).is_empty());
}
