# Changelog

All notable changes to the Wings framework. Every crate of the workspace shares one version.

## 0.2.0 (2026-10-20)

Wings 0.2 makes apps look and feel native on every platform: one widget API with the
platform's own design, Liquid Glass on iOS and macOS, safe areas for every screen, touch and
gamepad input, and a much richer navigation, data and widget set.

### Highlights

- **The platform's own design.** One widget API, four designs: Material (Android, Linux),
  Cupertino with Liquid Glass (iOS and iPadOS, macOS) and Fluent (Windows). The Apple designs
  were measured against SwiftUI on iOS 27 and macOS 27. `wings.toml` picks the platform's
  design (`ui = "system"`) or one custom design for every platform (`ui = "custom"`), and the
  design can change at runtime.
- **Liquid Glass.** Floating top bars, tab bars, sheets and menus on real glass: an overlay
  camera, a dual Kawase backdrop blur and a refracting glass material. iOS 26 navigation bars
  with glass capsule buttons, the soft scroll edge effect, and large and inline titles that hand
  over with the scroll.
- **Safe areas everywhere.** Notches, the Dynamic Island, home indicators, Android system and
  gesture bars, display cutouts, foldable hinges, the keyboard and macOS title bars, read from
  the platform (`native` feature), with device previews on the desktop.
- **Touch, keyboard and gamepad.** Kinetic touch scrolling with momentum, reliable quick taps,
  focus rings per design, gamepad navigation, and one back pipeline: Escape, gamepad B,
  Android's back button and gesture and the iOS edge swipe close the topmost menu, dialog or
  sheet before going back a screen, and Android stays open at the first screen.
- **Right to left** layouts.

### Added

- Designs: `WingsDesign` (Material, iOS, macOS, Fluent), Apple and Fluent color schemes, type
  ramps, shapes, spacing and motion; `wings.toml` with `wings_config!` and
  `WingsUiPlugins::configured`; theme files (`.wings.toml`) with hot reload
  (`WingsThemeFilePlugin`).
- Widgets: segmented buttons and tabs, dropdowns, menu buttons, tooltips, date and time pickers,
  bottom and side sheets, virtualized lists and grids (`WingsVirtualList`), focus rings, glass
  (`WingsGlass`), soft scroll edge (`WingsScrollEdge`), list items with detail text and
  accessories, destructive dialogs, large titles.
- Input: touch scrolling with momentum (`WingsTouchScroll`), gamepad navigation
  (`WingsGamepadNavigation`), shared back handling (`WingsBack`, `WingsBackSystems`).
- Navigation: results (`WingsNavigator::pop_with_result`), deep links (`WingsDeepLinks`),
  saving and restoring the back stack (`WingsBackStackPersistencePlugin`), shared element
  transitions (`WingsSharedElement`), transitions per design, page slides between tabs
  (`nav_entry(..).tab(i)`), Android system back and the interactive iOS edge swipe
  (`WingsEdgeSwipeBack`).
- Responsive: `WingsSafeArea`, `WingsSafeAreaPadding`, device profiles and previews,
  `WingsLayoutDirection` (right to left); compact sidebars on narrow windows.
- Data: `FileSource`, `DatabaseSource` (redb, feature `database`), `HttpRemote` with
  `UreqClient` (feature `http`), `Codec` and `JsonCodec`.
- Domain: use case pipelines (retry with backoff, timeout, logging, mapping) through
  `UseCaseExt`; serde support for ids and paging (feature `serde`).
- MVVM: `Loadable` bindings (`bind_loading`, `bind_refreshing`, `bind_loaded`, `bind_failed`,
  `bind_error_text`).
- Shaders: `WingsLiquidGlassMaterial`, `WingsScrollEdgeMaterial`, `WingsGradientBorderMaterial`,
  `WingsNoiseMaterial` and the backdrop blur.
- Quality: stress benchmarks (`cargo bench -p bevy_wings_ui_widgets --bench stress`), a
  screenshot regression harness (`scripts/visual-regression.py`) with a macOS GPU workflow,
  312 tests.

### Fixed

- Taps whose touch starts and ends within one frame (quick taps, simulator clicks) activate
  widgets instead of leaving them pressed.
- Navigation bar items share the bar width equally, however long their labels are.

### Upgrading from 0.1

- **The default look follows the platform.** Apps on iOS and macOS now look like Cupertino and
  on Windows like Fluent. To keep Material everywhere, add a `wings.toml` with `ui = "custom"`
  and `design = "material"` (and use `WingsUiPlugins::configured(wings_config!())`), or start
  from `WingsThemePlugin::with_theme(WingsTheme::for_design(WingsDesign::Material))`.
- **New system set.** `WingsUiSystems::Skin` runs between `Responsive` and `Modifier`, so
  `WingsUiSystems::ORDER` has 10 entries. Exhaustive `match`es on `WingsUiSystems` need the new
  arm.
- **New color roles.** `ColorRole` gained `Fill`, `SecondaryFill`, `TertiaryFill`,
  `TertiaryLabel`, `GroupedBackground` and `SecondaryGroupedBackground`
  (`ColorRole::BUILT_IN` has 44 entries). Exhaustive `match`es need the new arms.
- **New public fields.** Code that builds these structs field by field needs the new fields or
  `..default()`: `WingsBackInput` (`gamepad_back`, `system_back`), `WingsTheme` (`design`,
  `seed`), `WingsDialogProps` and `WingsDialogButton` (`destructive`), `WingsTopAppBarProps`
  (`title_display`), `WingsListItemProps` (`detail`, `accessory`). Props set through `bsn!`
  (`@prop: value`) are unaffected.
- **Back handling.** Escape and the browser back key close the topmost menu, dialog or sheet
  first; only a press that no overlay used pops the navigation stack. Custom overlays can join
  through `WingsBack::consume` in `WingsBackSystems::Overlays`.
- **Default features.** The facade enables the new `native` feature (platform safe areas) by
  default. Touch scrolling, gamepad navigation and, on iOS, the edge swipe are on by default;
  turn them off with `WingsTouchScroll`, `WingsGamepadNavigation` and `WingsEdgeSwipeBack`.
- **Transitions.** Navigation displays that keep the default transitions now use the design's
  transitions (iOS parallax push, macOS cross fade, Fluent entrance, Material slide). Set
  `WingsNavDisplay::transitions` to keep a fixed style.
- **System functions.** Several public systems gained parameters (for example `back_input`,
  `sync_nav_displays`, `typography_system` and the widget skin systems). This only matters when
  calling them directly instead of through the plugins.

### Known limitations

- `.bsn` scene asset files wait for Bevy's `.bsn` loader; scenes stay in `bsn!`.
- Apps linked against the iOS 27 SDK must adopt the UIScene life cycle, which winit 0.30 (Bevy
  0.20) does not support yet: build iOS apps with the iOS 26 SDK for now.
- On Android 13 and newer, set `android:enableOnBackInvokedCallback="false"` in the app
  manifest so the back gesture reaches the app.
- Screenshot baselines for the macOS workflow are seeded from the first workflow run.

## 0.1.0

First release: layered data, domain and presentation crates, dependency injection, ECS
theming, animation, responsive layouts, modifiers, custom UI shaders, navigation, MVVM and the
Material widget set. See [VERSION_0_1.MD](VERSION_0_1.MD).
