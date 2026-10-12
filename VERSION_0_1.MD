# Wings Framework: version plan

Legend:

* `[x]` implemented **and** covered by automated tests (`cargo test --workspace`).
* `[~]` implemented and compiled, but only partly verifiable here (needs a GPU or a window).
* `[ ]` not done yet.

Tests run headless with `MinimalPlugins`, so anything that needs a real renderer or a window
(drawing shaders on screen, OS dark mode, clicking with a real mouse) is marked `[~]` even when
the logic around it is tested.

---

## Version 0.1.0

### Workspace and tooling

- [x] Repository and main crate named `bevy_wings` (package `bevy_wings`)
- [x] Crates grouped by layer: `crates/domain`, `crates/data`, `crates/presentation`, `crates/shared`
- [x] Cargo workspace with 12 library crates, a facade crate and an example
- [x] Bevy 0.20 only, Rust 1.97.1 minimum, edition 2024
- [x] `missing_docs` lint enabled for every crate, `unsafe_code` forbidden
- [x] Clippy clean (`cargo clippy --workspace --all-targets`)
- [x] Facade crate `bevy_wings` with module re-exports for every layer and one `prelude`
- [x] `WingsUiPlugins` plugin group (core, animation, theme, responsive, modifier, shaders, widgets)
- [x] `shaders` cargo feature (default on), crates build with it off
- [x] README with a guide for every crate
- [x] This checklist

### Domain layer (`bevy_wings_domain`)

- [x] No dependencies (plain Rust)
- [x] Typed ids `Id<T>` and the `DomainEntity` trait
- [x] `UseCase` trait and closure use cases
- [x] `DomainError` with retryable classification and field errors
- [x] `Validator<T>` reporting every broken rule, plus reusable rules
- [x] `ReadRepository` and `WriteRepository` contracts with default paging and `require`
- [x] `PageRequest` and `Page<T>`
- [x] `Loadable<T>` that keeps the previous value while loading and after failures

### Data layer (`bevy_wings_data`)

- [x] `DataSource` contract and thread safe `MemorySource`
- [x] `RemoteSource` contract and `FakeRemote` (offline switch, call counter)
- [x] `OfflineFirstRepository` with cache first, remote first, cache only and remote only policies
- [x] Freshness with a time to live and an injectable clock
- [x] Stale data served when the remote is unavailable
- [x] Offline writes applied locally, queued and delivered by `sync()`
- [x] `Mapper` for transfer objects
- [x] Background loads on the async compute pool into a `WingsLoadable<T>` component, with `WingsLoadFinished`

### Presentation: core (`bevy_wings_ui_core`)

- [x] `WingsUiSystems` chained system sets (Input, ViewModel, Bind, Navigation, Responsive, Modifier, Style, Theme, Animate)
- [x] `WingsCorePlugin` and `ensure_core` so every plugin can be added alone
- [x] `WingsState<T>` versioned reactive value (set only bumps on real changes)
- [x] `WingsMemo<T>` version keyed cache
- [x] `WingsOpacity` subtree opacity with clamping and effective opacity
- [x] Hierarchy helpers `ancestors` and `find_ancestor_with`

### Presentation: animation (`bevy_wings_ui_animation`)

- [x] 16 easing curves including cubic bezier, back, elastic, bounce, steps and Bevy `EaseFunction`
- [x] `AnimationSpec`: tween, millis, short, medium, long, spring, snap, delay, stagger
- [x] Spring physics with fixed sub steps and velocity preserving retargeting
- [x] `Repeat`: once, times, infinite, ping pong
- [x] `WingsLerp` for f32, vectors, `Color` (Oklab), `Rot2`, `Val` and tuples
- [x] `WingsAnimatedValue<T>` standalone animated value
- [x] `WingsProperty` trait and `register_wings_property` for custom properties
- [x] Built in properties: background, text and border color, scale, rotation, translation, width, height, opacity
- [x] `animate`, `animate_from_to`, `stop_animation` entity command extensions
- [x] `WingsAnimationFinished` entity event
- [x] Keyframes with per segment easing and presets (shake, heartbeat, blink)
- [x] `WingsAnimatedVisibility` with fade, scale and slide enter and exit transitions

### Presentation: theme (`bevy_wings_ui_theme`)

- [x] 38 color roles plus custom roles
- [x] Light and dark schemes, seeded schemes from one color (Oklch tonal palettes)
- [x] Contrast checks (role pairs reach at least 3:1)
- [x] Typography with 15 type roles and scaling
- [x] Shape, spacing, elevation (box shadows) and motion tokens
- [x] `WingsThemeMode` light, dark and system
- [~] Following the OS dark mode preference (logic tested with a fake window, real OS not testable here)
- [x] Paint components for background, text, border and content color, with inheritance
- [x] State layers blended over fills
- [x] Incremental painter: full pass only on theme change, otherwise changed entities only
- [x] Cross fade on theme switch
- [x] Local light or dark subtrees (`WingsLocalTheme`)
- [x] 20 buttons switch from light to dark with one theme change (tested in theme and widgets crates)

### Presentation: responsiveness (`bevy_wings_ui_responsive`)

- [x] Width classes (compact, medium, expanded, large, extra large), height classes, orientation
- [x] Configurable breakpoints
- [x] Viewport from the primary window or `WingsViewportOverride`
- [x] `WidthClassChanged` message and `width_at_least` run condition
- [x] `WingsResponsive<T>` per class values with fallback
- [x] Responsive node patches, visibility, grid columns and components
- [x] Automatic UI scale per class

### Presentation: modifiers (`bevy_wings_ui_modifiers`)

- [x] Chainable `WingsModifier` with layout, paint, shape, elevation, transform and opacity operations
- [x] Folding only when the modifier changes, later operations win
- [x] Hover scale feedback

### Presentation: custom UI shaders (`bevy_wings_ui_shaders`)

- [x] Ripple, shimmer, circular progress, aurora and glass WESL shaders, validated with naga
- [x] Uniform layouts checked against the Rust material structs
- [x] Helper math (ripple center and radius, progress clamping) tested
- [~] Rendering on screen (needs a GPU, not available in the test environment)

### Presentation: widgets (`bevy_wings_ui_widgets`)

- [x] `WingsText`, `WingsIcon` (icon tint follows the content color)
- [x] `WingsColumn`, `WingsRow`, `WingsBox`, `WingsSpacer`, `WingsDivider`, `WingsScrollColumn`, `WingsGrid`
- [x] `WingsSurface`, `WingsCard` (elevated, filled, outlined)
- [x] `WingsButton` with six variants, icons, disabled state, full width
- [x] Runtime variant changes restyle buttons
- [x] `WingsIconButton`, `WingsFab` (small, regular, large, extended)
- [x] Hover, press and disabled styling through one system
- [x] `WingsCheckbox`, `WingsSwitch` (spring animated thumb), `WingsRadioGroup` and `WingsRadioButton`, `WingsChip`
- [x] Data driven checked parts (`WingsCheckedPart`, `WingsCheckedOwner`)
- [x] `WingsSlider` with fill and thumb layout
- [x] `WingsTextField` with label, initial value, change events and focus outline
- [x] `WingsLinearProgress` determinate (animated) and indeterminate (sweep)
- [~] `WingsCircularProgress` (shader driven)
- [x] `WingsSkeleton` with shimmer, and opacity pulse fallback
- [x] `WingsBadge` with counts and dots
- [x] Snackbars: message, host, action button, auto hide and despawn
- [x] `WingsDialog` with scrim, animated open and close, result event
- [x] `WingsScaffold`, `WingsTopAppBar`, `WingsListItem`
- [x] `WingsNavigationBar` and `WingsNavigationRail` swap by width class
- [x] `WingsNavigationItem` exclusive selection with index event
- [~] Press ripple from the pointer position (spawning logic compiled, visuals need a GPU)
- [x] Headless mode for tests and servers

### Presentation: navigation (`bevy_wings_ui_navigation`)

- [x] `WingsBackStack<K>` resource with push, single top, bring to top, pop, replace, pop to, reset
- [x] Navigation graph from a key to a `bsn!` scene destination
- [x] `WingsNavDisplay<K>` lazy spawning, hidden but alive covered entries, despawn on pop
- [x] Slide, fade, zoom and no transitions, per display or per destination
- [x] List and detail panes side by side on expanded windows
- [x] `WingsNavCommand<K>` and `WingsNavEvent<K>` messages
- [x] Back on Escape and mouse back button
- [x] One DI scope per entry

### Shared: dependency injection (`bevy_wings_di`)

- [x] Modules with single, eager single, instance, factory, scoped and parameterized definitions
- [x] Named qualifiers
- [x] Trait object bindings
- [x] Cycle detection and readable error chains
- [x] Duplicate detection with explicit override
- [x] `verify()` to build every definition in a test
- [x] `Inject<T>` system parameter, world extension, entity scopes closed on despawn

### Presentation: MVVM (`bevy_wings_ui_mvvm`)

- [x] `WingsViewModel` components with `on_cleared` lifecycle
- [x] Text, visible, enabled, checked and generic component bindings
- [x] Bindings resolve the closest view model and write only on change
- [x] Intents with `on_activate` and `on_vm`
- [x] One shot effects drained into messages
- [x] View models provided by the DI container (scoped when inside a navigation entry)
- [x] Bindings and intents inside `bsn!` scenes

### Example

- [x] `wings-gallery` builds and its headless test runs the whole app (navigation, DI, MVVM)
- [x] Gallery Notes screen loads through a domain use case and an offline first repository
- [~] Running the gallery in a window (needs a display, X11 enabled)

---

## Version 0.2.0 (planned)

- [ ] File and database backed data sources
- [ ] HTTP remote source on top of Bevy's async tasks
- [ ] Serialization of entities for persistence
- [ ] Use case pipelines (retry, timeout, logging) as decorators
- [ ] `Loadable` bindings in the MVVM crate
- [ ] `.bsn` asset files for widget scenes once Bevy ships the asset format
- [ ] Lazy, virtualized lists and grids for thousands of rows
- [ ] Navigation results (return a value to the previous destination)
- [ ] Deep links and saving and restoring the back stack
- [ ] Shared element transitions between destinations
- [ ] Tabs, segmented buttons, menus, dropdowns and tooltips
- [ ] Date and time pickers
- [ ] Bottom sheets and side sheets
- [ ] Focus ring styling for keyboard navigation
- [ ] Right to left layout support
- [ ] Theme definitions loaded from asset files with hot reload
- [ ] More shaders: blur behind panels, gradient borders, noise backgrounds
- [ ] Gamepad navigation polish
- [ ] Benchmarks with very large widget counts
- [ ] Screenshot based visual regression tests on a GPU runner
