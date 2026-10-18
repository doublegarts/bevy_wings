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
//! * **Designs**: `wings.toml` selects the platform's own look (Cupertino on Apple
//!   platforms, Fluent on Windows, Material elsewhere) or a custom design. The button in the
//!   top bar cycles through Material, iOS, macOS and Fluent at runtime, and `WINGS_DESIGN=ios`
//!   (or `macos`, `fluent`, `material`) starts the gallery in a given design.
//! * **Visual regression**: `WINGS_CAPTURE=shot.png` renders with a fixed frame time, saves a
//!   screenshot of the window at frame `WINGS_CAPTURE_FRAME` (default 90) and exits;
//!   `WINGS_SCALE=1` fixes the scale factor. `scripts/visual-regression.py` drives it.
//! * **Theme files**: `WINGS_THEME_FILE=themes/brand.wings.toml` follows a theme file from
//!   `assets/`; edit and save it while the gallery runs and the theme updates.
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
    Settings,
}

impl Screen {
    /// The first screen: `WINGS_SCREEN=controls` (or `counter`, `notes`, `settings`) starts
    /// somewhere else than Home.
    fn start() -> Screen {
        match std::env::var("WINGS_SCREEN").unwrap_or_default().as_str() {
            "controls" => Screen::Controls,
            "counter" => Screen::Counter,
            "notes" => Screen::Notes,
            "settings" => Screen::Settings,
            _ => Screen::Home,
        }
    }

    const ALL: [Screen; 5] = [
        Screen::Home,
        Screen::Controls,
        Screen::Counter,
        Screen::Notes,
        Screen::Settings,
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
            Screen::start(),
            |key: &Screen| match key {
                Screen::Home => nav_entry(home_screen()),
                Screen::Controls => nav_entry(controls_screen()),
                Screen::Counter => nav_entry(counter_screen()),
                Screen::Notes => nav_entry(notes_screen()),
                Screen::Settings => nav_entry(settings_screen()),
            },
        ))
        .register_loadable::<Vec<Note>>()
        .add_systems(Update, (start_notes_load, render_notes))
        .register_view_model::<CounterViewModel>()
        .register_effects::<CounterViewModel, WingsShowSnackbar>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (show_design_name, log_screen_once, attach_shader_demos),
        );
}

/// Screenshot request for visual regression runs.
#[derive(Resource)]
struct Capture {
    path: String,
    frame: u32,
    frames: u32,
    attempts: u32,
    /// The offscreen image the app renders into.
    image: Option<Handle<Image>>,
}

/// Takes a screenshot of the window at the requested frame.
fn capture_screenshot(mut commands: Commands, mut capture: ResMut<Capture>) {
    use bevy::render::view::screenshot::Screenshot;
    capture.frames += 1;
    if capture.frames == capture.frame {
        let screenshot = match &capture.image {
            Some(image) => Screenshot::image(image.clone()),
            None => Screenshot::primary_window(),
        };
        commands.spawn(screenshot).observe(save_capture);
    }
}

/// Saves the screenshot and exits. A blank image means nothing was drawn yet, so it tries
/// again a little later.
fn save_capture(
    captured: On<bevy::render::view::screenshot::ScreenshotCaptured>,
    mut capture: ResMut<Capture>,
    mut exit: MessageWriter<AppExit>,
) {
    // Compare colors only: a frame that never presented can still vary in alpha.
    let blank = captured.image.data.as_ref().is_none_or(|data| {
        let (pixels, _) = data.as_chunks::<4>();
        pixels
            .iter()
            .all(|p| pixels.first().is_some_and(|f| p[..3] == f[..3]))
    });
    if blank && capture.attempts < 10 {
        capture.attempts += 1;
        capture.frame = capture.frames + 30;
        return;
    }
    let mut save = bevy::render::view::screenshot::save_to_disk(capture.path.clone());
    save(captured);
    exit.write(AppExit::Success);
}

/// Logs the viewport and safe area once, which helps when running on devices.
fn log_screen_once(
    mut frames: Local<u32>,
    viewport: Res<WingsViewport>,
    safe: Res<WingsSafeArea>,
    windows: Query<&Window>,
) {
    *frames += 1;
    if *frames == 30 {
        for w in &windows {
            info!(
                "window {}x{} (physical {}x{}, scale {})",
                w.width(),
                w.height(),
                w.physical_width(),
                w.physical_height(),
                w.scale_factor()
            );
        }
        info!("viewport {viewport:?}");
        info!("safe area {safe:?}");
    }
}

fn main() {
    // The look comes from wings.toml; WINGS_DESIGN overrides the design for previews.
    let mut theme = wings_config!().theme();
    if let Some(design) = std::env::var("WINGS_DESIGN")
        .ok()
        .and_then(|d| WingsDesign::parse(&d))
    {
        theme.set_design(design);
    }
    if std::env::var("WINGS_DARK").is_ok() {
        theme.mode = WingsThemeMode::Dark;
    }
    // WINGS_DEVICE=iphone (or ipad, android, foldable, tv, ... with an optional -landscape)
    // previews a device: its size, safe area, Dynamic Island and home indicator.
    let device = std::env::var("WINGS_DEVICE")
        .ok()
        .and_then(|d| WingsDeviceProfile::by_name(&d));
    // WINGS_WINDOW=402x874 sets the window size.
    let (width, height) = std::env::var("WINGS_WINDOW")
        .ok()
        .and_then(|s| {
            let (w, h) = s.split_once('x')?;
            Some((w.parse().ok()?, h.parse().ok()?))
        })
        .or(device.as_ref().map(|d| (d.size.x as u32, d.size.y as u32)))
        .unwrap_or((1100u32, 760u32));
    let desktop = !cfg!(any(target_os = "ios", target_os = "android"));
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Wings gallery".into(),
            // Phones and tablets use the whole screen; only desktops get a sized window.
            resolution: if desktop {
                let resolution: bevy::window::WindowResolution = (width, height).into();
                match std::env::var("WINGS_SCALE")
                    .ok()
                    .and_then(|s| s.parse().ok())
                {
                    Some(scale) => resolution.with_scale_factor_override(scale),
                    None => resolution,
                }
            } else {
                default()
            },
            mode: if desktop {
                bevy::window::WindowMode::Windowed
            } else {
                bevy::window::WindowMode::BorderlessFullscreen(
                    bevy::window::MonitorSelection::Primary,
                )
            },
            // A unified, transparent title bar like SwiftUI apps on Mac (no effect elsewhere).
            // Device previews hide the title bar entirely.
            titlebar_transparent: true,
            fullsize_content_view: true,
            titlebar_show_title: false,
            titlebar_show_buttons: device.is_none(),
            // Screenshot runs render offscreen without showing a window.
            visible: std::env::var("WINGS_CAPTURE").is_err(),
            ..default()
        }),
        ..default()
    }))
    .add_plugins(WingsUiPlugins.set(WingsThemePlugin::with_theme(theme)));
    if let Ok(path) = std::env::var("WINGS_CAPTURE") {
        let frame = std::env::var("WINGS_CAPTURE_FRAME")
            .ok()
            .and_then(|f| f.parse().ok())
            .unwrap_or(90);
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_millis(16),
        ))
        .insert_resource(Capture {
            path,
            frame,
            frames: 0,
            attempts: 0,
            image: None,
        })
        .add_systems(Update, capture_screenshot);
    }
    if let Ok(path) = std::env::var("WINGS_THEME_FILE") {
        app.add_plugins(WingsThemeFilePlugin::new(path));
    }
    if let Some(device) = device {
        app.insert_resource(WingsDevicePreview::new(device));
    }
    wings(&mut app);
    app.run();
}

fn setup(
    mut commands: Commands,
    capture: Option<ResMut<Capture>>,
    mut images: ResMut<Assets<Image>>,
    windows: Query<&Window>,
) {
    let mut camera = commands.spawn(Camera2d);
    // Screenshot runs render into an image of the window's size with the window hidden, so
    // they never depend on the window being on screen (macOS stops drawing covered windows).
    if let (Some(mut capture), Ok(window)) = (capture, windows.single()) {
        let image = images.add(Image::new_target_texture(
            window.physical_width(),
            window.physical_height(),
            bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
            None,
        ));
        camera.insert((
            bevy::camera::RenderTarget::Image(bevy::camera::ImageRenderTarget {
                handle: image.clone(),
                scale_factor: window.scale_factor(),
            }),
            // UI goes to the window camera by default; this camera draws into the image.
            IsDefaultUiCamera,
        ));
        capture.image = Some(image);
    }
    commands.spawn_scene(root());
}

/// Marks the label of the design switch button.
#[derive(Component, Default, Clone)]
struct DesignButton;

/// Keeps the design button label on the active design's name.
fn show_design_name(
    theme: Res<WingsTheme>,
    buttons: Query<&Children, With<DesignButton>>,
    mut texts: Query<&mut Text>,
) {
    if !theme.is_changed() {
        return;
    }
    for children in &buttons {
        for child in children.iter() {
            if let Ok(mut text) = texts.get_mut(child) {
                text.0 = theme.design.name().to_string();
            }
        }
    }
}

fn navigation_item(screen: Screen, label: &'static str, icon: &'static str) -> impl Scene {
    let selected = screen == Screen::start();
    bsn! { @WingsNavigationItem { @label: label, @icon: icon, @selected: selected } }
}

fn navigation_items() -> impl SceneList {
    bsn_list! {
        @{navigation_item(Screen::Home, "Home", "icons/home.png")}
        --
        @{navigation_item(Screen::Controls, "Controls", "icons/controls.png")}
        --
        @{navigation_item(Screen::Counter, "Counter", "icons/counter.png")}
        --
        @{navigation_item(Screen::Notes, "Notes", "icons/notes.png")}
        --
        @{navigation_item(Screen::Settings, "Settings", "icons/settings.png")}
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
                @WingsButton { @text: "Design", @variant: WingsButtonVariant::Outlined }
                DesignButton
                on(|_: On<Activate>, mut theme: ResMut<WingsTheme>| {
                    let next = theme.design.next();
                    theme.set_design(next);
                })
                --
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
    // A colorful hero shows off Liquid Glass when it scrolls under the floating bars.
    let hero = BackgroundGradient::from(LinearGradient::to_bottom_right(vec![
        Color::srgb(0.98, 0.36, 0.42).into(),
        Color::srgb(0.99, 0.66, 0.22).into(),
        Color::srgb(0.30, 0.55, 0.98).into(),
        Color::srgb(0.47, 0.30, 0.92).into(),
    ]));
    bsn! {
        @WingsScrollColumn { @gap: 16.0, @padding: 24.0 }
        Node { height: percent(100) }
        Children [
            Node {
                height: px(180),
                padding: px(20),
                border_radius: BorderRadius::all(px(26)),
                align_items: AlignItems::End,
            }
            hero
            Children [
                @WingsText { @text: "Wings 0.2", @role: TypeRole::HeadlineLarge }
                WingsForeground({WingsPaint::Fixed(Color::WHITE)})
            ]
            --
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
            --
            @WingsText { @text: "Shaders", @role: TypeRole::HeadlineSmall }
            --
            @WingsRow { @gap: 12.0, @align: AlignItems::Stretch }
            Children [
                Node {
                    flex_grow: 1.0,
                    flex_basis: px(0),
                    height: px(120),
                    padding: px(16),
                    border_radius: BorderRadius::all(px(20)),
                    align_items: AlignItems::End,
                }
                ShaderDemo::Noise
                Children [
                    @WingsText { @text: "Noise", @role: TypeRole::TitleMedium }
                    WingsForeground({WingsPaint::Fixed(Color::WHITE)})
                ]
                --
                Node {
                    flex_grow: 1.0,
                    flex_basis: px(0),
                    height: px(120),
                    padding: px(16),
                    border: UiRect::all(px(2)),
                    border_radius: BorderRadius::all(px(20)),
                    align_items: AlignItems::End,
                }
                ShaderDemo::GradientBorder
                Children [
                    @WingsText { @text: "Gradient border", @role: TypeRole::TitleMedium }
                ]
            ]
        ]
    }
}

/// Nodes that get a shader material once spawned (scenes cannot reach material assets).
#[derive(Component, Clone, Copy, Default, PartialEq, Eq)]
enum ShaderDemo {
    #[default]
    Noise,
    GradientBorder,
}

fn attach_shader_demos(
    mut commands: Commands,
    added: Query<(Entity, &ShaderDemo), Added<ShaderDemo>>,
    borders: Option<ResMut<Assets<WingsGradientBorderMaterial>>>,
    noise: Option<ResMut<Assets<WingsNoiseMaterial>>>,
) {
    // Headless runs have no shader materials.
    let (Some(mut borders), Some(mut noise)) = (borders, noise) else {
        return;
    };
    for (entity, demo) in &added {
        match demo {
            ShaderDemo::Noise => {
                commands.entity(entity).insert(MaterialNode(noise.add(
                    WingsNoiseMaterial::clouds(
                        Color::srgb(0.08, 0.1, 0.22),
                        Color::srgb(0.42, 0.55, 0.98),
                    ),
                )));
            }
            ShaderDemo::GradientBorder => {
                commands.entity(entity).insert(MaterialNode(
                    borders.add(
                        WingsGradientBorderMaterial::new(
                            Color::srgb(0.25, 0.55, 1.0),
                            Color::srgb(0.95, 0.35, 0.75),
                        )
                        .turning(0.25),
                    ),
                ));
            }
        }
    }
}

/// A virtual list: ten thousand rows, only the visible ones exist.
fn virtual_rows() -> WingsVirtualList {
    WingsVirtualList::new(10_000, 56.0, |index| {
        let headline = format!("Row {}", index + 1);
        let supporting = if index % 2 == 0 { "Even" } else { "Odd" };
        Box::new(bsn! { @WingsListItem { @headline: headline, @supporting: supporting } })
    })
}

/// Selection controls, inputs and feedback.
fn controls_screen() -> impl Scene {
    let rows = virtual_rows();
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
            @WingsSegmented { @options: vec!["Day".into(), "Week".into(), "Month".into()], @width: px(300) }
            on(|ev: On<WingsSegmentChanged>| info!("segment {}", ev.index))
            --
            @WingsTabs { @options: vec!["Overview".into(), "Details".into(), "Activity".into()] }
            --
            @WingsRow { @gap: 12.0, @wrap: true }
            Children [
                @WingsDropdown { @options: vec!["Small".into(), "Medium".into(), "Large".into()], @selected: 1 }
                on(|ev: On<WingsMenuSelected>| info!("size {}", ev.label))
                --
                @WingsMenuButton { @label: "Actions", @items: vec!["Copy".into(), "Rename".into(), "Delete".into()] }
                on(|ev: On<WingsMenuSelected>| info!("action {}", ev.label))
                --
                @WingsButton { @text: "Hover me", @variant: WingsButtonVariant::Text }
                WingsTooltip { text: "Tooltips follow the design", delay: 0.5 }
                --
                @WingsButton { @text: "Open sheet", @variant: WingsButtonVariant::Tonal }
                on(|_: On<Activate>, mut sheets: Query<&mut WingsAnimatedVisibility, With<WingsSheetDetent>>| {
                    for mut s in &mut sheets {
                        s.visible = true;
                    }
                })
            ]
            --
            @WingsRow { @gap: 24.0, @wrap: true, @align: AlignItems::Start }
            Children [
                @WingsDatePicker
                on(|ev: On<WingsDateChanged>| info!("date {:?}", ev.date))
                --
                @WingsTimePicker { @hour: 9, @minute: 41 }
            ]
            --
            @WingsText { @text: "10,000 rows", @role: TypeRole::TitleMedium }
            --
            @WingsCard { @style: WingsCardStyle::Outlined }
            Node { height: px(280), padding: px(0) }
            Children [
                Node { height: percent(100) }
                rows
            ]
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
            @WingsBottomSheet { @title: "Share" }
            Children [
                @WingsText { @text: "Sheets slide over the content with a scrim behind them.", @role: TypeRole::BodyMedium }
            ]
            --
            @WingsDialog {
                @title: "Discard draft?",
                @text: "Your changes will be lost.",
                @confirm: "Discard",
                @destructive: true,
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
        @WingsScrollColumn
        Node { height: percent(100) }
        Children [
            @WingsList
            Node { flex_grow: 1.0 }
            Children [
                @WingsListSection { @header: "Notes", @footer: "Loaded through a use case and an offline first repository." }
                NotesList
                Children [ @WingsLinearProgress ]
            ]
        ]
    }
}

/// A settings page: grouped list sections with details, accessories and controls.
fn settings_screen() -> impl Scene {
    bsn! {
        @WingsScrollColumn
        Node { height: percent(100) }
        Children [
            @WingsList
            Node { flex_grow: 1.0 }
            Children [
                @WingsListSection { @header: "Network" }
                Children [
                    @WingsListItem {
                        @headline: "Wi-Fi",
                        @icon: "icons/wifi.png",
                        @detail: "Home",
                        @accessory: WingsListAccessory::Chevron,
                    }
                    --
                    @WingsListItem {
                        @headline: "Notifications",
                        @icon: "icons/bell.png",
                        @accessory: WingsListAccessory::Chevron,
                    }
                    --
                    @WingsListItem {
                        @headline: "Favorites",
                        @icon: "icons/star.png",
                        @detail: "12",
                        @accessory: WingsListAccessory::Chevron,
                    }
                ]
                --
                @WingsListSection { @header: "Appearance", @footer: "The design follows wings.toml unless you pick one here." }
                Children [
                    @WingsListItem { @headline: "Material", @supporting: "Android and Linux" }
                    on(|_: On<Activate>, mut theme: ResMut<WingsTheme>| theme.set_design(WingsDesign::Material))
                    --
                    @WingsListItem { @headline: "iOS", @supporting: "iPhone and iPad" }
                    on(|_: On<Activate>, mut theme: ResMut<WingsTheme>| theme.set_design(WingsDesign::Ios))
                    --
                    @WingsListItem { @headline: "macOS", @supporting: "Mac" }
                    on(|_: On<Activate>, mut theme: ResMut<WingsTheme>| theme.set_design(WingsDesign::MacOs))
                    --
                    @WingsListItem { @headline: "Fluent", @supporting: "Windows" }
                    on(|_: On<Activate>, mut theme: ResMut<WingsTheme>| theme.set_design(WingsDesign::Fluent))
                ]
            ]
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
                @WingsListItem {
                    @headline: headline,
                    @supporting: supporting,
                    @accessory: WingsListAccessory::Chevron,
                }
                ChildOf(list)
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
