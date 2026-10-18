# Wings 0.2: Apple (SwiftUI) design

Status: approved in conversation on 2026-10-18. This file records the decisions so the
implementation can be checked against them.

## Goal

Apps built with Wings look like native SwiftUI apps on Apple devices without code changes:
the iOS look on iPhone and iPad, the macOS look on Mac, matching the current Liquid Glass era
(iOS and macOS 26 and later). Material stays the default on every other platform.

## Decisions

1. **Per platform.** `WingsDesign::{Material, Ios, MacOs}`. `WingsDesign::native()` picks
   `MacOs` on macOS, `Ios` on iOS and visionOS, `Material` elsewhere.
2. **Liquid Glass era.** Floating glass bars, tab bar and buttons; true backdrop blur when the
   `shaders` feature is on, translucent fills otherwise.
3. **One API with design skins.** Widget names and props do not change. Scenes spawn meaning
   (marker, variant, label, value). A skin per design applies metrics, shapes, palette and
   decorative parts. Changing `WingsTheme::design` at runtime re-skins every live widget.

## Architecture

- `WingsDesign` is a field of `WingsTheme`. `WingsTheme::default()` uses `native()`.
  `WingsTheme::material()`, `ios()`, `macos()` force one. `set_design` swaps every token set
  to that design's defaults and keeps the mode and the accent. `from_seed` is design aware:
  Material builds tonal palettes, Apple designs use the seed as the single tint.
- Apple semantic colors map onto existing roles (`Primary` = tint, `Surface` = system
  background, `OnSurface` = label, `OnSurfaceVariant` = secondary label, `Outline` = opaque
  separator, `OutlineVariant` = separator, `Error`/`Success`/`Warning` = system red, green,
  orange). New roles valid in every design: `Fill`, `SecondaryFill`, `TertiaryFill`,
  `TertiaryLabel`, `GroupedBackground`, `SecondaryGroupedBackground`.
- New set `WingsUiSystems::Skin` between `Responsive` and `Modifier`. Skin systems run for
  newly spawned widgets and for every widget when the design changes; they mark modifiers
  changed afterwards so user modifiers still win. Decorative skin children carry
  `WingsSkinPart` and are rebuilt on a design change.
- Press feedback: Material keeps state layers and ripples. iOS dims the control while
  pressed (no hover, no ripple). macOS darkens the fill while held.
- Navigation default transition: iOS slides in from the trailing edge with the previous
  screen shifting a little, macOS cross fades, Material keeps its default.

## Tokens

Values were taken from Apple's published system colors and text styles and are verified
against a SwiftUI reference app (`tools/swiftui-reference`, not published) on the iOS
simulator and on macOS.

| Wings role | iOS | macOS |
| --- | --- | --- |
| DisplayLarge | Large Title 34 bold | 26 bold |
| DisplayMedium | Large Title 34 | 26 |
| DisplaySmall | Title 1 28 | 22 |
| HeadlineLarge | Title 1 28 bold | 22 bold |
| HeadlineMedium / Small | Title 2 22 / Title 3 20 | 17 / 15 |
| TitleLarge / Medium | Headline 17 semibold | 13 bold |
| TitleSmall | Subheadline 15 semibold | 11 semibold |
| BodyLarge / BodyMedium | Body 17 | 13 |
| BodySmall | Footnote 13 | 10 |
| LabelLarge | Body 17 | 13 |
| LabelMedium / Small | Subheadline 15 / Caption 12 | 11 / 10 |

- Font: the system UI font (San Francisco) on Apple targets through Bevy's
  `system_font_discovery`, falling back to the bundled Inter. SF Pro and SF Symbols are never
  bundled.
- Shapes ExtraSmall..ExtraLarge: iOS 6, 10, 16, 26, 34; macOS 4, 6, 10, 16, 26. Full = capsule.
- Spacing: iOS 4, 8, 16, 20, 32; macOS 4, 8, 12, 20, 28.
- Elevation: soft shadows (about 12% alpha light), none on cards and grouped sections.
- Motion: springs (snappy about 0.3 s, smooth about 0.5 s), press dim in about 0.1 s.

## Widgets

| Widget | iOS | macOS |
| --- | --- | --- |
| Button Filled / Tonal / Outlined / Text / Elevated / Danger | borderedProminent / bordered tinted / bordered gray / borderless / glass / destructive prominent | accent push / push / push / borderless / glass / push with red label |
| IconButton | borderless, 44 hit area, glass circle in bars | borderless toolbar item |
| Fab | tinted glass circle | same, smaller |
| Checkbox | round checkmark | square checkbox |
| Switch | 64x28 green capsule, pill thumb | small switch, accent |
| Radio | inline picker rows with trailing checkmark | round radio |
| Slider | thin track, white pill thumb | thin track, round knob |
| Chip | capsule, tint at 15% when selected | same |
| TextField | filled rounded field, label as placeholder | white field, hairline, accent focus ring |
| Linear / circular progress | 4 pt capsule bar / spoke spinner or tint ring | 6 pt bar / spinner or ring |
| Skeleton | gray placeholders, soft pulse | same |
| Badge | red capsule | same, smaller |
| Snackbar | glass capsule toast | glass toast |
| Dialog | iOS alert | Mac alert |
| TopAppBar | large title collapsing to inline, glass back and action buttons | 52 tall unified toolbar |
| NavigationBar | floating glass tab bar | hidden (sidebar) |
| NavigationRail | glass sidebar (iPad, wide) | floating glass sidebar at every width |
| ListItem | inset grouped row, 44 min, chevron accessory | 28 tall row |
| Card / Surface | rounded panel on grouped background, no shadow | same |

Additive API: `WingsList` and `WingsListSection` (header, footer), `WingsListItem` accessory
(none, chevron, checkmark, detail text), `WingsTopAppBar` title display (automatic, large,
inline) with collapse on scroll, and `WingsGlass` for any node.

## Liquid Glass rendering

- Two UI layers on the same window: the app camera (content) and a Wings overlay camera
  (higher order, no clear, empty render layers) that renders only an overlay UI root.
- A render system in the overlay camera's `Core2d` schedule, before `ui_pass`, copies the
  window texture (which holds the finished content) and blurs it into a small backdrop.
- The glass material samples the backdrop at its screen position, bends it near the edges,
  adds a rim light, tint and adaptive brightness, and draws continuous corners.
- Apple skins move floating chrome (top bar, tab bar, sidebar, toasts, alerts) into the
  overlay root, remember the original parent and index, and restore them for Material.
- Fallback when shaders are off, headless, or disabled through `WingsGlassSettings`:
  translucent fill, hairline border, soft shadow.

## Verification

- Headless tests: tokens per design, contrast, skin metrics per widget, design switching at
  runtime, overlay layer setup and restore, fallbacks, shader validation with naga.
- On this Mac: run the gallery in each design and compare screenshots with the SwiftUI
  reference app.
