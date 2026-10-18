# SwiftUI reference measurements

Measured on 2026-10-18 with Xcode 27 on macOS 27 and the iPhone 18 Pro simulator (iOS 27),
using `Reference.swift` (`METRIC` lines are SwiftUI frames in points). The Wings iOS and macOS
skins use these values.

## Colors

The system colors are identical on iOS 27 and macOS 27 (sRGB):

| Color | Light | Dark |
| --- | --- | --- |
| systemBlue | #0088FF | #0091FF |
| systemRed | #FF383C | #FF4245 |
| systemOrange | #FF8D28 | #FF9230 |
| systemYellow | #FFCC00 | #FFD600 |
| systemGreen | #34C759 | #30D158 |
| systemIndigo | #6155F5 | #6D7CFF |
| systemGray | #8E8E93 | #98989D (macOS) |

macOS only: `controlAccentColor` #007AFF in both appearances (SwiftUI's default tint on Mac),
`windowBackgroundColor` and `controlBackgroundColor` white / #1E1E1E,
`underPageBackgroundColor` #F6F6F6 / #282828, `labelColor` 85% black / 85% white,
`secondaryLabelColor` 50% / 55%, `tertiaryLabelColor` 26% / 25%, `separatorColor` 10%,
`systemFill` 10%, `secondarySystemFill` 8%, `tertiarySystemFill` 5%.

## Fonts

iOS (`.SFUI`): largeTitle 34, title1 28, title2 22, title3 20, headline 17 semibold, body 17,
callout 16, subheadline 15, footnote 13, caption1 12, caption2 11.

macOS (`.SFNS`): largeTitle 26, title1 22, title2 17, title3 15, headline 13 bold, body 13,
callout 12, subheadline 11, footnote 10, caption1 10, caption2 10.

## iOS 27 controls

| Control | Size (pt) | Notes |
| --- | --- | --- |
| Button regular | h 34 | capsule; prominent: tint fill, white label; bordered: gray fill, tint label |
| Button large / small | h 50 / 28 | capsule |
| Destructive prominent | h 34 | red fill |
| Glass / glass prominent | h 34 | white glass capsule / tint glass capsule |
| Toggle (switch) | 61 x 28 | green track, white pill thumb about 38 x 24 |
| Slider | h 31 | thin track, white pill thumb about 38 x 24 with shadow |
| Linear progress | h 4 | capsule, tint on gray fill |
| Spinner | 20 (large 37) | 8 spokes |
| Rounded text field | h 34 | white, hairline border |
| Segmented picker | h 31 | gray capsule track, white selected capsule |
| Stepper | h 32 | gray capsule, minus and plus |
| Inset grouped list row | h 52 | sections inset 16, corner about 26, separators inset to the text |
| Navigation bar | large title 34 bold at x 16, glass capsule toolbar buttons h 44 |
| Tab bar | floating glass capsule about 360 x 62, inset 21 from the sides, bottom safe area 83 |
| Alert | width about 318, corner about 34, title 17 semibold and message 15 left aligned, two capsule buttons side by side h 48 |

## macOS 27 controls

| Control | Size (pt) | Notes |
| --- | --- | --- |
| Push / bordered button | h 24 | rounded rect (radius about 6), gray fill |
| Prominent button | h 24 | accent fill, white label |
| Large / small button | h 28 / 20 | large is a capsule |
| Switch | 54 x 24 | accent track, white pill thumb; in forms it is smaller |
| Checkbox | 14 x 14 | radius about 3, accent fill, white check |
| Slider | h 16 | thin track, small white pill thumb |
| Linear progress | about 6 | capsule |
| Spinner | 32 | 12 spokes |
| Text field | h 24 | white, hairline border, radius about 6 |
| Segmented picker | h 24 | gray track, accent filled selection |
| Sidebar | 220 wide | rows about 32 pitch, accent icons |
| Toolbar | 52 tall | title 15 semibold, glass circle buttons 36 |
| Grouped form | sections #F6F6F6 / #282828, radius about 10, rows about 37 |
