//! Push and pop destinations; wide windows show list and detail side by side.
mod support;
use bevy::prelude::*;
use bevy_wings::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Screen {
    List,
    Detail,
}
fn main() {
    support::run("Wings: navigation", configure);
}
fn configure(app: &mut App) {
    app.add_plugins(WingsNavigationPlugin::new(Screen::List, |screen: &Screen| match screen {
        Screen::List => nav_entry(bsn! {
            @WingsColumn { @padding: 32.0, @gap: 16.0 }
            Children [
                @WingsText { @text: "Your list", @role: TypeRole::HeadlineMedium }
                --
                @WingsButton { @text: "Open detail" }
                on(|_: On<Activate>, mut stack: ResMut<WingsBackStack<Screen>>| { stack.push(Screen::Detail); })
            ]
        }).list_pane(),
        Screen::Detail => nav_entry(bsn! {
            @WingsColumn { @padding: 32.0, @gap: 16.0 }
            Children [
                @WingsText { @text: "Detail pane", @role: TypeRole::HeadlineMedium }
                --
                @WingsButton { @text: "Back (or Escape)" }
                on(|_: On<Activate>, mut stack: ResMut<WingsBackStack<Screen>>| { stack.pop(); })
            ]
        }).detail_pane(),
    })).add_systems(Startup, |mut commands: Commands| {
        commands.spawn_scene(bsn! { WingsNavDisplay::<Screen> Node { width: percent(100), height: percent(100) } });
    });
}

#[test]
fn push_and_pop_preserve_list() {
    let mut app = support::app("navigation", true);
    configure(&mut app);
    app.update();
    app.world_mut()
        .resource_mut::<WingsBackStack<Screen>>()
        .push(Screen::Detail);
    for _ in 0..60 {
        app.update();
    }
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|t| t.0 == "Detail pane")
    );
    app.world_mut()
        .resource_mut::<WingsBackStack<Screen>>()
        .pop();
    for _ in 0..60 {
        app.update();
    }
    assert!(
        !app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|t| t.0 == "Detail pane")
    );
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|t| t.0 == "Your list")
    );
}
