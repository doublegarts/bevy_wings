//! Integration tests: widgets are spawned from `bsn!` scenes in a headless app and inspected
//! through their components, exactly like gameplay code would.

use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use bevy::ui::{Checked, InteractionDisabled, Pressed};
use bevy::ui_widgets::{Activate, SliderValue, ValueChange};
use core::time::Duration;

use bevy_wings_ui_animation::prelude::WingsAnimatedVisibility;
use bevy_wings_ui_core::WingsOpacity;
use bevy_wings_ui_responsive::{
    WidthClass, WingsDevicePreview, WingsDeviceProfile, WingsViewport, WingsViewportOverride,
};
use bevy_wings_ui_theme::prelude::*;

use crate::feedback::{WingsProgressBar, WingsSnackbar};
use crate::prelude::*;
use crate::selection::{WingsCheckMarkProgress, WingsSliderFill};
use crate::skin::WingsSkinPart;
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

/// A headless Material app with instant theme switches and a fixed 50 ms frame time.
fn app() -> App {
    app_with(WingsDesign::Material)
}

/// A headless app in a given design.
fn app_with(design: WingsDesign) -> App {
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
    let mut theme = WingsTheme::for_design(design);
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

// ---------------------------------------------------------------------------------------------
// Designs: Material, iOS, macOS and Fluent skins
// ---------------------------------------------------------------------------------------------

fn set_design(app: &mut App, design: WingsDesign) {
    let mut theme = app.world_mut().resource_mut::<WingsTheme>();
    theme.set_design(design);
    theme.motion.theme_transition = None;
    settle(app, 2);
}

/// One button spawned in Material takes the size of every design when the design changes at
/// runtime, and comes back to Material.
#[test]
fn design_switch_reskins_buttons() {
    let mut app = app();
    let b = spawn(&mut app, bsn! { @WingsButton { @text: "Save" } });
    let height = |app: &App| app.world().get::<Node>(b).unwrap().min_height;
    assert_eq!(height(&app), px(40));
    for (design, h) in [
        (WingsDesign::Ios, 34.0),
        (WingsDesign::MacOs, 24.0),
        (WingsDesign::Fluent, 32.0),
        (WingsDesign::Material, 40.0),
    ] {
        set_design(&mut app, design);
        assert_eq!(height(&app), px(h), "{design:?}");
        assert_eq!(
            app.world().get::<BackgroundColor>(b).unwrap().0,
            color(&app, ColorRole::Primary),
            "{design:?}"
        );
    }
    // Material corners are full, macOS corners are small.
    set_design(&mut app, WingsDesign::MacOs);
    assert_eq!(
        app.world().get::<WingsShape>(b).unwrap().0,
        ShapeRole::Small
    );
}

/// The iOS outlined button is the gray `.bordered` style with a tinted label.
#[test]
fn ios_bordered_button() {
    let mut app = app_with(WingsDesign::Ios);
    let b = spawn(
        &mut app,
        bsn! { @WingsButton { @text: "Edit", @variant: WingsButtonVariant::Outlined } },
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(b).unwrap().0,
        color(&app, ColorRole::SecondaryFill)
    );
    let label = descendants_with::<Text>(&mut app, b)[0];
    assert_eq!(
        app.world().get::<TextColor>(label).unwrap().0,
        color(&app, ColorRole::Primary)
    );
}

/// iOS controls dim while pressed instead of showing a state layer.
#[test]
fn ios_press_dims() {
    let mut app = app_with(WingsDesign::Ios);
    let b = spawn(&mut app, bsn! { @WingsButton { @text: "Go" } });
    app.world_mut().entity_mut(b).insert(Pressed);
    settle(&mut app, 4);
    assert_eq!(app.world().get::<WingsStateLayer>(b).unwrap().opacity, 0.0);
    assert!(app.world().get::<WingsOpacity>(b).unwrap().0 < 0.7);
    app.world_mut().entity_mut(b).remove::<Pressed>();
    settle(&mut app, 10);
    assert!((app.world().get::<WingsOpacity>(b).unwrap().0 - 1.0).abs() < 1e-3);
}

/// The iOS switch is a 61 by 28 green capsule with a white pill thumb.
#[test]
fn ios_switch_style() {
    let mut app = app_with(WingsDesign::Ios);
    let s = spawn(&mut app, bsn! { @WingsSwitch { @checked: true } });
    let track = descendants_with::<crate::selection::WingsSwitchTrack>(&mut app, s)[0];
    let thumb = descendants_with::<crate::selection::WingsSwitchThumb>(&mut app, s)[0];
    let node = app.world().get::<Node>(track).unwrap();
    assert_eq!((node.width, node.height), (px(61), px(28)));
    assert_eq!(
        app.world().get::<BackgroundColor>(track).unwrap().0,
        color(&app, ColorRole::Success)
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(thumb).unwrap().0,
        Color::WHITE
    );
    assert_eq!(app.world().get::<Node>(thumb).unwrap().width, px(37));
    assert_eq!(
        app.world().get::<UiTransform>(thumb).unwrap().translation,
        Val2::px(20.0, 0.0)
    );
}

/// iOS radio options become rows with a trailing checkmark; switching to macOS brings back
/// the radio ring and removes the checkmark.
#[test]
fn ios_radio_rows() {
    let mut app = app_with(WingsDesign::Ios);
    let group = spawn(
        &mut app,
        bsn! {
            @WingsRadioGroup
            Children [
                @WingsRadioButton { @label: "Small", @checked: true }
                --
                @WingsRadioButton { @label: "Large" }
            ]
        },
    );
    let radios = children(&app, group);
    let ring = descendants_with::<crate::selection::WingsRadioRing>(&mut app, radios[0])[0];
    assert_eq!(
        app.world().get::<Node>(ring).unwrap().display,
        Display::None
    );
    assert_eq!(
        descendants_with::<WingsSkinPart>(&mut app, radios[0]).len(),
        1,
        "trailing checkmark"
    );
    set_design(&mut app, WingsDesign::MacOs);
    assert_eq!(
        app.world().get::<Node>(ring).unwrap().display,
        Display::Flex
    );
    assert_eq!(app.world().get::<Node>(ring).unwrap().width, px(14));
    assert!(descendants_with::<WingsSkinPart>(&mut app, radios[0]).is_empty());
}

/// macOS and Fluent hide the bottom navigation bar and always show the sidebar.
#[test]
fn desktop_designs_use_sidebars() {
    for design in [WingsDesign::MacOs, WingsDesign::Fluent] {
        let mut app = app_with(design);
        app.insert_resource(WingsViewportOverride {
            width: 400.0,
            height: 800.0,
        });
        let bar = spawn(&mut app, bsn! { @WingsNavigationBar });
        let rail = spawn(
            &mut app,
            bsn! {
                @WingsNavigationRail
                Children [ @WingsNavigationItem { @label: "Home", @selected: true } ]
            },
        );
        settle(&mut app, 2);
        assert_eq!(
            app.world().get::<Node>(bar).unwrap().display,
            Display::None,
            "{design:?}"
        );
        assert_ne!(
            app.world().get::<Node>(rail).unwrap().display,
            Display::None,
            "{design:?}"
        );
        // Narrow window: the sidebar collapses to its icons.
        let label = descendants_with::<crate::scaffold::WingsNavigationLabel>(&mut app, rail)[0];
        let width = |app: &App| app.world().get::<Node>(rail).unwrap().width;
        let label_display = |app: &App| app.world().get::<Node>(label).unwrap().display;
        assert_eq!(
            width(&app),
            px(crate::scaffold_skin::sidebar_width(design, true))
        );
        assert_eq!(label_display(&app), Display::None, "{design:?}");
        // Wide window: full sidebar with labels.
        app.insert_resource(WingsViewportOverride {
            width: 1200.0,
            height: 800.0,
        });
        settle(&mut app, 3);
        assert_eq!(
            width(&app),
            px(crate::scaffold_skin::sidebar_width(design, false))
        );
        assert_eq!(label_display(&app), Display::Flex, "{design:?}");
    }
}

/// iOS root screens show a large title; screens with a back button show an inline one.
#[test]
fn ios_large_titles() {
    let mut app = app_with(WingsDesign::Ios);
    let root = spawn(&mut app, bsn! { @WingsTopAppBar { @title: "Inbox" } });
    let pushed = spawn(
        &mut app,
        bsn! { @WingsTopAppBar { @title: "Mail", @navigation_icon: "back.png" } },
    );
    settle(&mut app, 2);
    assert_eq!(app.world().get::<Node>(root).unwrap().height, px(104));
    assert_eq!(app.world().get::<Node>(pushed).unwrap().height, px(52));
    let title = descendants_with::<crate::scaffold::WingsTopAppBarTitle>(&mut app, root)[0];
    assert_eq!(
        app.world().get::<WingsTypeStyle>(title).unwrap().0,
        TypeRole::DisplayLarge
    );
}

/// Grouped sections: header first, footer last, rounded first and last rows and inset
/// separators between rows.
#[test]
fn grouped_list_sections() {
    let mut app = app_with(WingsDesign::Ios);
    let list = spawn(
        &mut app,
        bsn! {
            @WingsList
            Children [
                @WingsListSection { @header: "Network", @footer: "Updates" }
                Children [
                    @WingsListItem { @headline: "Wi-Fi", @accessory: WingsListAccessory::Chevron }
                    --
                    @WingsListItem { @headline: "Bluetooth", @detail: "On" }
                    --
                    @WingsListItem { @headline: "VPN" }
                ]
            ]
        },
    );
    settle(&mut app, 3);
    let section = children(&app, list)[0];
    let kids = children(&app, section);
    assert_eq!(kids.len(), 5);
    assert!(
        app.world()
            .get::<crate::list::WingsListSectionHeader>(kids[0])
            .is_some()
    );
    assert!(
        app.world()
            .get::<crate::list::WingsListSectionFooter>(kids[4])
            .is_some()
    );
    let first = app.world().get::<Node>(kids[1]).unwrap().border_radius;
    let middle = app.world().get::<Node>(kids[2]).unwrap().border_radius;
    assert_eq!(first.top_left, px(26).into());
    assert_eq!(first.bottom_left, px(0).into());
    assert_eq!(middle.top_left, px(0).into());
    assert_eq!(
        app.world().get::<BackgroundColor>(kids[1]).unwrap().0,
        color(&app, ColorRole::SecondaryGroupedBackground)
    );
    let separators = descendants_with::<crate::list::WingsListSeparator>(&mut app, section);
    assert_eq!(separators.len(), 2);
    assert_eq!(
        app.world().get::<BackgroundColor>(list).unwrap().0,
        color(&app, ColorRole::GroupedBackground)
    );
    // Fluent turns rows into separate cards without separators.
    set_design(&mut app, WingsDesign::Fluent);
    settle(&mut app, 2);
    assert!(descendants_with::<crate::list::WingsListSeparator>(&mut app, section).is_empty());
    let kids = children(&app, section);
    assert_eq!(
        app.world().get::<Node>(kids[1]).unwrap().margin.bottom,
        px(4)
    );
}

/// An iOS alert: narrow panel, capsule buttons side by side, destructive action in red.
#[test]
fn ios_alert_dialog() {
    let mut app = app_with(WingsDesign::Ios);
    let d = spawn(
        &mut app,
        bsn! {
            @WingsDialog { @title: "Discard?", @text: "Lost.", @confirm: "Discard", @destructive: true, @open: true }
        },
    );
    settle(&mut app, 3);
    let surface = descendants_with::<crate::feedback::WingsDialogSurface>(&mut app, d)[0];
    assert_eq!(app.world().get::<Node>(surface).unwrap().max_width, px(318));
    let buttons = descendants_with::<crate::feedback::WingsDialogButton>(&mut app, d);
    assert_eq!(buttons.len(), 2);
    for b in &buttons {
        assert_eq!(app.world().get::<Node>(*b).unwrap().min_height, px(48));
        assert_eq!(app.world().get::<Node>(*b).unwrap().flex_grow, 1.0);
    }
    let confirm = buttons
        .iter()
        .find(|b| {
            app.world()
                .get::<crate::feedback::WingsDialogButton>(**b)
                .unwrap()
                .confirm
        })
        .copied()
        .unwrap();
    let label = descendants_with::<Text>(&mut app, confirm)[0];
    assert_eq!(
        app.world().get::<TextColor>(label).unwrap().0,
        color(&app, ColorRole::Error)
    );
}

/// Apple text fields show their label as a placeholder that hides while there is text.
#[test]
fn apple_text_field_placeholder() {
    let mut app = app_with(WingsDesign::MacOs);
    let empty = spawn(&mut app, bsn! { @WingsTextField { @label: "Name" } });
    let filled = spawn(
        &mut app,
        bsn! { @WingsTextField { @label: "Name", @value: "Ada" } },
    );
    let label =
        |app: &mut App, f| descendants_with::<crate::text_field::WingsTextFieldLabel>(app, f)[0];
    let l = label(&mut app, empty);
    assert_eq!(app.world().get::<Node>(l).unwrap().display, Display::Flex);
    assert_eq!(
        app.world().get::<Node>(l).unwrap().position_type,
        PositionType::Absolute
    );
    let l = label(&mut app, filled);
    assert_eq!(app.world().get::<Node>(l).unwrap().display, Display::None);
    assert_eq!(app.world().get::<Node>(filled).unwrap().min_height, px(24));
}

/// Fluent text fields carry the focus underline.
#[test]
fn fluent_text_field_underline() {
    let mut app = app_with(WingsDesign::Fluent);
    let f = spawn(&mut app, bsn! { @WingsTextField { @label: "Name" } });
    assert_eq!(
        descendants_with::<crate::text_field::WingsTextFieldUnderline>(&mut app, f).len(),
        1
    );
    assert_eq!(app.world().get::<Node>(f).unwrap().min_height, px(32));
}

// ---------------------------------------------------------------------------------------------
// Safe area
// ---------------------------------------------------------------------------------------------

fn scaffold_with_bars(app: &mut App) -> Entity {
    spawn(
        app,
        bsn! {
            @WingsScaffold
            Children [
                @WingsTopAppBar { @title: "Inbox" }
                --
                @WingsColumn Node { flex_grow: 1.0 }
                --
                @WingsNavigationBar
                Children [ @WingsNavigationItem { @label: "Home", @selected: true } ]
            ]
        },
    )
}

/// On an iPhone the top bar grows under the Dynamic Island and the tab bar floats above the
/// home indicator; the keyboard hides the tab bar and pads the content instead.
#[test]
fn iphone_safe_area_scaffold() {
    let mut app = app_with(WingsDesign::Ios);
    app.insert_resource(WingsDevicePreview::new(WingsDeviceProfile::IPHONE_PRO));
    let scaffold = scaffold_with_bars(&mut app);
    settle(&mut app, 3);
    let bar = descendants_with::<WingsTopAppBar>(&mut app, scaffold)[0];
    let tab = descendants_with::<WingsNavigationBar>(&mut app, scaffold)[0];
    let bar_node = app.world().get::<Node>(bar).unwrap();
    assert_eq!(bar_node.height, px(104.0 + 62.0));
    assert_eq!(bar_node.padding.top, px(66.0));
    assert_eq!(
        app.world().get::<Node>(tab).unwrap().margin.bottom,
        px(22.0)
    );
    assert_ne!(app.world().get::<Node>(tab).unwrap().display, Display::None);
    // Keyboard up: tab bar hidden, content padded by the keyboard.
    app.world_mut()
        .resource_mut::<WingsDevicePreview>()
        .keyboard = true;
    settle(&mut app, 3);
    assert_eq!(app.world().get::<Node>(tab).unwrap().display, Display::None);
    assert_eq!(
        app.world().get::<Node>(scaffold).unwrap().padding.bottom,
        px(336.0)
    );
    // Keyboard down: tab bar back.
    app.world_mut()
        .resource_mut::<WingsDevicePreview>()
        .keyboard = false;
    settle(&mut app, 3);
    assert_ne!(app.world().get::<Node>(tab).unwrap().display, Display::None);
    assert_eq!(
        app.world().get::<Node>(scaffold).unwrap().padding.bottom,
        px(0.0)
    );
}

/// Material on an Android phone: the bottom bar grows under the gesture bar, landscape insets
/// pad the content on the camera side.
#[test]
fn android_safe_area_scaffold() {
    let mut app = app();
    app.insert_resource(WingsDevicePreview::new(WingsDeviceProfile::ANDROID_PHONE));
    let scaffold = scaffold_with_bars(&mut app);
    settle(&mut app, 3);
    let tab = descendants_with::<WingsNavigationBar>(&mut app, scaffold)[0];
    assert_eq!(app.world().get::<Node>(tab).unwrap().height, px(104.0));
    app.insert_resource(WingsDevicePreview::new(
        WingsDeviceProfile::ANDROID_PHONE.landscape(),
    ));
    settle(&mut app, 3);
    assert_eq!(
        app.world().get::<Node>(scaffold).unwrap().padding.left,
        px(32.0)
    );
}

// ---------------------------------------------------------------------------------------------
// New components: segmented, tabs, menus, dropdowns, sheets, pickers, focus ring
// ---------------------------------------------------------------------------------------------

#[derive(Resource, Default)]
struct Picked(Vec<usize>);

/// Activating a segment moves the selection and reports it.
#[test]
fn segmented_selection() {
    let mut app = app_with(WingsDesign::Ios);
    app.init_resource::<Picked>();
    let s = spawn(
        &mut app,
        bsn! {
            @WingsSegmented { @options: vec!["Day".into(), "Week".into(), "Month".into()] }
            on(|ev: On<WingsSegmentChanged>, mut picked: ResMut<Picked>| picked.0.push(ev.index))
        },
    );
    let segments = children(&app, s);
    assert_eq!(segments.len(), 3);
    assert!(app.world().get::<Checked>(segments[0]).is_some());
    app.world_mut().trigger(Activate {
        entity: segments[2],
    });
    settle(&mut app, 2);
    assert_eq!(app.world().resource::<Picked>().0, vec![2]);
    assert!(app.world().get::<Checked>(segments[2]).is_some());
    assert!(app.world().get::<Checked>(segments[0]).is_none());
    // iOS lifts the selected segment off the gray track.
    assert_eq!(app.world().get::<Node>(s).unwrap().height, px(32));
    assert_eq!(
        app.world()
            .get::<WingsElevationLevel>(segments[2])
            .unwrap()
            .0,
        1
    );
}

/// Material tabs draw an accent indicator under the selected tab only.
#[test]
fn material_tabs_indicator() {
    let mut app = app();
    let t = spawn(
        &mut app,
        bsn! { @WingsTabs { @options: vec!["One".into(), "Two".into()] } },
    );
    settle(&mut app, 2);
    let tabs = children(&app, t);
    let indicator = |app: &mut App, tab| descendants_with::<WingsSkinPart>(app, tab)[0];
    let first = indicator(&mut app, tabs[0]);
    let second = indicator(&mut app, tabs[1]);
    assert_ne!(
        app.world().get::<Node>(first).unwrap().display,
        Display::None
    );
    assert_eq!(
        app.world().get::<Node>(second).unwrap().display,
        Display::None
    );
}

/// A dropdown opens a popup, picking an item closes it and updates the value.
#[test]
fn dropdown_picks_option() {
    let mut app = app_with(WingsDesign::Fluent);
    app.init_resource::<Picked>();
    let d = spawn(
        &mut app,
        bsn! {
            @WingsDropdown { @options: vec!["Small".into(), "Medium".into(), "Large".into()] }
            on(|ev: On<WingsMenuSelected>, mut picked: ResMut<Picked>| picked.0.push(ev.index))
        },
    );
    assert_eq!(app.world().get::<Node>(d).unwrap().min_height, px(32));
    app.world_mut().trigger(Activate { entity: d });
    settle(&mut app, 3);
    let popup = app
        .world()
        .get::<crate::popup::WingsPopupOwner>(d)
        .unwrap()
        .popup
        .unwrap();
    let items = children(&app, popup);
    assert_eq!(items.len(), 3);
    app.world_mut().trigger(Activate { entity: items[1] });
    settle(&mut app, 3);
    assert_eq!(app.world().resource::<Picked>().0, vec![1]);
    let owner = app.world().get::<crate::popup::WingsPopupOwner>(d).unwrap();
    assert!(!owner.open && owner.popup.is_none());
    assert!(app.world().get_entity(popup).is_err());
    let value = descendants_with::<crate::popup::WingsDropdownValue>(&mut app, d)[0];
    assert_eq!(app.world().get::<Text>(value).unwrap().0, "Medium");
}

/// A sheet shows a scrim while open; the scrim goes away when it closes.
#[test]
fn sheet_scrim_follows_visibility() {
    let mut app = app_with(WingsDesign::Ios);
    let sheet = spawn(
        &mut app,
        bsn! { @WingsBottomSheet { @title: "Share", @open: true } },
    );
    settle(&mut app, 2);
    let scrims = |app: &mut App| {
        app.world_mut()
            .query::<&crate::sheet::WingsSheetScrim>()
            .iter(app.world())
            .count()
    };
    assert_eq!(scrims(&mut app), 1);
    // iOS sheets have 34 point corners and a grabber.
    assert_eq!(descendants_with::<WingsSkinPart>(&mut app, sheet).len(), 1);
    app.world_mut()
        .get_mut::<WingsAnimatedVisibility>(sheet)
        .unwrap()
        .visible = false;
    settle(&mut app, 2);
    assert_eq!(scrims(&mut app), 0);
}

/// The date picker lays out the month and reports the picked day.
#[test]
fn date_picker_month_grid() {
    let mut app = app();
    let date = crate::picker::WingsDate::new(2026, 2, 10).unwrap();
    let p = spawn(&mut app, bsn! { @WingsDatePicker { @date: date } });
    settle(&mut app, 2);
    let cells = descendants_with::<crate::picker::WingsDayCell>(&mut app, p);
    assert_eq!(cells.len(), 28, "February 2026 has 28 days");
    let title = descendants_with::<crate::picker::WingsDatePickerTitle>(&mut app, p)[0];
    assert_eq!(app.world().get::<Text>(title).unwrap().0, "February 2026");
    let fifth = cells
        .iter()
        .copied()
        .find(|c| {
            app.world()
                .get::<crate::picker::WingsDayCell>(*c)
                .unwrap()
                .date
                .day
                == 5
        })
        .unwrap();
    app.world_mut().trigger(Activate { entity: fifth });
    settle(&mut app, 2);
    let state = app
        .world()
        .get::<crate::picker::WingsDatePickerState>(p)
        .unwrap();
    assert_eq!(state.selected.day, 5);
    // Next month.
    let next = descendants_with::<crate::picker::WingsMonthStep>(&mut app, p)
        .into_iter()
        .find(|e| {
            app.world()
                .get::<crate::picker::WingsMonthStep>(*e)
                .unwrap()
                .0
                == 1
        })
        .unwrap();
    app.world_mut().trigger(Activate { entity: next });
    settle(&mut app, 2);
    assert_eq!(app.world().get::<Text>(title).unwrap().0, "March 2026");
}

/// Time picker steppers wrap around and update the texts.
#[test]
fn time_picker_steps() {
    let mut app = app();
    let p = spawn(
        &mut app,
        bsn! { @WingsTimePicker { @hour: 9, @minute: 59, @h24: true } },
    );
    settle(&mut app, 2);
    let up = descendants_with::<crate::picker::WingsTimeStep>(&mut app, p)
        .into_iter()
        .find(|e| {
            let s = app.world().get::<crate::picker::WingsTimeStep>(*e).unwrap();
            s.field == crate::picker::WingsTimeField::Minute && s.delta == 1
        })
        .unwrap();
    app.world_mut().trigger(Activate { entity: up });
    settle(&mut app, 2);
    let state = *app
        .world()
        .get::<crate::picker::WingsTimePickerState>(p)
        .unwrap();
    assert_eq!((state.hour, state.minute), (9, 0));
    let texts: Vec<String> = descendants_with::<Text>(&mut app, p)
        .into_iter()
        .map(|t| app.world().get::<Text>(t).unwrap().0.clone())
        .collect();
    assert!(texts.contains(&"09".to_string()) && texts.contains(&"00".to_string()));
}

/// Keyboard focus shows the design's ring; pointer focus does not.
#[test]
fn focus_ring_follows_keyboard_focus() {
    use bevy::input_focus::{InputFocus, InputFocusVisible};
    let mut app = app_with(WingsDesign::MacOs);
    app.init_resource::<InputFocusVisible>();
    let b = spawn(&mut app, bsn! { @WingsButton { @text: "Go" } });
    app.world_mut().insert_resource(InputFocus::from_entity(b));
    app.world_mut().resource_mut::<InputFocusVisible>().0 = true;
    settle(&mut app, 2);
    let outline = *app.world().get::<Outline>(b).unwrap();
    assert_eq!(outline.width, px(3));
    assert!(outline.color.alpha() < 1.0);
    app.world_mut().resource_mut::<InputFocusVisible>().0 = false;
    settle(&mut app, 2);
    assert_eq!(app.world().get::<Outline>(b).unwrap().width, px(0));
}

/// A virtual list of ten thousand items only spawns the rows in view, and swaps rows as it
/// scrolls.
#[test]
fn virtual_list_spawns_visible_rows_only() {
    let mut app = app();
    app.insert_resource(WingsViewportOverride {
        width: 400.0,
        height: 500.0,
    });
    let list = app
        .world_mut()
        .spawn(WingsVirtualList::new(10_000, 50.0, |i| {
            Box::new(bsn! { Text(format!("Row {i}")) })
        }))
        .id();
    settle(&mut app, 3);
    let rows = app.world().get::<WingsVirtualRows>(list).unwrap();
    assert!(rows.row(0).is_some());
    assert!(rows.len() < 30, "spawned {} rows", rows.len());
    let first = rows.row(0).unwrap();
    assert!(
        app.world()
            .get::<crate::virtual_list::WingsVirtualRow>(first)
            .is_some()
    );

    app.world_mut().get_mut::<ScrollPosition>(list).unwrap().0.y = 5_000.0;
    settle(&mut app, 3);
    let rows = app.world().get::<WingsVirtualRows>(list).unwrap();
    assert!(rows.row(0).is_none());
    assert!(rows.row(100).is_some());
    assert!(rows.len() < 30);
    let row = rows.row(100).unwrap();
    assert!(app.world().get_entity(first).is_err());
    let text = descendants_with::<Text>(&mut app, row);
    assert_eq!(app.world().get::<Text>(text[0]).unwrap().0, "Row 100");
}

/// Interactive widgets join directional navigation.
#[test]
fn interactive_widgets_are_navigable() {
    let mut app = app();
    let b = spawn(&mut app, bsn! { @WingsButton { @text: "Go" } });
    assert!(
        app.world()
            .get::<bevy::ui::auto_directional_navigation::AutoDirectionalNavigation>(b)
            .is_some()
    );
}

/// The iOS large title collapses when the screen scrolls, not when a list nested in it does.
#[test]
fn large_title_ignores_nested_scrolls() {
    let mut app = app_with(WingsDesign::Ios);
    let scaffold = spawn(
        &mut app,
        bsn! {
            @WingsScaffold
            Children [
                @WingsTopAppBar { @title: "Inbox" }
                --
                @WingsScrollColumn
                Children [
                    @WingsScrollColumn Node { height: px(200) }
                ]
            ]
        },
    );
    settle(&mut app, 3);
    let bar = descendants_with::<WingsTopAppBar>(&mut app, scaffold)[0];
    let collapsed = |app: &App| {
        app.world()
            .get::<crate::scaffold_skin::WingsResolvedTitle>(bar)
            .unwrap()
            .collapsed
    };
    let mut scrolls = descendants_with::<bevy::ui_widgets::ScrollArea>(&mut app, scaffold);
    assert_eq!(scrolls.len(), 2);
    // Outer first.
    scrolls.sort_by_key(|e| children(&app, *e).is_empty());
    let (outer, inner) = (scrolls[0], scrolls[1]);
    // Headless apps do not propagate visibility.
    for e in [outer, inner] {
        app.world_mut()
            .entity_mut(e)
            .insert(InheritedVisibility::VISIBLE);
    }
    assert!(!collapsed(&app));
    app.world_mut()
        .get_mut::<ScrollPosition>(inner)
        .unwrap()
        .0
        .y = 500.0;
    settle(&mut app, 2);
    assert!(
        !collapsed(&app),
        "a nested list does not collapse the title"
    );
    app.world_mut()
        .get_mut::<ScrollPosition>(outer)
        .unwrap()
        .0
        .y = 100.0;
    settle(&mut app, 2);
    assert!(collapsed(&app));
}

/// One back press closes one layer: the menu first, then the dialog, then the sheet.
#[test]
fn back_closes_topmost_layer() {
    #[derive(Resource, Default)]
    struct Dismissed(Vec<&'static str>);
    let mut app = app_with(WingsDesign::Material);
    app.init_resource::<Dismissed>();
    let sheet = spawn(
        &mut app,
        bsn! {
            @WingsBottomSheet { @title: "Share", @open: true }
            on(|_: On<WingsSheetDismissed>, mut d: ResMut<Dismissed>| d.0.push("sheet"))
        },
    );
    let dialog = spawn(
        &mut app,
        bsn! {
            @WingsDialog { @title: "Delete?", @text: "Gone.", @confirm: "Delete", @open: true }
            on(|r: On<WingsDialogResult>, mut d: ResMut<Dismissed>| {
                assert!(!r.confirmed);
                d.0.push("dialog");
            })
        },
    );
    let dropdown = spawn(
        &mut app,
        bsn! { @WingsDropdown { @options: vec!["A".into(), "B".into()] } },
    );
    app.world_mut().trigger(Activate { entity: dropdown });
    settle(&mut app, 3);
    let popup_open = |app: &App| {
        app.world()
            .get::<crate::popup::WingsPopupOwner>(dropdown)
            .unwrap()
            .open
    };
    let visible = |app: &App, e: Entity| {
        app.world()
            .get::<WingsAnimatedVisibility>(e)
            .unwrap()
            .visible
    };
    assert!(popup_open(&app) && visible(&app, dialog) && visible(&app, sheet));
    // The same path Escape, the browser back key and gamepad B take.
    let press = |app: &mut App| {
        app.world_mut()
            .resource_mut::<bevy_wings_ui_core::back::WingsBack>()
            .press();
        app.update();
        app.update();
    };
    press(&mut app);
    assert!(!popup_open(&app));
    assert!(visible(&app, dialog) && visible(&app, sheet));
    press(&mut app);
    assert!(!visible(&app, dialog) && visible(&app, sheet));
    press(&mut app);
    assert!(!visible(&app, sheet));
    assert_eq!(
        app.world().resource::<Dismissed>().0,
        vec!["dialog", "sheet"]
    );
}

/// iOS bars always carry the soft scroll edge over the status bar and toolbar row (content
/// shows it by moving into it), and the titles hand over with the scroll offset.
#[test]
fn ios_bar_scroll_edge_and_title_handover() {
    let mut app = app_with(WingsDesign::Ios);
    let scaffold = spawn(
        &mut app,
        bsn! {
            @WingsScaffold
            Children [
                @WingsTopAppBar { @title: "Inbox" }
                --
                @WingsScrollColumn
            ]
        },
    );
    settle(&mut app, 3);
    let bar = descendants_with::<WingsTopAppBar>(&mut app, scaffold)[0];
    let title = descendants_with::<crate::scaffold::WingsTopAppBarTitle>(&mut app, bar)[0];
    let scroll = descendants_with::<bevy::ui_widgets::ScrollArea>(&mut app, scaffold)[0];
    app.world_mut()
        .entity_mut(scroll)
        .insert(InheritedVisibility::VISIBLE);
    // At rest: the edge is there, drawn on a layer that reaches below the toolbar row.
    let layers = descendants_with::<crate::glass::WingsScrollEdgeLayer>(&mut app, bar);
    assert_eq!(layers.len(), 1);
    assert!(app.world().get::<crate::glass::WingsGlass>(bar).is_none());
    assert!(app.world().get::<BackgroundGradient>(layers[0]).is_some());
    let edge = *app
        .world()
        .get::<crate::glass::WingsScrollEdge>(bar)
        .unwrap();
    assert_eq!(
        app.world().get::<Node>(layers[0]).unwrap().height,
        px(edge.height + crate::glass::SCROLL_EDGE_OVERHANG)
    );
    let opacity = |app: &App| app.world().get::<WingsOpacity>(title).unwrap().0;
    assert_eq!(opacity(&app), 1.0);
    // Half way to the collapse: the large title is half faded.
    let set = |app: &mut App, y: f32| {
        app.world_mut()
            .get_mut::<ScrollPosition>(scroll)
            .unwrap()
            .0
            .y = y;
        settle(app, 3);
    };
    set(&mut app, 12.0);
    assert!((opacity(&app) - 0.5).abs() < 0.01);
    // Just collapsed: the inline title starts invisible and fades in with more scrolling.
    set(&mut app, 25.0);
    assert!(opacity(&app) < 0.1);
    set(&mut app, 60.0);
    assert_eq!(opacity(&app), 1.0);
    // Back to the top: the large title is fully there again.
    set(&mut app, 0.0);
    assert_eq!(opacity(&app), 1.0);
    assert_eq!(
        descendants_with::<crate::glass::WingsScrollEdgeLayer>(&mut app, bar).len(),
        1
    );
}
