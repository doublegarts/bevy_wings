//! Safe areas: the parts of the screen that system UI, hardware or the keyboard cover.
//!
//! [`WingsSafeArea`] describes, in logical pixels:
//!
//! * [`insets`](WingsSafeArea::insets): status bars, notches, the Dynamic Island, home
//!   indicators, Android system bars and display cutouts, rounded screen corners;
//! * [`title_bar`](WingsSafeArea::title_bar): a transparent, full size content title bar on
//!   desktops (macOS);
//! * [`keyboard`](WingsSafeArea::keyboard): the on-screen keyboard;
//! * [`overscan`](WingsSafeArea::overscan): the TV "title safe" margin;
//! * [`folds`](WingsSafeArea::folds): hinges and folds of foldable devices;
//! * [`corner_radius`](WingsSafeArea::corner_radius): the screen corner radius, for
//!   concentric corners.
//!
//! It is filled from the platform (iOS, iPadOS, visionOS, Android, macOS) when the `native`
//! feature is on, or from a [`WingsDeviceProfile`] preview, so phone, tablet, foldable, TV and
//! desktop layouts can be checked on any machine:
//!
//! ```ignore
//! // Preview an iPhone 18 Pro in landscape on a desktop.
//! app.insert_resource(WingsDevicePreview::new(WingsDeviceProfile::IPHONE_PRO.landscape()));
//! ```
//!
//! UI reads it through [`WingsSafeAreaPadding`], which adds the insets of chosen edges to a
//! node's padding (the Wings scaffold does this for its bars and content automatically, like
//! SwiftUI), or directly through the resource.

use bevy::prelude::*;
use bevy::ui::{Node, UiRect, Val};

use crate::{WingsViewport, WingsViewportOverride};

/// Distances from the four edges, in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Default, Reflect)]
pub struct WingsInsets {
    /// Top edge.
    pub top: f32,
    /// Right edge.
    pub right: f32,
    /// Bottom edge.
    pub bottom: f32,
    /// Left edge.
    pub left: f32,
}

/// Edge wise sum.
impl core::ops::Add for WingsInsets {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self::new(
            self.top + other.top,
            self.right + other.right,
            self.bottom + other.bottom,
            self.left + other.left,
        )
    }
}

impl WingsInsets {
    /// No insets.
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0, 0.0);

    /// Insets in CSS order: top, right, bottom, left.
    pub const fn new(top: f32, right: f32, bottom: f32, left: f32) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    /// The same inset on every edge.
    pub const fn all(v: f32) -> Self {
        Self::new(v, v, v, v)
    }

    /// Vertical and horizontal insets.
    pub const fn symmetric(vertical: f32, horizontal: f32) -> Self {
        Self::new(vertical, horizontal, vertical, horizontal)
    }

    /// Edge wise maximum.
    pub fn max(self, other: Self) -> Self {
        Self::new(
            self.top.max(other.top),
            self.right.max(other.right),
            self.bottom.max(other.bottom),
            self.left.max(other.left),
        )
    }

    /// Keeps only the selected edges.
    pub fn only(self, edges: WingsEdges) -> Self {
        Self::new(
            if edges.top { self.top } else { 0.0 },
            if edges.right { self.right } else { 0.0 },
            if edges.bottom { self.bottom } else { 0.0 },
            if edges.left { self.left } else { 0.0 },
        )
    }

    /// The insets rotated 90 degrees clockwise (portrait to landscape with the top of the
    /// device on the left).
    pub fn rotated(self) -> Self {
        Self::new(self.left, self.top, self.right, self.bottom)
    }

    /// As a Bevy [`UiRect`].
    pub fn to_ui_rect(self) -> UiRect {
        UiRect {
            left: Val::Px(self.left),
            right: Val::Px(self.right),
            top: Val::Px(self.top),
            bottom: Val::Px(self.bottom),
        }
    }

    /// True when every edge is zero.
    pub fn is_zero(self) -> bool {
        self == Self::ZERO
    }
}

/// A set of edges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub struct WingsEdges {
    /// Top edge.
    pub top: bool,
    /// Right edge.
    pub right: bool,
    /// Bottom edge.
    pub bottom: bool,
    /// Left edge.
    pub left: bool,
}

impl Default for WingsEdges {
    fn default() -> Self {
        Self::ALL
    }
}

impl WingsEdges {
    /// Every edge.
    pub const ALL: Self = Self {
        top: true,
        right: true,
        bottom: true,
        left: true,
    };
    /// No edge.
    pub const NONE: Self = Self {
        top: false,
        right: false,
        bottom: false,
        left: false,
    };
    /// Top only.
    pub const TOP: Self = Self {
        top: true,
        ..Self::NONE
    };
    /// Bottom only.
    pub const BOTTOM: Self = Self {
        bottom: true,
        ..Self::NONE
    };
    /// Left and right.
    pub const HORIZONTAL: Self = Self {
        left: true,
        right: true,
        ..Self::NONE
    };
    /// Top and bottom.
    pub const VERTICAL: Self = Self {
        top: true,
        bottom: true,
        ..Self::NONE
    };
    /// Left, right and bottom.
    pub const SIDES_AND_BOTTOM: Self = Self {
        top: false,
        ..Self::ALL
    };
    /// Left, right and top.
    pub const SIDES_AND_TOP: Self = Self {
        bottom: false,
        ..Self::ALL
    };
}

/// The safe area of the primary window. See the [module docs](self).
#[derive(Resource, Debug, Clone, PartialEq, Default)]
pub struct WingsSafeArea {
    /// System UI and hardware: status bar, notch, Dynamic Island, home indicator, Android
    /// system bars and display cutouts, rounded corners.
    pub insets: WingsInsets,
    /// Height of a transparent title bar that content extends under (macOS full size content
    /// windows). Toolbars that sit in the title bar ignore it.
    pub title_bar: f32,
    /// Height of the on-screen keyboard, `0` when hidden.
    pub keyboard: f32,
    /// TV overscan ("title safe") margins. Only [`WingsSafeAreaPadding`] with `overscan` uses
    /// them.
    pub overscan: WingsInsets,
    /// Hinges and folds of foldable devices, in logical window coordinates. Content should not
    /// cross them.
    pub folds: Vec<Rect>,
    /// Radius of the screen corners, for concentric corners of floating chrome.
    pub corner_radius: f32,
}

impl WingsSafeArea {
    /// Everything content must avoid: system insets, title bar on top and the keyboard at the
    /// bottom (whichever is larger).
    pub fn content(&self) -> WingsInsets {
        let mut i = self.insets;
        i.top += self.title_bar;
        i.bottom = i.bottom.max(self.keyboard);
        i
    }

    /// True when the keyboard is visible.
    pub fn keyboard_visible(&self) -> bool {
        self.keyboard > 0.0
    }

    /// Sets the keyboard height (for platforms that report it through your own code, or for
    /// tests).
    pub fn set_keyboard(&mut self, height: f32) {
        self.keyboard = height.max(0.0);
    }
}

/// Kind of screen a [`WingsDeviceProfile`] describes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub enum WingsScreenKind {
    /// A phone.
    Phone,
    /// A tablet.
    Tablet,
    /// A foldable phone, unfolded.
    Foldable,
    /// A desktop or laptop window.
    Desktop,
    /// A television (10 foot UI).
    Tv,
    /// A headset window (visionOS).
    Spatial,
}

/// A device's screen size and safe area, for previews and tests.
#[derive(Debug, Clone, PartialEq, Reflect)]
pub struct WingsDeviceProfile {
    /// Display name.
    pub name: &'static str,
    /// Kind of screen.
    pub kind: WingsScreenKind,
    /// Logical size in portrait (or the native orientation).
    pub size: Vec2,
    /// System insets in that orientation.
    pub insets: WingsInsets,
    /// System insets in landscape, when the device rotates.
    pub landscape_insets: Option<WingsInsets>,
    /// Screen corner radius.
    pub corner_radius: f32,
    /// On-screen keyboard height in portrait (0 for devices without one).
    pub keyboard: f32,
    /// Overscan margins (TVs).
    pub overscan: WingsInsets,
    /// A vertical fold at this x position (foldables).
    pub fold_x: Option<f32>,
    /// Shows a Dynamic Island in previews.
    pub dynamic_island: bool,
    /// Shows a home indicator in previews.
    pub home_indicator: bool,
    /// Transparent title bar height (desktops).
    pub title_bar: f32,
    /// Profile is rotated to landscape.
    pub landscape: bool,
}

impl WingsDeviceProfile {
    const BASE: Self = Self {
        name: "",
        kind: WingsScreenKind::Phone,
        size: Vec2::ZERO,
        insets: WingsInsets::ZERO,
        landscape_insets: None,
        corner_radius: 0.0,
        keyboard: 0.0,
        overscan: WingsInsets::ZERO,
        fold_x: None,
        dynamic_island: false,
        home_indicator: false,
        title_bar: 0.0,
        landscape: false,
    };

    /// iPhone with a Dynamic Island (iPhone 18 Pro class, 402 by 874).
    pub const IPHONE_PRO: Self = Self {
        name: "iPhone Pro",
        size: Vec2::new(402.0, 874.0),
        insets: WingsInsets::new(62.0, 0.0, 34.0, 0.0),
        landscape_insets: Some(WingsInsets::new(0.0, 62.0, 21.0, 62.0)),
        corner_radius: 55.0,
        keyboard: 336.0,
        dynamic_island: true,
        home_indicator: true,
        ..Self::BASE
    };
    /// iPhone with a notch (390 by 844).
    pub const IPHONE_NOTCH: Self = Self {
        name: "iPhone (notch)",
        size: Vec2::new(390.0, 844.0),
        insets: WingsInsets::new(47.0, 0.0, 34.0, 0.0),
        landscape_insets: Some(WingsInsets::new(0.0, 47.0, 21.0, 47.0)),
        corner_radius: 47.0,
        keyboard: 336.0,
        home_indicator: true,
        ..Self::BASE
    };
    /// iPhone with a home button (375 by 667).
    pub const IPHONE_SE: Self = Self {
        name: "iPhone SE",
        size: Vec2::new(375.0, 667.0),
        insets: WingsInsets::new(20.0, 0.0, 0.0, 0.0),
        landscape_insets: Some(WingsInsets::ZERO),
        keyboard: 260.0,
        ..Self::BASE
    };
    /// 11 inch iPad (834 by 1210).
    pub const IPAD: Self = Self {
        name: "iPad",
        kind: WingsScreenKind::Tablet,
        size: Vec2::new(834.0, 1210.0),
        insets: WingsInsets::new(24.0, 0.0, 20.0, 0.0),
        landscape_insets: Some(WingsInsets::new(24.0, 0.0, 20.0, 0.0)),
        corner_radius: 18.0,
        keyboard: 313.0,
        home_indicator: true,
        ..Self::BASE
    };
    /// Android phone with a punch hole camera and gesture navigation (412 by 915).
    pub const ANDROID_PHONE: Self = Self {
        name: "Android phone",
        size: Vec2::new(412.0, 915.0),
        insets: WingsInsets::new(32.0, 0.0, 24.0, 0.0),
        landscape_insets: Some(WingsInsets::new(0.0, 0.0, 24.0, 32.0)),
        corner_radius: 32.0,
        keyboard: 300.0,
        ..Self::BASE
    };
    /// Android phone with three button navigation (412 by 915).
    pub const ANDROID_PHONE_BUTTONS: Self = Self {
        name: "Android phone (buttons)",
        insets: WingsInsets::new(32.0, 0.0, 48.0, 0.0),
        landscape_insets: Some(WingsInsets::new(0.0, 48.0, 0.0, 32.0)),
        ..Self::ANDROID_PHONE
    };
    /// Android tablet (800 by 1280).
    pub const ANDROID_TABLET: Self = Self {
        name: "Android tablet",
        kind: WingsScreenKind::Tablet,
        size: Vec2::new(800.0, 1280.0),
        insets: WingsInsets::new(24.0, 0.0, 48.0, 0.0),
        landscape_insets: Some(WingsInsets::new(24.0, 0.0, 48.0, 0.0)),
        corner_radius: 20.0,
        keyboard: 360.0,
        ..Self::BASE
    };
    /// Foldable, unfolded, with a vertical hinge (673 by 841).
    pub const FOLDABLE: Self = Self {
        name: "Foldable",
        kind: WingsScreenKind::Foldable,
        size: Vec2::new(673.0, 841.0),
        insets: WingsInsets::new(28.0, 0.0, 24.0, 0.0),
        landscape_insets: Some(WingsInsets::new(28.0, 0.0, 24.0, 0.0)),
        corner_radius: 24.0,
        keyboard: 320.0,
        fold_x: Some(336.5),
        ..Self::BASE
    };
    /// 1080p television with a 5% title safe margin.
    pub const TV: Self = Self {
        name: "TV",
        kind: WingsScreenKind::Tv,
        size: Vec2::new(1920.0, 1080.0),
        overscan: WingsInsets::new(54.0, 96.0, 54.0, 96.0),
        landscape: true,
        ..Self::BASE
    };
    /// Mac window with a transparent title bar.
    pub const MAC_WINDOW: Self = Self {
        name: "Mac window",
        kind: WingsScreenKind::Desktop,
        size: Vec2::new(1100.0, 760.0),
        corner_radius: 26.0,
        title_bar: 28.0,
        landscape: true,
        ..Self::BASE
    };
    /// Windows or Linux desktop window.
    pub const DESKTOP_WINDOW: Self = Self {
        name: "Desktop window",
        kind: WingsScreenKind::Desktop,
        size: Vec2::new(1280.0, 800.0),
        corner_radius: 8.0,
        landscape: true,
        ..Self::BASE
    };
    /// visionOS window.
    pub const VISION_WINDOW: Self = Self {
        name: "Vision window",
        kind: WingsScreenKind::Spatial,
        size: Vec2::new(1280.0, 720.0),
        corner_radius: 46.0,
        landscape: true,
        ..Self::BASE
    };

    /// Every built in profile.
    pub const ALL: [WingsDeviceProfile; 12] = [
        Self::IPHONE_PRO,
        Self::IPHONE_NOTCH,
        Self::IPHONE_SE,
        Self::IPAD,
        Self::ANDROID_PHONE,
        Self::ANDROID_PHONE_BUTTONS,
        Self::ANDROID_TABLET,
        Self::FOLDABLE,
        Self::TV,
        Self::MAC_WINDOW,
        Self::DESKTOP_WINDOW,
        Self::VISION_WINDOW,
    ];

    /// The profile rotated to landscape (no change for devices that are already landscape).
    pub fn landscape(&self) -> Self {
        if self.landscape {
            return self.clone();
        }
        let mut p = self.clone();
        p.landscape = true;
        p.size = Vec2::new(self.size.y, self.size.x);
        p.insets = self.landscape_insets.unwrap_or(self.insets.rotated());
        p.keyboard = (self.keyboard * 0.62).round();
        p.fold_x = self.fold_x.map(|_| p.size.x * 0.5);
        p
    }

    /// The safe area this profile describes, with the keyboard shown or hidden.
    pub fn safe_area(&self, keyboard_visible: bool) -> WingsSafeArea {
        WingsSafeArea {
            insets: self.insets,
            title_bar: self.title_bar,
            keyboard: if keyboard_visible { self.keyboard } else { 0.0 },
            overscan: self.overscan,
            folds: self
                .fold_x
                .map(|x| vec![Rect::new(x, 0.0, x, self.size.y)])
                .unwrap_or_default(),
            corner_radius: self.corner_radius,
        }
    }

    /// Looks a profile up by a short name: `iphone`, `iphone-notch`, `iphone-se`, `ipad`,
    /// `android`, `android-buttons`, `android-tablet`, `foldable`, `tv`, `mac`, `desktop`,
    /// `vision`. A `-landscape` suffix rotates it.
    pub fn by_name(name: &str) -> Option<Self> {
        let (base, landscape) = match name.trim().to_ascii_lowercase() {
            n if n.ends_with("-landscape") => (n.trim_end_matches("-landscape").to_string(), true),
            n => (n, false),
        };
        let p = match base.as_str() {
            "iphone" | "iphone-pro" => Self::IPHONE_PRO,
            "iphone-notch" => Self::IPHONE_NOTCH,
            "iphone-se" => Self::IPHONE_SE,
            "ipad" => Self::IPAD,
            "android" | "android-phone" => Self::ANDROID_PHONE,
            "android-buttons" => Self::ANDROID_PHONE_BUTTONS,
            "android-tablet" => Self::ANDROID_TABLET,
            "foldable" | "fold" => Self::FOLDABLE,
            "tv" => Self::TV,
            "mac" => Self::MAC_WINDOW,
            "desktop" | "windows" | "linux" => Self::DESKTOP_WINDOW,
            "vision" => Self::VISION_WINDOW,
            _ => return None,
        };
        Some(if landscape { p.landscape() } else { p })
    }
}

/// Previews a device on any machine: the viewport takes the profile's size and the safe area
/// its insets. Set [`keyboard`](Self::keyboard) to simulate the on-screen keyboard.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct WingsDevicePreview {
    /// The previewed device.
    pub profile: WingsDeviceProfile,
    /// Simulates the on-screen keyboard.
    pub keyboard: bool,
    /// Draws the device's status bar, Dynamic Island and home indicator over the UI.
    pub decorations: bool,
    /// Color of the simulated status bar text and home indicator.
    pub ink: Color,
}

impl WingsDevicePreview {
    /// Previews `profile` with decorations and without keyboard.
    pub fn new(profile: WingsDeviceProfile) -> Self {
        Self {
            profile,
            keyboard: false,
            decorations: true,
            ink: Color::BLACK,
        }
    }
}

/// Adds the safe area insets of some edges to a node's padding, like SwiftUI content that
/// respects the safe area. Whatever padding the node has on its own is kept.
///
/// ```ignore
/// commands.spawn((Node { padding: UiRect::all(px(16)), ..default() }, WingsSafeAreaPadding::default()));
/// ```
#[derive(Component, Debug, Clone, Copy, PartialEq, Reflect)]
#[require(Node)]
pub struct WingsSafeAreaPadding {
    /// Edges that respect the safe area.
    pub edges: WingsEdges,
    /// Include the on-screen keyboard at the bottom.
    pub keyboard: bool,
    /// Include the title bar at the top.
    pub title_bar: bool,
    /// Include TV overscan margins.
    pub overscan: bool,
    /// Insets added by the last update (kept so the node's own padding survives).
    pub applied: WingsInsets,
}

impl Default for WingsSafeAreaPadding {
    fn default() -> Self {
        Self::edges(WingsEdges::ALL)
    }
}

impl WingsSafeAreaPadding {
    /// Respects the safe area on `edges` (with keyboard, title bar and overscan).
    pub fn edges(edges: WingsEdges) -> Self {
        Self {
            edges,
            keyboard: true,
            title_bar: true,
            overscan: true,
            applied: WingsInsets::ZERO,
        }
    }

    /// The insets this component asks for under a safe area.
    pub fn wanted(&self, safe: &WingsSafeArea) -> WingsInsets {
        let mut i = safe.insets;
        if self.title_bar {
            i.top += safe.title_bar;
        }
        if self.keyboard {
            i.bottom = i.bottom.max(safe.keyboard);
        }
        if self.overscan {
            i = i.max(safe.overscan);
        }
        i.only(self.edges)
    }
}

fn px_of(v: Val) -> f32 {
    match v {
        Val::Px(p) => p,
        _ => 0.0,
    }
}

/// Adds the difference between `wanted` and `applied` to a padding edge.
fn shift(edge: &mut Val, applied: f32, wanted: f32) {
    if applied == wanted {
        return;
    }
    *edge = Val::Px(px_of(*edge) - applied + wanted);
}

/// Applies [`WingsSafeAreaPadding`] when the safe area or the component changes.
pub fn apply_safe_area_padding(
    safe: Res<WingsSafeArea>,
    mut nodes: Query<(&mut WingsSafeAreaPadding, &mut Node)>,
) {
    let safe_changed = safe.is_changed();
    for (mut pad, mut node) in &mut nodes {
        let wanted = pad.wanted(&safe);
        if !safe_changed && wanted == pad.applied {
            continue;
        }
        let applied = pad.applied;
        shift(&mut node.padding.top, applied.top, wanted.top);
        shift(&mut node.padding.right, applied.right, wanted.right);
        shift(&mut node.padding.bottom, applied.bottom, wanted.bottom);
        shift(&mut node.padding.left, applied.left, wanted.left);
        if pad.applied != wanted {
            pad.applied = wanted;
        }
    }
}

/// Fills [`WingsSafeArea`] from a [`WingsDevicePreview`], when one is set, and sizes the
/// viewport like the previewed device.
pub fn apply_device_preview(
    preview: Option<Res<WingsDevicePreview>>,
    mut safe: ResMut<WingsSafeArea>,
    mut commands: Commands,
    mut had_preview: Local<bool>,
) {
    let Some(preview) = preview else {
        if *had_preview {
            commands.remove_resource::<WingsViewportOverride>();
            *had_preview = false;
        }
        return;
    };
    if !preview.is_changed() {
        return;
    }
    *had_preview = true;
    let area = preview.profile.safe_area(preview.keyboard);
    if *safe != area {
        *safe = area;
    }
    commands.insert_resource(WingsViewportOverride {
        width: preview.profile.size.x,
        height: preview.profile.size.y,
    });
}

/// Reads the safe area from the platform: iOS, iPadOS and visionOS (window frame minus the
/// UIKit safe area), Android (window minus the content rectangle, which covers system bars,
/// cutouts and an `adjustResize` keyboard) and macOS (transparent full size title bars).
#[cfg(feature = "native")]
pub fn read_native_safe_area(
    preview: Option<Res<WingsDevicePreview>>,
    windows: Query<(Entity, &Window), With<bevy::window::PrimaryWindow>>,
    mut safe: ResMut<WingsSafeArea>,
    _main_thread: bevy::ecs::system::NonSendMarker,
) {
    if preview.is_some() {
        return;
    }
    let Ok((entity, window)) = windows.single() else {
        return;
    };
    let mut area = safe.clone();
    area.title_bar =
        if cfg!(target_os = "macos") && window.titlebar_transparent && window.fullsize_content_view
        {
            MAC_TITLE_BAR
        } else {
            0.0
        };
    #[cfg(any(target_os = "ios", target_os = "visionos", target_os = "tvos"))]
    {
        bevy::winit::WINIT_WINDOWS.with_borrow(|windows| {
            let Some(w) = windows.get_window(entity) else {
                return;
            };
            let scale = w.scale_factor() as f32;
            let (Ok(inner), Ok(outer)) = (w.inner_position(), w.outer_position()) else {
                return;
            };
            let (inner_size, outer_size) = (w.inner_size(), w.outer_size());
            let top = (inner.y - outer.y) as f32 / scale;
            let left = (inner.x - outer.x) as f32 / scale;
            let bottom = (outer_size.height as f32 - inner_size.height as f32) / scale - top;
            let right = (outer_size.width as f32 - inner_size.width as f32) / scale - left;
            // A full screen window (borderless fullscreen) draws edge to edge, so every inset
            // applies. Otherwise winit 0.30 sizes the drawable to the safe area anchored at the
            // top left: only the top and leading insets overlap it.
            let full_screen = window.physical_height() >= outer_size.height.saturating_sub(1);
            area.insets = if full_screen {
                WingsInsets::new(top.max(0.0), right.max(0.0), bottom.max(0.0), left.max(0.0))
            } else {
                WingsInsets::new(top.max(0.0), 0.0, 0.0, left.max(0.0))
            };
            area.corner_radius = if bottom > 0.0 { 47.0 } else { 0.0 };
        });
    }
    #[cfg(target_os = "android")]
    {
        if let Some(app) = bevy::android::ANDROID_APP.get() {
            let scale = window.scale_factor();
            let rect = app.content_rect();
            let (w, h) = (
                window.physical_width() as f32,
                window.physical_height() as f32,
            );
            if rect.right > rect.left && rect.bottom > rect.top {
                let top = rect.top as f32 / scale;
                let left = rect.left as f32 / scale;
                let right = (w - rect.right as f32).max(0.0) / scale;
                let bottom_total = (h - rect.bottom as f32).max(0.0) / scale;
                // A large bottom inset is the keyboard pushing the content up.
                let (bottom, keyboard) = if bottom_total > 120.0 {
                    (area.insets.bottom.min(bottom_total), bottom_total)
                } else {
                    (bottom_total, 0.0)
                };
                area.insets = WingsInsets::new(top, right, bottom, left);
                area.keyboard = keyboard;
            }
        }
    }
    let _ = (entity, window);
    if *safe != area {
        *safe = area;
    }
}

/// Height of a standard macOS title bar.
pub const MAC_TITLE_BAR: f32 = 28.0;

/// Marks the simulated status bar, Dynamic Island and home indicator a
/// [`WingsDevicePreview`] draws.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct WingsDevicePreviewDecoration;

/// Draws (or removes) the previewed device's Dynamic Island, status bar clock and home
/// indicator on top of the UI.
pub fn draw_device_preview(
    mut commands: Commands,
    preview: Option<Res<WingsDevicePreview>>,
    existing: Query<Entity, With<WingsDevicePreviewDecoration>>,
    viewport: Res<WingsViewport>,
) {
    let changed = preview.as_ref().is_some_and(|p| p.is_changed()) || viewport.is_changed();
    if !changed && (preview.is_some() || existing.is_empty()) {
        return;
    }
    for e in &existing {
        commands.entity(e).despawn();
    }
    let Some(preview) = preview else {
        return;
    };
    if !preview.decorations {
        return;
    }
    let p = &preview.profile;
    let ink = preview.ink;
    let root = commands
        .spawn((
            Node {
                position_type: bevy::ui::PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                ..default()
            },
            bevy::ui::GlobalZIndex(i32::MAX - 1),
            Pickable::IGNORE,
            WingsDevicePreviewDecoration,
        ))
        .id();
    if p.dynamic_island && !p.landscape {
        commands.spawn((
            Node {
                position_type: bevy::ui::PositionType::Absolute,
                top: Val::Px(11.0),
                left: Val::Px(p.size.x * 0.5 - 63.0),
                width: Val::Px(126.0),
                height: Val::Px(37.0),
                border_radius: bevy::ui::BorderRadius::MAX,
                ..default()
            },
            BackgroundColor(Color::BLACK),
            Pickable::IGNORE,
            ChildOf(root),
        ));
    }
    if p.insets.top >= 20.0 && !p.landscape {
        commands.spawn((
            Node {
                position_type: bevy::ui::PositionType::Absolute,
                top: Val::Px((p.insets.top - 17.0) * 0.5),
                left: Val::Px(if p.dynamic_island { 52.0 } else { 24.0 }),
                ..default()
            },
            Text::new("9:41"),
            bevy::text::TextFont {
                font_size: bevy::text::FontSize::Px(17.0),
                weight: bevy::text::FontWeight::SEMIBOLD,
                ..default()
            },
            TextColor(ink),
            Pickable::IGNORE,
            ChildOf(root),
        ));
    }
    if p.home_indicator {
        commands.spawn((
            Node {
                position_type: bevy::ui::PositionType::Absolute,
                bottom: Val::Px(8.0),
                left: Val::Px(viewport.width * 0.5 - 67.0),
                width: Val::Px(134.0),
                height: Val::Px(5.0),
                border_radius: bevy::ui::BorderRadius::MAX,
                ..default()
            },
            BackgroundColor(ink),
            Pickable::IGNORE,
            ChildOf(root),
        ));
    }
    if let Some(x) = p.fold_x {
        commands.spawn((
            Node {
                position_type: bevy::ui::PositionType::Absolute,
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                left: Val::Px(x - 0.5),
                width: Val::Px(1.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.12)),
            Pickable::IGNORE,
            ChildOf(root),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Landscape rotates the size and uses the landscape insets.
    #[test]
    fn landscape_profiles() {
        let p = WingsDeviceProfile::IPHONE_PRO.landscape();
        assert_eq!(p.size, Vec2::new(874.0, 402.0));
        assert_eq!(p.insets.left, 62.0);
        assert_eq!(p.insets.top, 0.0);
        assert_eq!(WingsDeviceProfile::TV.landscape(), WingsDeviceProfile::TV);
        assert_eq!(
            WingsDeviceProfile::by_name("ipad-landscape").unwrap().size,
            Vec2::new(1210.0, 834.0)
        );
        assert!(WingsDeviceProfile::by_name("toaster").is_none());
    }

    /// Content insets combine system insets, title bar and keyboard.
    #[test]
    fn content_insets() {
        let mut area = WingsDeviceProfile::IPHONE_PRO.safe_area(false);
        assert_eq!(area.content().bottom, 34.0);
        area.set_keyboard(336.0);
        assert_eq!(area.content().bottom, 336.0);
        let mac = WingsDeviceProfile::MAC_WINDOW.safe_area(false);
        assert_eq!(mac.content().top, 28.0);
    }

    /// Padding keeps the node's own padding and follows safe area changes.
    #[test]
    fn padding_follows_safe_area() {
        let mut app = App::new();
        app.init_resource::<WingsSafeArea>()
            .add_systems(Update, apply_safe_area_padding);
        let e = app
            .world_mut()
            .spawn((
                Node {
                    padding: UiRect::all(Val::Px(16.0)),
                    ..default()
                },
                WingsSafeAreaPadding::edges(WingsEdges::VERTICAL),
            ))
            .id();
        app.world_mut().resource_mut::<WingsSafeArea>().insets =
            WingsInsets::new(62.0, 0.0, 34.0, 0.0);
        app.update();
        let pad = app.world().get::<Node>(e).unwrap().padding;
        assert_eq!(pad.top, Val::Px(78.0));
        assert_eq!(pad.bottom, Val::Px(50.0));
        assert_eq!(pad.left, Val::Px(16.0));
        app.world_mut()
            .resource_mut::<WingsSafeArea>()
            .set_keyboard(300.0);
        app.update();
        assert_eq!(
            app.world().get::<Node>(e).unwrap().padding.bottom,
            Val::Px(316.0)
        );
        app.world_mut().resource_mut::<WingsSafeArea>().insets = WingsInsets::ZERO;
        app.world_mut()
            .resource_mut::<WingsSafeArea>()
            .set_keyboard(0.0);
        app.update();
        assert_eq!(
            app.world().get::<Node>(e).unwrap().padding,
            UiRect::all(Val::Px(16.0))
        );
    }

    /// TV overscan only applies where asked.
    #[test]
    fn tv_overscan() {
        let area = WingsDeviceProfile::TV.safe_area(false);
        let pad = WingsSafeAreaPadding::default();
        assert_eq!(pad.wanted(&area).left, 96.0);
        let no_overscan = WingsSafeAreaPadding {
            overscan: false,
            ..WingsSafeAreaPadding::default()
        };
        assert_eq!(no_overscan.wanted(&area), WingsInsets::ZERO);
    }

    /// Foldables report their hinge.
    #[test]
    fn foldable_hinge() {
        let area = WingsDeviceProfile::FOLDABLE.safe_area(false);
        assert_eq!(area.folds.len(), 1);
        assert_eq!(area.folds[0].min.x, 336.5);
    }
}
