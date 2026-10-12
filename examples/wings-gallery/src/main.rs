//! Wings gallery: a small app that exercises the whole toolkit.
//!
//! * **Theme**: a switch in the top bar flips light and dark. Twenty buttons on the home
//!   screen (and everything else) repaint at once, with a short cross fade.
//! * **Navigation**: three destinations driven by a [`WingsBackStack`]. On compact windows a
//!   bottom navigation bar is shown, on wider windows a navigation rail.
//! * **DI + MVVM**: the counter screen gets its view model from the DI container; the view
//!   model depends on a repository singleton.
//! * **Data + domain**: the notes screen runs a domain use case on a background task. The use
//!   case reads an offline first repository (data layer) backed by a fake remote.
//!
//! Run with `cargo run -p wings-gallery`.

use bevy::prelude::*;
use bevy_wings::prelude::*;
use std::sync::Arc;

/// Navigation keys.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
enum Screen {
    Home,
    Controls,
    Counter,
    Notes,
}

impl Screen {
    const ALL: [Screen; 4] = [
        Screen::Home,
        Screen::Controls,
        Screen::Counter,
        Screen::Notes,
    ];
}

// ---------------------------------------------------------------------------------------------
// Domain layer: an entity and a use case that only knows the repository contract.
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
struct Note {
    id: Id<Note>,
    title: String,
    pinned: bool,
}

impl DomainEntity for Note {
    fn id(&self) -> Id<Note> {
        self.id
    }
}

/// Loads notes, pinned first.
struct LoadNotes {
    repository: Arc<dyn ReadRepository<Note>>,
}

impl UseCase for LoadNotes {
    type Input = ();
    type Output = Vec<Note>;

    fn execute(&self, _: ()) -> DomainResult<Vec<Note>> {
        let mut notes = self.repository.all()?;
        notes.sort_by_key(|n| (!n.pinned, n.id));
        Ok(notes)
    }
}

// ---------------------------------------------------------------------------------------------
// Data layer: an offline first repository over a remote (faked here).
// ---------------------------------------------------------------------------------------------

fn notes_repository() -> OfflineFirstRepository<Note> {
    let remote = FakeRemote::with_items([
        Note {
            id: Id::new(1),
            title: "Buy milk".into(),
            pinned: false,
        },
        Note {
            id: Id::new(2),
            title: "Ship Wings 0.1".into(),
            pinned: true,
        },
        Note {
            id: Id::new(3),
            title: "Call Ada".into(),
            pinned: false,
        },
    ]);
    OfflineFirstRepository::new(Arc::new(remote)).with_policy(CachePolicy::CacheFirst)
}

/// A repository shared by every screen (DI singleton).
struct GreetingRepository {
    greeting: &'static str,
}

/// The counter screen view model, created by the DI container.
#[derive(Component)]
struct CounterViewModel {
    count: i32,
    repository: std::sync::Arc<GreetingRepository>,
    effects: WingsEffects<WingsShowSnackbar>,
}

impl WingsViewModel for CounterViewModel {}

impl WingsHasEffects<WingsShowSnackbar> for CounterViewModel {
    fn effects(&mut self) -> &mut WingsEffects<WingsShowSnackbar> {
        &mut self.effects
    }
}

impl CounterViewModel {
    fn increment(&mut self) {
        self.count += 1;
        if self.count % 5 == 0 {
            self.effects.send(WingsShowSnackbar::new(format!(
                "{} reached {}",
                self.repository.greeting, self.count
            )));
        }
    }

    fn reset(&mut self) {
        self.count = 0;
        self.effects
            .send(WingsShowSnackbar::new("Counter reset").with_action("OK"));
    }
}

fn app_module() -> WingsModule {
    WingsModule::new("app")
        .single(|_| GreetingRepository {
            greeting: "Counter",
        })
        .view_model(|r| CounterViewModel {
            count: 0,
            repository: r.get(),
            effects: WingsEffects::default(),
        })
        .bind::<dyn ReadRepository<Note>>(|_| Arc::new(notes_repository()))
        .single(|r| LoadNotes {
            repository: r.get(),
        })
}

/// Registers everything except the windowing plugins, so tests can reuse it headless.
fn wings(app: &mut App) {
    app.add_plugins(WingsDiPlugin::new([app_module()]))
        .add_plugins(WingsNavigationPlugin::new(
            Screen::Home,
            |key: &Screen| match key {
                Screen::Home => nav_entry(home_screen()),
                Screen::Controls => nav_entry(controls_screen()),
                Screen::Counter => nav_entry(counter_screen()),
                Screen::Notes => nav_entry(notes_screen()),
            },
        ))
        .register_loadable::<Vec<Note>>()
        .add_systems(Update, (start_notes_load, render_notes))
        .register_view_model::<CounterViewModel>()
        .register_effects::<CounterViewModel, WingsShowSnackbar>()
        .add_systems(Startup, setup);
}

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Wings gallery".into(),
            ..default()
        }),
        ..default()
    }))
    .add_plugins(
        WingsUiPlugins.set(WingsThemePlugin::with_theme(WingsTheme::from_seed(
            Color::srgb(0.30, 0.36, 0.92),
        ))),
    );
    wings(&mut app);
    app.run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands.spawn_scene(root());
}

fn navigation_item(screen: Screen, label: &'static str) -> impl Scene {
    let selected = screen == Screen::Home;
    bsn! { @WingsNavigationItem { @label: label, @selected: selected } }
}

fn navigation_items() -> impl SceneList {
    bsn_list! {
        @{navigation_item(Screen::Home, "Home")}
        --
        @{navigation_item(Screen::Controls, "Controls")}
        --
        @{navigation_item(Screen::Counter, "Counter")}
        --
        @{navigation_item(Screen::Notes, "Notes")}
    }
}

/// Moves the back stack to the selected destination.
fn on_destination_selected(
    ev: On<WingsNavigationSelected>,
    mut stack: ResMut<WingsBackStack<Screen>>,
) {
    if let Some(screen) = Screen::ALL.get(ev.index) {
        stack.reset(screen.clone());
    }
}

fn root() -> impl Scene {
    bsn! {
        @WingsScaffold
        Children [
            @WingsTopAppBar { @title: "Wings gallery" }
            Children [
                @WingsSwitch { @label: "Dark" }
                on(|change: On<ValueChange<bool>>, mut theme: ResMut<WingsTheme>| {
                    theme.mode = if change.value { WingsThemeMode::Dark } else { WingsThemeMode::Light };
                })
            ]
            --
            @WingsRow { @align: AlignItems::Stretch }
            Node { flex_grow: 1.0, min_height: px(0) }
            Children [
                @WingsNavigationRail
                on(on_destination_selected)
                Children [ {navigation_items()} ]
                --
                WingsNavDisplay::<Screen>
                Node { flex_grow: 1.0, height: percent(100) }
            ]
            --
            @WingsNavigationBar
            on(on_destination_selected)
            Children [ {navigation_items()} ]
        ]
    }
}

/// Twenty buttons in a responsive grid: all of them follow the theme.
fn home_screen() -> impl Scene {
    let buttons: Vec<_> = (1..=20)
        .map(|i| {
            let text = format!("Button {i}");
            let variant = match i % 4 {
                0 => WingsButtonVariant::Filled,
                1 => WingsButtonVariant::Tonal,
                2 => WingsButtonVariant::Outlined,
                _ => WingsButtonVariant::Elevated,
            };
            bsn! { @WingsButton { @text: text, @variant: variant } }
        })
        .collect();
    let columns = WingsResponsiveColumns(WingsResponsive::new(2).medium(3).expanded(4));
    bsn! {
        @WingsScrollColumn { @gap: 16.0, @padding: 24.0 }
        Node { height: percent(100) }
        Children [
            @WingsText { @text: "Theme aware buttons", @role: TypeRole::HeadlineSmall }
            --
            @WingsText {
                @text: "Flip the switch in the top bar. Every button repaints from the same theme roles.",
                @role: TypeRole::BodyMedium,
                @color: ColorRole::OnSurfaceVariant,
            }
            --
            @WingsGrid { @columns: 2, @gap: 12.0 }
            columns
            Children [ {buttons} ]
            --
            @WingsCard
            Children [
                @WingsText { @text: "Loading placeholders", @role: TypeRole::TitleMedium }
                --
                @WingsSkeleton { @height: px(14) }
                --
                @WingsSkeleton { @width: percent(70), @height: px(14) }
            ]
        ]
    }
}

/// Selection controls, inputs and feedback.
fn controls_screen() -> impl Scene {
    bsn! {
        @WingsScrollColumn { @gap: 16.0, @padding: 24.0 }
        Node { height: percent(100) }
        Children [
            @WingsText { @text: "Controls", @role: TypeRole::HeadlineSmall }
            --
            @WingsCard { @style: WingsCardStyle::Outlined }
            Children [
                @WingsCheckbox { @label: "Enable notifications", @checked: true }
                --
                @WingsSwitch { @label: "Wi-Fi" }
                --
                @WingsSlider { @value: 40.0, @min: 0.0, @max: 100.0, @width: px(260) }
                --
                @WingsRadioGroup { @gap: 4.0 }
                Children [
                    @WingsRadioButton { @label: "Small", @checked: true }
                    --
                    @WingsRadioButton { @label: "Medium" }
                    --
                    @WingsRadioButton { @label: "Large" }
                ]
                --
                @WingsRow { @gap: 8.0, @wrap: true }
                Children [
                    @WingsChip { @label: "Rust", @checked: true }
                    --
                    @WingsChip { @label: "Bevy" }
                    --
                    @WingsChip { @label: "ECS" }
                ]
            ]
            --
            @WingsTextField { @label: "Your name", @value: "Ferris" }
            on(|ev: On<WingsTextChanged>| info!("name changed to {}", ev.value))
            --
            @WingsLinearProgress { @progress: 0.6 }
            --
            @WingsLinearProgress
            --
            @WingsCircularProgress { @size: 40.0 }
            --
            @WingsButton { @text: "Show snackbar", @variant: WingsButtonVariant::Tonal }
            on(|_: On<Activate>, mut snack: MessageWriter<WingsShowSnackbar>| {
                snack.write(WingsShowSnackbar::new("Hello from Wings").with_action("Undo"));
            })
            --
            @WingsButton { @text: "Open dialog", @variant: WingsButtonVariant::Outlined }
            on(|_: On<Activate>, mut dialogs: Query<&mut WingsAnimatedVisibility, With<WingsDialog>>| {
                for mut d in &mut dialogs {
                    d.visible = true;
                }
            })
            --
            @WingsDialog {
                @title: "Discard draft?",
                @text: "Your changes will be lost.",
                @confirm: "Discard",
            }
            on(|r: On<WingsDialogResult>| info!("dialog confirmed: {}", r.confirmed))
        ]
    }
}

/// MVVM: the view model comes from DI, the texts are bindings, the buttons are intents.
fn counter_screen() -> impl Scene {
    let count = bind_text(|vm: &CounterViewModel| vm.count.to_string());
    let greeting = bind_text(|vm: &CounterViewModel| format!("{} screen", vm.repository.greeting));
    let can_reset = bind_enabled(|vm: &CounterViewModel| vm.count > 0);
    bsn! {
        @WingsColumn { @gap: 16.0, @padding: 24.0, @align: AlignItems::Center }
        WingsProvideViewModel::<CounterViewModel>
        Children [
            @WingsText { @text: "", @role: TypeRole::TitleLarge } greeting
            --
            @WingsText { @text: "0", @role: TypeRole::DisplayLarge } count
            --
            @WingsRow { @gap: 12.0 }
            Children [
                @WingsButton { @text: "Increment" }
                on(on_activate(|vm: &mut CounterViewModel| vm.increment()))
                --
                @WingsButton { @text: "Reset", @variant: WingsButtonVariant::Outlined }
                can_reset
                on(on_activate(|vm: &mut CounterViewModel| vm.reset()))
            ]
        ]
    }
}

// ---------------------------------------------------------------------------------------------
// Presentation layer: load in the background, render the Loadable.
// ---------------------------------------------------------------------------------------------

/// Marks the notes list container.
#[derive(Component, Default, Clone)]
struct NotesList;

fn notes_screen() -> impl Scene {
    bsn! {
        @WingsScrollColumn { @gap: 8.0, @padding: 24.0 }
        Node { height: percent(100) }
        Children [
            @WingsText { @text: "Notes", @role: TypeRole::HeadlineSmall }
            --
            @WingsColumn { @gap: 4.0 }
            NotesList
            Children [ @WingsLinearProgress ]
        ]
    }
}

/// Runs the use case on a background task when the notes screen appears.
fn start_notes_load(
    mut commands: Commands,
    lists: Query<Entity, Added<NotesList>>,
    di: Res<WingsDi>,
) {
    for list in &lists {
        let load_notes = di.get::<LoadNotes>();
        commands.entity(list).load(move || load_notes.execute(()));
    }
}

/// Rebuilds the list whenever the loading state changes.
fn render_notes(
    mut commands: Commands,
    lists: Query<(Entity, &WingsLoadable<Vec<Note>>), Changed<WingsLoadable<Vec<Note>>>>,
) {
    for (list, notes) in &lists {
        commands.entity(list).despawn_children();
        match &notes.0 {
            Loadable::Idle | Loadable::Loading { previous: None } => {
                commands.spawn_scene(bsn! { @WingsLinearProgress ChildOf(list) });
            }
            Loadable::Failed { error, .. } => {
                let text = error.to_string();
                commands.spawn_scene(bsn! {
                    @WingsText { @text: text, @color: ColorRole::Error } ChildOf(list)
                });
            }
            _ => {}
        }
        for note in notes.value().into_iter().flatten() {
            let headline = note.title.clone();
            let supporting = if note.pinned {
                "Pinned".to_string()
            } else {
                String::new()
            };
            commands.spawn_scene(bsn! {
                @WingsListItem { @headline: headline, @supporting: supporting } ChildOf(list)
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::time::TimeUpdateStrategy;
    use core::time::Duration;

    /// The whole gallery runs headless: scenes spawn, the counter view model is injected and
    /// navigation swaps destinations.
    #[test]
    fn gallery_runs_headless() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            bevy::scene::ScenePlugin,
            bevy::input::InputPlugin,
        ));
        app.init_asset::<Image>();
        app.add_plugins((
            bevy::ui_widgets::ButtonPlugin,
            bevy::ui_widgets::CheckboxPlugin,
            bevy::ui_widgets::RadioGroupPlugin,
            bevy::ui_widgets::SliderPlugin,
        ));
        app.add_plugins((
            WingsCorePlugin,
            WingsAnimationPlugin,
            WingsThemePlugin::default(),
            WingsResponsivePlugin::default(),
            WingsModifierPlugin,
            WingsWidgetsPlugin::headless(),
        ));
        app.insert_resource(WingsViewportOverride {
            width: 1280.0,
            height: 800.0,
        });
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            50,
        )));
        wings(&mut app);
        for _ in 0..3 {
            app.update();
        }
        let buttons = app
            .world_mut()
            .query::<&WingsButtonVariant>()
            .iter(app.world())
            .count();
        assert!(
            buttons >= 20,
            "home screen shows twenty buttons, found {buttons}"
        );

        app.world_mut()
            .resource_mut::<WingsBackStack<Screen>>()
            .reset(Screen::Counter);
        for _ in 0..30 {
            app.update();
        }
        let vms = app
            .world_mut()
            .query::<&CounterViewModel>()
            .iter(app.world())
            .count();
        assert_eq!(vms, 1, "counter view model injected from DI");

        app.world_mut()
            .resource_mut::<WingsBackStack<Screen>>()
            .reset(Screen::Notes);
        let mut titles = Vec::new();
        for _ in 0..200 {
            app.update();
            titles = app
                .world_mut()
                .query_filtered::<&Text, ()>()
                .iter(app.world())
                .map(|t| t.0.clone())
                .collect::<Vec<_>>();
            if titles.iter().any(|t| t == "Ship Wings 0.1") {
                break;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(
            titles.iter().any(|t| t == "Ship Wings 0.1"),
            "notes loaded through use case and repository"
        );
    }
}
