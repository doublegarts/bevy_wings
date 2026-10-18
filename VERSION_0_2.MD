# Wings Framework: version 0.2 plan

Legend:

* `[x]` implemented **and** covered by automated tests (`cargo test --workspace`).
* `[~]` implemented and checked by hand on a real display or simulator, only partly verifiable
  headless (needs a GPU, a window or a device).
* `[-]` blocked on an upstream dependency.
* `[ ]` not done yet.

Tests run headless with `MinimalPlugins`. Anything that needs a renderer (shaders on screen,
Liquid Glass, screenshots) is checked by the visual regression harness or by hand and marked
`[~]` even when the logic around it is tested.

---

## Version 0.2.0

### Designs

- [x] `WingsDesign`: Material, iOS, macOS and Fluent; `Cupertino` resolves per platform
- [x] One widget API, per design skins in a `Skin` system set; designs switch at runtime
- [x] Apple colors (iOS and macOS system colors, labels, fills, grouped backgrounds), type
      ramps, radii, spacing, motion (springs by response and damping), measured against a
      SwiftUI reference app on iOS 27 and macOS 27 (`tools/swiftui-reference`)
- [x] Translucent Apple and Fluent colors blended in sRGB like the platforms
- [x] System UI font on Apple platforms (San Francisco, never bundled), Inter fallback
- [x] Fluent 2 colors, type ramp, radii and controls
- [x] `wings.toml`: `ui = "system"` (Material on Android and Linux, Fluent on Windows,
      Cupertino on iOS and macOS) or `ui = "custom"` (one design everywhere), appearance,
      accent, custom fonts, radii, spacing and color roles; `wings_config!` embeds it
- [x] iOS: large titles that collapse on scroll, inset grouped lists, capsule tab bar,
      toolbar buttons, alerts, sheets with detents, switches, segmented controls
- [x] macOS: unified transparent title bar, sidebar, push buttons, switches, pop up buttons
- [x] Narrow windows collapse macOS sidebars and Fluent navigation panes to their icons
- [~] Liquid Glass: floating chrome on an overlay camera (following the app camera's render
      target, so it also works for images and second windows), backdrop capture with a dual Kawase
      blur, `WingsLiquidGlassMaterial` refraction and rim light (checked on macOS and the iOS
      26 simulator)
- [~] Side by side comparisons with SwiftUI screenshots (iOS 26 simulator, macOS)

### Safe areas and screens

- [x] `WingsSafeArea`: notches, Dynamic Island, home indicator, status, navigation and
      gesture bars, display cutouts, foldable hinges, keyboard, macOS title bar
- [x] Scaffolds apply insets by themselves; `WingsSafeAreaPadding` for any node
- [x] Device profiles and previews (iPhone, iPad, Android phone and tablet, foldable, TV,
      Mac, desktop, Vision, landscape variants)
- [~] Native insets read from iOS (winit), Android (content rect) and macOS title bars
      (`native` feature), checked on the iOS 26 simulator and macOS
- [x] Right to left layouts (`WingsLayoutDirection`, `WingsNoMirror`)

### Data and domain

- [x] File backed data source (`FileSource`, atomic writes, any `Codec`)
- [x] Database backed data source (`DatabaseSource` on redb, feature `database`)
- [x] HTTP remote source (`HttpRemote` over any `WingsHttpClient`, `UreqClient` with feature
      `http`), status codes mapped to domain errors; runs on Bevy's async tasks through `load`
- [x] Serialization of entities for persistence (`Codec`, `JsonCodec`, serde for `Id<T>` and
      paging types)
- [x] Use case pipelines: retry with backoff, timeout, logging, mapping (`UseCaseExt`)
- [x] `Loadable` bindings in the MVVM crate (`bind_loading`, `bind_refreshing`,
      `bind_loaded`, `bind_failed`, `bind_error_text`)

### Widgets

- [-] `.bsn` asset files for widget scenes: Bevy 0.20 has no `.bsn` asset loader yet
      ("in the future" per the Bevy 0.20 notes); scenes stay in `bsn!` until it ships
- [x] Virtualized lists and grids (`WingsVirtualList`, `WingsVirtualGrid`): only visible rows
      exist, 10,000 item test, 100,000 row benchmark
- [x] Tabs, segmented buttons, menus, dropdowns and tooltips
- [x] Date and time pickers
- [x] Bottom sheets and side sheets with scrims, detents and dismissal
- [x] Focus ring styling for keyboard navigation, per design
- [x] Gamepad navigation: D-pad and stick move focus between all interactive widgets, South
      activates, East closes popups and sheets and pops the navigation stack; arrow keys too

### Navigation

- [x] Navigation results (`WingsNavigator::pop_with_result`, `WingsNavResultReceived`)
- [x] Deep links (`WingsDeepLinks`, custom schemes and web links, command line links)
- [x] Saving and restoring the back stack (`WingsBackStackPersistencePlugin`, file and memory
      stores)
- [x] Shared element transitions (`WingsSharedElement`)
- [x] Transitions per design (iOS parallax push, macOS fade, Fluent entrance, Material slide)

### Theming

- [x] Theme definitions loaded from `.wings.toml` asset files (`WingsThemeFilePlugin`)
- [~] Hot reload while the app runs (needs Bevy's `file_watcher`; the gallery enables it)

### Shaders

- [~] Blur behind panels (backdrop capture and Liquid Glass)
- [~] Gradient borders (`WingsGradientBorderMaterial`)
- [~] Noise backgrounds (`WingsNoiseMaterial`)
- [x] Every shader validated with naga; uniform layouts checked against the Rust structs

### Quality

- [x] Benchmarks with very large widget counts
      (`cargo bench -p bevy_wings_ui_widgets --bench stress`)
- [~] Screenshot based visual regression tests (`scripts/visual-regression.py`, `Visual
      regression` workflow on a macOS runner): 48 cases rendered offscreen with a fixed frame
      time, identical across runs locally; baselines are seeded from the runner
- [x] Workspace at version 0.2.0, release check passes

### Known limitations

* iOS 27 SDK: apps linked against it must adopt the UIScene life cycle, which winit 0.30 (used
  by Bevy 0.20) does not do yet, so iOS builds run on the iOS 26 SDK and simulator for now.
* On iOS the drawable covers the whole screen and insets come from winit's safe area; there is
  no native UIKit chrome (Wings draws its own Liquid Glass).
* Screenshot baselines depend on fonts and GPU drivers and are kept per platform.

---

## Version 0.3.0 (ideas)

- [ ] `.bsn` asset files for widget scenes once Bevy ships the loader
- [ ] Native platform text input (IME composition on every platform)
- [ ] Accessibility tree for screen readers (AccessKit roles and labels on every widget)
- [ ] Pull to refresh and swipe actions on list rows
- [ ] Context menus and long press menus
- [ ] Charts (line, bar, pie) themed per design
- [ ] Localization helpers (string tables, plural rules, number and date formats)
- [ ] Android: Material You dynamic colors from the wallpaper
