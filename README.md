# Wings Framework for Bevy

[![crates.io](https://img.shields.io/crates/v/dga-bevy-wings-framework.svg)](https://crates.io/crates/dga-bevy-wings-framework)
[![docs](https://docs.rs/dga-bevy-wings-framework/badge.svg)](https://docs.rs/dga-bevy-wings-framework/latest/dga_bevy_wings_framework/)
[![CI](https://github.com/doublegarts/dga_bevy_wings_framework/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/doublegarts/dga_bevy_wings_framework/actions/workflows/ci.yml)

**dga_bevy_wings_framework** is a layered application framework for **Bevy 0.20**. It is split
into **data**, **domain** and **presentation** crates, wired together with dependency
injection:

```
 Presentation  widgets, theme, animation, responsive, modifiers, shaders, MVVM, navigation
      |
      v
 Domain        entities, typed ids, use cases, validation, repository contracts, Loadable
      ^
      |
 Data          data sources, offline first repositories, caching, mappers, background loads

 Shared        dependency injection (used by every layer to wire implementations)
```

Dependencies point towards the domain: the domain crate is plain Rust with no engine
dependency, the data layer implements the domain's repository traits, and the presentation
layer calls use cases and renders their results.

In the presentation layer you describe screens with Bevy's `bsn!` scene macro, using ready made widgets such as
`WingsButton`, `WingsText`, `WingsCard`, `WingsSwitch` and `WingsSlider`. Everything that
happens after that (theming, animation, responsiveness, navigation, view model bindings) is
plain ECS: components on entities, change detected systems, observers and messages. There is
no retained widget tree next to the ECS, no virtual DOM and no hidden controller.

```rust
use bevy::prelude::*;
use dga_bevy_wings_framework::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(WingsUiPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    let content = modifier().fill_max_width().padding(24.0);
    commands.spawn_scene(bsn! {
        @WingsScaffold
        Children [
            @WingsTopAppBar { @title: "Hello Wings" }
            --
            @WingsColumn { @gap: 12.0 }
            content
            Children [
                @WingsText { @text: "Welcome", @role: TypeRole::HeadlineMedium }
                --
                @WingsButton { @text: "Toggle dark theme" }
                on(|_: On<Activate>, mut theme: ResMut<WingsTheme>| theme.toggle_dark())
            ]
        ]
    });
}
```

The `content` modifier uses a fluent chain, similar to Compose's `Modifier`. Place it alongside
a widget inside `bsn!` to apply it to that widget's entity. Widget props configure the initial
scene; modifiers apply layout and styling afterward, and changing a modifier component at
runtime reapplies the chain. `Children [...]` describes the hierarchy.

> Bevy 0.20 only. The crates use `bsn!` scene components, `bevy_ui_widgets`, entity events,
> `UiTransform`, WESL UI materials and other 0.20 APIs directly. Rust 1.97.1 or newer.

---

## Table of contents

1. [Highlights](#highlights)
2. [Workspace layout](#workspace-layout)
3. [Installation](#installation)
4. [Domain layer](#domain-layer)
5. [Data layer](#data-layer)
6. [Widgets](#widgets)
7. [Theming: the ECS theme](#theming-the-ecs-theme)
8. [Animation](#animation)
9. [Responsiveness](#responsiveness)
10. [Modifiers](#modifiers)
11. [Custom UI shaders](#custom-ui-shaders)
12. [Navigation](#navigation)
13. [Dependency injection](#dependency-injection)
14. [MVVM](#mvvm)
15. [Frame order](#frame-order)
16. [Performance notes](#performance-notes)
17. [Testing](#testing)
18. [Gallery example](#gallery-example)
19. [Focused examples](#focused-examples)
20. [CI and crates.io releases](#ci-and-cratesio-releases)
21. [Roadmap](#roadmap)
22. [License](#license)

---

## Highlights

* **Clean layers.** A pure Rust domain crate (entities, use cases, validation, repository
  contracts), a data crate (offline first repositories with caching policies and write
  queues) and the presentation crates below, connected through DI.
* **Background loading in the ECS.** `commands.entity(screen).load(move || use_case.execute(()))`
  runs work on Bevy's async compute pool and stores a `Loadable<T>` component that keeps the
  previous value while refreshing.
* **Widgets as scene components.** `@WingsButton { @text: "Save" }` spawns a fully themed,
  keyboard accessible button. Widgets are ordinary entities, so you can query, mutate, add
  components to, or despawn any part of them.
* **ECS theme.** Widgets store *roles* (`ColorRole::Primary`, `TypeRole::BodyLarge`,
  `ShapeRole::Medium`), never colors. One painter system resolves roles to colors. Spawn 20
  white buttons in the light scheme, flip `WingsTheme` to dark and all 20 (and everything else)
  turn dark in the same frame, with an optional cross fade.
* **Animation system.** Tweens with 16 easing curves, physical springs that keep their
  velocity when retargeted, keyframes, repeat and ping pong, staggering, and animated
  visibility with fade, scale and slide transitions. Any component field can be animated by
  implementing a tiny `WingsProperty` trait.
* **Responsive layouts.** Window width classes (compact, medium, expanded, large, extra large),
  responsive node patches, responsive visibility, responsive grid columns and automatic UI
  scaling. The navigation bar becomes a navigation rail on wide windows by itself.
* **Modifiers.** A chainable `WingsModifier` (`.fill_max_width().padding(16.0).shape(...)`)
  that folds into Bevy UI components only when it changes.
* **Custom UI shaders.** WESL materials for touch ripples, skeleton shimmer, circular progress,
  animated aurora backgrounds and frosted glass panels, all anti aliased against rounded
  corners.
* **Navigation.** A type safe back stack that is just a resource holding your own enum.
  Destinations are `bsn!` scenes, entries below the top stay alive (their state survives),
  transitions are animated, and list and detail panes appear side by side on wide windows.
* **Dependency injection.** Modules with singletons, factories, scoped definitions,
  parameterized factories, named qualifiers, trait bindings, cycle detection and an `Inject`
  system parameter. Every navigation entry gets its own scope.
* **MVVM.** View models are components; Bevy change detection is the reactivity. Bindings
  update text, visibility, enabled and checked states, or any component, only when the view
  model changed. Intents and one shot effects round it off.
* **Tested.** About 200 documented unit and integration tests run headless.

---

## Workspace layout

| Folder | Crate | What it contains |
| --- | --- | --- |
| `/` | `dga-bevy-wings-framework` | Facade: re-exports every crate, `WingsUiPlugins`, one `prelude` |
| `crates/domain` | `dga-bevy-wings-domain` | Entities, `Id<T>`, use cases, validation, repository traits, paging, `Loadable` |
| `crates/data` | `dga-bevy-wings-data` | Data sources, offline first repositories, caching, mappers, background loads |
| `crates/shared` | `dga-bevy-wings-di` | Dependency injection container, modules and scopes |
| `crates/presentation` | `dga-bevy-wings-ui-core` | `WingsUiSystems` sets, `WingsState`, `WingsMemo`, `WingsOpacity`, hierarchy helpers |
| `crates/presentation` | `dga-bevy-wings-ui-animation` | Easing, tweens, springs, keyframes, property animators, animated visibility |
| `crates/presentation` | `dga-bevy-wings-ui-theme` | Color schemes (seeded), typography, shapes, spacing, elevation, motion, the painter |
| `crates/presentation` | `dga-bevy-wings-ui-responsive` | Breakpoints, viewport, responsive nodes, visibility, columns, auto scale |
| `crates/presentation` | `dga-bevy-wings-ui-modifier` | `WingsModifier` chains and hover scale |
| `crates/presentation` | `dga-bevy-wings-ui-shaders` | WESL UI materials: ripple, shimmer, circular progress, aurora, glass |
| `crates/presentation` | `dga-bevy-wings-ui-widgets` | All `Wings*` widgets |
| `crates/presentation` | `dga-bevy-wings-ui-mvvm` | View models, bindings, intents, effects |
| `crates/presentation` | `dga-bevy-wings-ui-navigation` | Back stack, navigation graph and display, adaptive panes |
| `examples` | `wings-gallery` | A runnable gallery app using all three layers |

The package name is `dga-bevy-wings-framework` and the import path is
`dga_bevy_wings_framework` (Cargo treats `-` and `_` the same).

Each crate can be used on its own. For example, the domain crate has no dependencies at all,
the DI crate does not depend on any UI crate and the theme crate works with hand written Bevy
UI nodes.

---

## Installation

```toml
[dependencies]
bevy = "0.20"
dga-bevy-wings-framework = { git = "https://github.com/doublegarts/dga_bevy_wings_framework" }
```

After the first crates.io release, replace the Git dependency with
`dga-bevy-wings-framework = "0.1"`.

Cargo features of the facade:

| Feature | Default | Effect |
| --- | --- | --- |
| `shaders` | yes | Custom WESL materials and the widgets that use them (ripple, circular progress, shimmer skeletons) |

Without `shaders`, skeletons fall back to an opacity pulse and the circular progress widget is
not available. Apps without a renderer (servers, tests) can also keep the feature and use
`WingsWidgetsPlugin::headless()`.

Add `WingsUiPlugins` **after** `DefaultPlugins`. The widgets reuse the headless widget plugins
(`bevy_ui_widgets`) that `DefaultPlugins` already registers, and only add them when missing.

```rust
app.add_plugins(DefaultPlugins)
    .add_plugins(WingsUiPlugins.set(WingsThemePlugin::with_theme(
        WingsTheme::from_seed(Color::srgb(0.30, 0.36, 0.92)),
    )));
```

`WingsUiPlugins` contains core, animation, theme, responsive, modifier, shaders (with the
feature) and widgets. Dependency injection and navigation take app specific configuration and
are added separately (see below).

---

## Domain layer

`dga-bevy-wings-domain` holds the business rules. It has no dependencies, so it compiles and
tests in moments and can be shared with servers or tools.

```rust
use dga_bevy_wings_framework::prelude::*;

#[derive(Clone, Debug, PartialEq)]
struct Todo { id: Id<Todo>, title: String, done: bool }

impl DomainEntity for Todo {
    fn id(&self) -> Id<Todo> { self.id }
}

/// A use case depends only on the repository contract.
struct CompleteTodo { todos: Arc<dyn WriteRepository<Todo>> }

impl UseCase for CompleteTodo {
    type Input = Id<Todo>;
    type Output = Todo;

    fn execute(&self, id: Id<Todo>) -> DomainResult<Todo> {
        let todo = Todo { done: true, ..self.todos.require(id)? };
        self.todos.save(todo.clone())?;
        Ok(todo)
    }
}
```

| Item | Purpose |
| --- | --- |
| `Id<T>` | Typed ids: `Id<User>` and `Id<Order>` can not be mixed up |
| `DomainEntity` | An object with identity |
| `UseCase`, `use_case(closure)` | One business operation with input and output |
| `DomainError`, `DomainResult` | Validation, not found, conflict, unauthorized, unavailable (retryable), unexpected |
| `Validator<T>`, `rules` | Declarative rules that report every broken field at once (`not_blank`, `length_between`, `in_range`, `looks_like_email`) |
| `ReadRepository<T>`, `WriteRepository<T>` | Contracts with default `page` and `require` methods |
| `PageRequest`, `Page<T>` | Paging helpers |
| `Loadable<T>` | Idle, loading, ready or failed, keeping the previous value while loading or after a failure |

---

## Data layer

`dga-bevy-wings-data` implements the domain contracts.

```rust
let repository = Arc::new(
    OfflineFirstRepository::new(Arc::new(HttpTodos::new(base_url)))
        .with_policy(CachePolicy::CacheFirst)
        .with_ttl(Duration::from_secs(60)),
);

let module = WingsModule::new("data")
    .bind::<dyn WriteRepository<Todo>>(move |_| repository.clone())
    .single(|r| CompleteTodo { todos: r.get() });
```

* **`OfflineFirstRepository<T>`** keeps a local copy of a `RemoteSource<T>`:
  * `CacheFirst` (default) serves fresh local data without a remote call, refreshes expired
    data and serves stale data when the remote is unavailable.
  * `RemoteFirst`, `CacheOnly` and `RemoteOnly` cover the other common strategies.
  * Writes apply locally at once and are pushed to the remote. Offline writes are queued and
    `sync()` delivers them in order.
* **`DataSource<K, V>`** is the local storage contract; `MemorySource` is a thread safe ordered
  implementation. Plug in files or databases by implementing the trait.
* **`RemoteSource<T>`** is the backend contract; `FakeRemote<T>` can be switched offline and
  counts calls, which makes caching behavior easy to test and to preview.
* **`TtlCache`** with an injectable `Clock` (`SystemClock`, `ManualClock` for tests).
* **`Mapper<A, B>`** and `mapper(closure)` convert transfer objects into entities.

### Background loading

```rust
app.register_loadable::<Vec<Todo>>();

fn start_loading(mut commands: Commands, screens: Query<Entity, Added<TodoScreen>>, di: Res<WingsDi>) {
    for screen in &screens {
        let load = di.get::<LoadTodos>();
        commands.entity(screen).load(move || load.execute(()));
    }
}

fn render(screens: Query<&WingsLoadable<Vec<Todo>>, Changed<WingsLoadable<Vec<Todo>>>>) {
    for todos in &screens {
        // todos.is_loading(), todos.value(), todos.error()
    }
}
```

`load` runs the closure on Bevy's async compute pool, sets `WingsLoadable<T>` to loading, and
a `PreUpdate` system stores the result and triggers `WingsLoadFinished` on the entity. The
gallery's Notes screen uses exactly this path: use case, offline first repository, fake remote.

---

## Widgets

Every widget is a `SceneComponent`. Include it with `@Widget { @prop: value }`, add any extra
components after it, attach observers with `on(...)` and add children with `Children [ ... ]`.
Children you add are appended after the widget's own children.

```rust
bsn! {
    @WingsCard { @style: WingsCardStyle::Outlined }
    Children [
        @WingsText { @text: "Notifications", @role: TypeRole::TitleMedium }
        --
        @WingsSwitch { @label: "Enabled", @checked: true }
        on(|change: On<ValueChange<bool>>| info!("enabled: {}", change.value))
    ]
}
```

### Catalog

| Group | Widget | Props | Emits |
| --- | --- | --- | --- |
| Text | `WingsText` | `text`, `role`, `color` | |
| | `WingsIcon` | `image`, `size` (tinted with the content color) | |
| Layout | `WingsColumn`, `WingsRow` | `gap`, `padding`, `justify`, `align`, `width`, `height`, `wrap` | |
| | `WingsBox` | stacks children in one cell (overlays, badges) | |
| | `WingsSpacer` | `size`, `weight` | |
| | `WingsDivider` | `vertical`, `inset` | |
| | `WingsScrollColumn` | stack props, scrolls with wheel and touchpad | |
| | `WingsGrid` | `columns`, `gap` | |
| | `WingsSurface` | `color`, `shape`, `elevation`, `padding`, `gap`, `outlined` | |
| | `WingsCard` | `style` (elevated, filled, outlined), `padding`, `gap` | |
| Buttons | `WingsButton` | `text`, `icon`, `variant`, `enabled`, `fill_width` | `Activate` |
| | `WingsIconButton` | `icon`, `variant`, `enabled` | `Activate` |
| | `WingsFab` | `icon`, `text` (extended), `size` | `Activate` |
| Selection | `WingsCheckbox` | `checked`, `label`, `enabled` | `ValueChange<bool>` |
| | `WingsSwitch` | `checked`, `label`, `enabled` | `ValueChange<bool>` |
| | `WingsRadioGroup` / `WingsRadioButton` | `horizontal`, `gap` / `checked`, `label` | `ValueChange<Entity>` |
| | `WingsSlider` | `value`, `min`, `max`, `width`, `enabled` | `ValueChange<f32>` |
| | `WingsChip` | `checked`, `label`, `enabled` | `ValueChange<bool>` |
| Input | `WingsTextField` | `value`, `label`, `width`, `max_characters` | `WingsTextChanged` |
| Feedback | `WingsLinearProgress` | `progress` (negative = indeterminate), `size` | |
| | `WingsCircularProgress` | `progress`, `size` (shader) | |
| | `WingsSkeleton` | `width`, `height`, `shape` (shimmer shader) | |
| | `WingsBadge` | `count` (0 = dot) | |
| | `WingsDialog` | `title`, `text`, `confirm`, `dismiss`, `open` | `WingsDialogResult` |
| | Snackbars | `WingsShowSnackbar` message, shown in a `WingsSnackbarHost` | `WingsSnackbarAction` |
| Structure | `WingsScaffold` | full window layout with a snackbar host | |
| | `WingsTopAppBar` | `title`, `navigation_icon`; children become actions | |
| | `WingsNavigationBar` / `WingsNavigationRail` | responsive pair | `WingsNavigationSelected` |
| | `WingsNavigationItem` | `icon`, `label`, `selected` | |
| | `WingsListItem` | `headline`, `supporting`, `icon` | `Activate` |

`WingsCheckbox` traces its check mark from the starting point through the corner to the tip over 220 ms. Unchecking erases the stroke in reverse. The mark is drawn with UI geometry and follows the theme. `WingsSlider` supports both dragging the thumb and clicking the track to move directly to that position.

### Button variants

`WingsButtonVariant` is a component, so changing it restyles the button at runtime:

| Variant | Fill | Content | Notes |
| --- | --- | --- | --- |
| `Filled` | `Primary` | `OnPrimary` | Highest emphasis |
| `Tonal` | `SecondaryContainer` | `OnSecondaryContainer` | |
| `Outlined` | none | `Primary` | `Outline` border |
| `Text` | none | `Primary` | Lowest emphasis |
| `Elevated` | `SurfaceContainerLow` | `Primary` | Shadow level 1 |
| `Danger` | `Error` | `OnError` | Destructive actions |

### Interaction states

Clickable widgets carry `WingsInteractive`. One system maps hover, press and disabled states to
a state layer (8 % hover, 12 % press, blended over the fill by the painter) and dims disabled
widgets to 38 % opacity. Disable any widget with `@enabled: false` or by inserting
`InteractionDisabled`. With the `shaders` feature, a press also spawns a ripple from the
pointer position; opt out with `WingsNoRipple`.

### Checked parts

Selection controls describe their checked look with data: any entity inside a control can
carry a `WingsCheckedPart` with checked and unchecked fill, border, content color, visibility,
translation and scale. One system visits only the controls whose `Checked` state changed and
animates transforms with a spring. Your own controls can reuse it by adding
`WingsCheckedOwner` and parts.

### Disabled, focus and keyboard

All controls build on Bevy's headless widgets, so they get `TabIndex` focus, Enter and Space
activation and accessibility roles. Text fields highlight their outline while focused.

---

## Theming: the ECS theme

`WingsTheme` is a resource holding a light and a dark `WingsColorScheme` (38 roles plus custom
roles), typography (15 type roles), shapes, spacing, elevation and motion tokens.

```rust
// A full light and dark palette from one seed color (Oklch tonal palettes).
let theme = WingsTheme::from_seed(Color::srgb(0.85, 0.30, 0.25)).with_mode(WingsThemeMode::System);
app.add_plugins(WingsUiPlugins.set(WingsThemePlugin::with_theme(theme)));
```

Entities opt in with paint components that store roles:

| Component | Writes |
| --- | --- |
| `WingsBackground(WingsPaint)` | `BackgroundColor` |
| `WingsForeground(WingsPaint)` | `TextColor` |
| `WingsBorderPaint(WingsPaint)` | `BorderColor` |
| `WingsContentColor(WingsPaint)` | content color for descendants |
| `WingsInheritContentColor` | `TextColor` from the closest content color |
| `WingsStateLayer` | interaction overlay blended over the background |
| `WingsTypeStyle(TypeRole)` | `TextFont`, `LineHeight`, `LetterSpacing` |
| `WingsShape(ShapeRole)` | `Node::border_radius` |
| `WingsElevationLevel(u8)` | `BoxShadow` |
| `WingsColorTransition(AnimationSpec)` | animates color changes caused by state |
| `WingsLocalTheme` | forces light or dark for a subtree |

### Default font

`WingsThemePlugin` (included in `WingsUiPlugins`) embeds **Inter** and installs it at Bevy's
default font asset ID before startup. The gallery, every UI example, widget labels, text
fields and ordinary Bevy text using the implicit font therefore use Inter without extra
asset files. The variable font supports the typography roles' regular and medium weights.
Explicit custom font assets remain available.

Inter is bundled under the SIL Open Font License 1.1. The font and its license ship inside
the theme crate; see [font provenance and license](crates/presentation/dga-bevy-wings-ui-theme/src/fonts/README.md).

To select your own font for every themed text role, set the typography font source:

```rust
fn customize_font(mut theme: ResMut<WingsTheme>, assets: Res<AssetServer>) {
    theme.typography.font = assets.load::<Font>("fonts/MyFont.ttf").into();
}
```

Changes apply to existing text as well as newly spawned widgets. Add Wings after
`DefaultPlugins` so Bevy's font assets are initialized before the default is installed.

`WingsPaint` can be a role, a role with alpha, a fixed color, an adaptive pair
(`light`, `dark`) or transparent.

### The 20 buttons scenario

```rust
for i in 0..20 {
    commands.spawn_scene(bsn! { @WingsButton { @text: {format!("Button {i}")}, @variant: WingsButtonVariant::Elevated } });
}

// Later, anywhere:
fn go_dark(mut theme: ResMut<WingsTheme>) {
    theme.mode = WingsThemeMode::Dark;
}
```

When `WingsTheme` changes, the painter repaints every painted entity once, cross fading over
`theme.motion.theme_transition`. In all other frames it only touches entities whose paint,
content color, opacity or local theme changed. `WingsThemeMode::System` follows the window's
dark mode preference. The test `twenty_buttons_follow_dark_theme` in the widgets crate checks
exactly this scenario.

---

## Animation

```rust
// Animate any registered property of an entity.
commands.entity(card).animate::<ScaleProperty>(Vec2::splat(1.05), AnimationSpec::spring());
commands.entity(dot).animate_from_to::<OpacityProperty>(
    1.0, 0.2, AnimationSpec::millis(600).with_easing(WingsEasing::EaseInOut), Repeat::ping_pong(),
);
```

* **Specs:** `AnimationSpec::tween(secs)`, `millis(ms)`, `short()`, `medium()`, `long()`,
  `spring()`, `spring_with(SpringSpec::BOUNCY)`, `Snap`, plus `.with_easing()`, `.with_delay()`
  and `.stagger(index, step)` for cascades.
* **Easing:** `Linear`, `EaseIn`, `EaseOut`, `EaseInOut`, `Standard` (with accelerate and
  decelerate), `Emphasized` (with accelerate and decelerate), `CubicBezier`, `BackOut`,
  `ElasticOut`, `BounceOut`, `Steps(n)` and any Bevy `EaseFunction`.
* **Springs** are simulated with fixed sub steps and keep their velocity when the target
  changes mid flight, so interrupted animations never jump.
* **Built in properties:** background, text and border color (interpolated in Oklab), scale,
  rotation, translation, width, height and `WingsOpacity`.
* **Your own properties:** implement `WingsProperty` (a getter and a setter) and call
  `app.register_wings_property::<MyProperty>()`.
* **Keyframes:** `WingsKeyframes::new(0.6).at(0.5, a).at_with(1.0, b, easing)` with presets
  `shake`, `heartbeat` and `blink`.
* **Animated visibility:** `WingsAnimatedVisibility { visible, enter, exit }` with
  `WingsTransition::fade()`, `scale(0.9)`, `slide_vertically(24.0)` and `.and(...)` to combine.
  Hidden nodes end in `Display::None`, so they cost nothing in layout.
* **Completion:** `WingsAnimationFinished` is triggered on the entity when an animation ends.

Bevy UI has no node opacity, so Wings adds `WingsOpacity`. It multiplies into every painted
color of the subtree.

---

## Responsiveness

`WingsViewport` tracks the primary window (or a `WingsViewportOverride`) and classifies it:

| Width class | Logical width |
| --- | --- |
| `Compact` | below 600 |
| `Medium` | 600 to 839 |
| `Expanded` | 840 to 1199 |
| `Large` | 1200 to 1599 |
| `ExtraLarge` | 1600 and above |

```rust
// Different values per class, falling back to the closest smaller class.
let padding = WingsResponsive::new(12.0).medium(24.0).expanded(32.0);

commands.spawn((
    Node::default(),
    WingsResponsiveNode(WingsResponsive::new(node_patch(|n| n.flex_direction = FlexDirection::Column))
        .expanded(node_patch(|n| n.flex_direction = FlexDirection::Row))),
    WingsResponsiveVisibility::from(WidthClass::Expanded),
));
```

* `WingsResponsiveColumns` sets grid column counts per class.
* `WingsResponsiveComponent<C>` swaps any component per class
  (`app.register_responsive_component::<C>()`).
* `WingsAutoScale` scales the whole UI per class.
* `width_at_least(WidthClass::Expanded)` is a run condition, and `WidthClassChanged` is a
  message.

---

## Modifiers

```rust
commands.spawn((
    Node::default(),
    modifier()
        .fill_max_width()
        .padding(16.0)
        .background(ColorRole::SurfaceContainer)
        .shape(ShapeRole::Large)
        .elevation(2)
        .hover_scale(1.03),
));
```

Operations apply left to right and are folded into `Node`, paint, shape, elevation, transform
and opacity components only when the modifier changes. Available operations include size,
width, height, square, circle, fill max width, height and size, padding and margin variants,
background, border, shape, rounded, elevation, offset, absolute, scale, rotate, alpha, clip,
align self, z index, weight, aspect ratio, hover scale and `then` to append another chain.

---

## Custom UI shaders

All shaders are WESL files embedded in the binary and registered by `WingsShadersPlugin`:

| Material | Effect |
| --- | --- |
| `WingsRippleMaterial` | Expanding touch ripple from the pointer position, clipped to rounded corners |
| `WingsShimmerMaterial` | Diagonal loading shimmer for skeletons |
| `WingsCircularProgressMaterial` | Anti aliased arc, determinate or rotating indeterminate |
| `WingsAuroraMaterial` | Animated three color aurora background |
| `WingsGlassMaterial` | Frosted glass panel with grain and a lit edge |

```rust
fn spawn_aurora(mut commands: Commands, mut materials: ResMut<Assets<WingsAuroraMaterial>>) {
    commands.spawn((
        Node { width: percent(100), height: percent(100), ..default() },
        MaterialNode(materials.add(WingsAuroraMaterial::new(
            Color::srgb(0.4, 0.3, 1.0),
            Color::srgb(0.1, 0.8, 0.9),
            Color::BLACK,
        ))),
    ));
}
```

Every shader is validated with naga in the test suite and the uniform layouts are checked
against the Rust structs.

---

## Navigation

The back stack is a resource of **your own key type**. Push, pop or rewrite it from any system.

```rust
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
enum Screen { Inbox, Message(u32), Settings }

app.add_plugins(WingsNavigationPlugin::new(Screen::Inbox, |key: &Screen| match key {
    Screen::Inbox => nav_entry(inbox_screen()).list_pane(),
    Screen::Message(id) => nav_entry(message_screen(*id)).detail_pane(),
    Screen::Settings => nav_entry(settings_screen()).with_transitions(WingsNavTransitions::fade()),
}));

// Where the current destination is rendered:
commands.spawn_scene(bsn! {
    WingsNavDisplay::<Screen>
    Node { width: percent(100), height: percent(100) }
});

fn open_message(mut stack: ResMut<WingsBackStack<Screen>>) {
    stack.push(Screen::Message(42));
}
```

* `WingsBackStack<K>`: `push`, `push_single_top`, `bring_to_top`, `pop` (never removes the
  root), `replace_top`, `pop_to(key, inclusive)`, `reset`, `top`, `entries`.
* `WingsNavCommand<K>` messages for event driven code and `WingsNavEvent<K>` messages when
  entries are spawned or removed.
* Entries spawn lazily. Covered entries stay alive but hidden, so scroll positions, text input
  and view models survive. Popped entries play their exit transition and are despawned.
* Transitions: `slide()` (default), `fade()`, `zoom()`, `none()`, per display or per
  destination.
* Adaptive panes: with the default `NavSceneStrategy::ListDetail`, a list destination and the
  detail destination above it are shown side by side from the `Expanded` width class.
* Back input: Escape, the browser back key and the mouse back button pop the stack (disable
  with `without_back_input()`).
* Each entry root carries a `WingsDiScope`, so scoped dependencies live exactly as long as the
  entry.

---

## Dependency injection

```rust
trait Clock: Send + Sync { fn now(&self) -> f64; }
struct SystemClock;
impl Clock for SystemClock { fn now(&self) -> f64 { 0.0 } }

struct Api { base_url: String }
struct Repository { api: Arc<Api>, clock: Arc<dyn Clock> }

let data = WingsModule::new("data")
    .single(|_| Api { base_url: "https://example.com".into() })
    .bind::<dyn Clock>(|_| Arc::new(SystemClock))
    .factory(|r| Repository { api: r.get(), clock: r.get() })
    .scoped(|_| SessionCache::default())
    .factory_with(|r, user_id: u64| ProfileLoader::new(r.get(), user_id));

app.add_plugins(WingsDiPlugin::new([data]));

fn load(repo: Inject<Repository>) {
    let repository = repo.get();
}
```

| Definition | Lifetime |
| --- | --- |
| `single` | One shared instance, created on first use |
| `single_eager` | One shared instance, created at startup |
| `instance` | A value you already have |
| `factory` | A new instance per request |
| `scoped` | One instance per scope (for example per navigation entry) |
| `factory_with` | A new instance per request, built from runtime parameters |
| `single_named`, `factory_named` | Qualified definitions resolved with `get_named` |
| `bind`, `bind_factory` | Trait objects (`Arc<dyn Trait>`) |

Resolution failures name the full dependency chain, cycles are detected, duplicate
definitions are rejected unless a module calls `allow_override()`, and `WingsDi::verify()`
builds every definition once so a test can catch wiring mistakes early. Despawning an entity
with `WingsDiScope` closes its scope.

---

## MVVM

```rust
#[derive(Component)]
struct CounterViewModel {
    count: i32,
    effects: WingsEffects<WingsShowSnackbar>,
}
impl WingsViewModel for CounterViewModel {}
impl WingsHasEffects<WingsShowSnackbar> for CounterViewModel {
    fn effects(&mut self) -> &mut WingsEffects<WingsShowSnackbar> { &mut self.effects }
}

app.register_view_model::<CounterViewModel>()
    .register_effects::<CounterViewModel, WingsShowSnackbar>();

// Provided by DI:
let module = WingsModule::new("ui").view_model(|_| CounterViewModel { count: 0, effects: default() });

fn counter_screen() -> impl Scene {
    let count = bind_text(|vm: &CounterViewModel| vm.count.to_string());
    let can_reset = bind_enabled(|vm: &CounterViewModel| vm.count > 0);
    bsn! {
        @WingsColumn { @gap: 12.0 }
        WingsProvideViewModel::<CounterViewModel>
        Children [
            @WingsText { @text: "0", @role: TypeRole::DisplayLarge } count
            --
            @WingsButton { @text: "Increment" }
            on(on_activate(|vm: &mut CounterViewModel| vm.count += 1))
            --
            @WingsButton { @text: "Reset", @variant: WingsButtonVariant::Outlined }
            can_reset
            on(on_activate(|vm: &mut CounterViewModel| {
                vm.count = 0;
                vm.effects.send(WingsShowSnackbar::new("Reset"));
            }))
        ]
    }
}
```

* **View model:** a component on the screen root. Mutating it marks it changed; that is the
  whole reactivity model.
* **Bindings:** `bind_text`, `bind_visible`, `bind_enabled`, `bind_checked` and the generic
  `bind::<VM, C>` (register with `register_binding`). Each finds the closest view model of its
  type up the hierarchy, recomputes only when it changed and writes only differing values.
* **Intents:** `on_activate` and `on_vm::<VM, E>` turn widget events into view model calls.
* **Effects:** `WingsEffects<E>` queues one shot events that are drained into Bevy messages.
* **Lifecycle:** `WingsViewModel::on_cleared` runs when the screen is despawned. Inside a
  navigation display, view models survive while their entry is in the back stack.

---

## Frame order

All Wings systems run in `Update`, in chained `WingsUiSystems` sets:

```
Input -> ViewModel -> Bind -> Navigation -> Responsive -> Modifier -> Style -> Theme -> Animate
```

Your own systems can be ordered against these sets. For example, put game logic that mutates
view models in `WingsUiSystems::ViewModel` and the bindings refresh in the same frame.

---

## Performance notes

* Every system is change driven: painters, bindings, responsive patches, modifiers, checked
  parts and interaction styles skip entities that did not change.
* Writes use `set_if_neq` or explicit comparisons, so downstream change detection does not
  cascade.
* The theme painter does a full pass only when the theme resource changes.
* Hidden navigation entries and animated visibility end in `Display::None`, removing them from
  layout.
* Shader effects are plain `MaterialNode`s; ripples despawn themselves after their fade.

---

## Testing

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo test --workspace --doc --all-features --locked
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps --locked
```

Every crate has documented unit tests, and the widgets crate has integration tests that spawn
widgets from `bsn!` scenes in a headless app (`MinimalPlugins`, `WingsWidgetsPlugin::headless()`)
and assert on their components. The gallery example has a headless test that runs the complete
app: navigation, DI, MVVM and the data and domain layers included.

`--all-targets` also runs the focused UI example tests. Documentation tests run separately
because Cargo's `--all-targets` selection does not include them. To test without shaders:

```bash
cargo test --workspace --exclude wings-gallery --all-targets --no-default-features --locked
cargo test --workspace --exclude wings-gallery --doc --no-default-features --locked
```

The gallery is excluded here because its facade dependency enables shaders by default.

---

## Gallery example

```bash
cargo run -p wings-gallery
```

The gallery shows a theme switch in the top bar, twenty buttons that follow the theme,
selection controls, a text field, progress indicators, skeletons, a dialog, snackbars, a
navigation bar that becomes a rail on wide windows, a DI provided MVVM counter and a Notes
screen that loads data through a domain use case and an offline first repository. On Linux
the example enables X11 only.

---

## Focused examples

Run these commands from the repository root. Each file in `examples/` demonstrates one
feature independently, while `wings-gallery` combines them into an application.

| Feature | Run command | What to try |
| --- | --- | --- |
| Domain | `cargo run --example domain` | See validation errors and a use case creating an entity with a typed ID. Runs in the terminal. |
| Data | `cargo run --example data` | Observe cached reads, an offline write and synchronization after reconnecting. Runs in the terminal. |
| Dependency injection | `cargo run --example di` | Compare factories with a shared singleton and resolve a scoped service. Runs in the terminal. |
| Reactive state | `cargo run --example state` | Watch memoized values recompute only when the source version changes. Runs in the terminal. |
| Background loading | `cargo run --example loading` | Click Load, refresh again, then simulate a failure; previous data stays visible. |
| Animation | `cargo run --example animation` | Toggle the card's spring scale, including while it is moving. |
| Theme | `cargo run --example theme` | Switch light/dark mode and choose a green seed; every themed node repaints. |
| Responsive layout | `cargo run --example responsive` | Resize across 600, 840 and 1200 logical pixels to change the grid column count. |
| Modifiers | `cargo run --example modifiers` | Hover the card; its modifier chain applies layout, paint, corners, elevation and spring scale. |
| Widgets | `cargo run --example widgets` | Try the animated checkbox, switch, radio group, slider track clicks, text field and snackbar button. Slider and text changes print to the terminal. |
| MVVM | `cargo run --example mvvm` | Increment/reset the counter; intents mutate a component view model and the text binding updates. |
| Navigation | `cargo run --example navigation` | Open the detail pane and go back with its button or Escape. Resize below 840 px for a single pane. |
| Shaders | `cargo run --example shaders` | Inspect aurora, procedural glass, shimmer, circular progress and the button's press ripple. Requires the default `shaders` feature. |

UI examples open a 960×720 window. They require a display and a working GPU driver. On Linux,
install the native build dependencies before building (Ubuntu/Debian):

```bash
sudo apt-get update
sudo apt-get install -y libudev-dev libx11-dev libxi-dev libxrandr-dev libxcursor-dev libxkbcommon-dev
```

For a terminal-only smoke run of any focused UI example, append `-- --headless`:

```bash
cargo run --example mvvm -- --headless
cargo run --example responsive -- --headless
cargo run --example widgets --no-default-features -- --headless
```

This runs 120 fixed frames without opening a window. Shader headless mode constructs materials;
visual rendering and real input still need a window. The example tests exercise spring targets,
theme repainting, responsive layout, modifiers, MVVM intents, loading, navigation and snackbars:

```bash
cargo test -p dga-bevy-wings-framework --examples --all-features --locked
```

## CI and crates.io releases

[CI](.github/workflows/ci.yml) runs on pushes to `main` and pull requests. It checks formatting,
Clippy with warnings denied, unit/integration/example tests, documentation tests and documentation
builds on Linux and macOS. Separate jobs check the no-shaders configuration, Rust 1.97.1, release
metadata and a complete publication dry run. Keep `Cargo.lock` committed so `--locked` checks
use the same dependency versions.

[Publish to crates.io](.github/workflows/publish.yml) runs when you publish a GitHub release.
It validates the release tag and reruns CI before uploading all **13 library crates**. Cargo
publishes the workspace in dependency order; `wings-gallery` has `publish = false` and is
explicitly excluded. See [Cargo's publishing documentation](https://doc.rust-lang.org/cargo/commands/cargo-publish.html).

One-time setup:

1. Push the complete project, including `Cargo.lock`, `scripts/` and `.github/workflows/`, to
   `doublegarts/dga_bevy_wings_framework` on GitHub and enable Actions.
2. Create a crates.io API token with publication permission for all the `dga-bevy-wings-*`
   packages, including permission to create the names for the first release.
3. Add it to GitHub **Settings → Secrets and variables → Actions → New repository secret**,
   named **`CARGO_REGISTRY_TOKEN`**. Keep the token out of source files.

Before the first release, run **Actions → Publish to crates.io → Run workflow** on `main`
with **dry_run checked**. This verifies every package without uploading or requiring a token.
You can also verify locally while your checkout has uncommitted changes:

```bash
python3 scripts/check-release.py --tag v0.1.0
cargo publish --workspace --exclude wings-gallery --dry-run --allow-dirty --locked
```

To publish version 0.1.0, commit and push the reviewed changes, wait for CI to pass, then
create and publish a GitHub release with the tag **`v0.1.0`** targeting that commit. The release
workflow publishes the crates. This setup does not upload anything until you publish a release
or manually run the publish workflow on a version tag with **dry_run unchecked**.

For later releases, update `[workspace.package].version` and every internal dependency version
in `[workspace.dependencies]` in `Cargo.toml`, update the changelog/checklist, refresh `Cargo.lock`
with Cargo, and use the matching `vX.Y.Z` release tag. The release script rejects mismatched tags
or crate versions. crates.io versions cannot be overwritten; if publication stops partway,
inspect the workflow log and registry before retrying to confirm which packages were uploaded.

## Roadmap

See [VERSION_0_1.MD](VERSION_0_1.MD) for the full feature and task checklist of this release and
what is planned next.

---

## License

MIT, see [LICENSE](LICENSE).
