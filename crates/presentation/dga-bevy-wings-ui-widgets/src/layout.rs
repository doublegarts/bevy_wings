//! Layout containers.
//!
//! Containers only describe a [`Node`] plus theme paint, so children are attached by the caller
//! with the usual `Children [ ... ]` syntax:
//!
//! ```ignore
//! bsn! {
//!     @WingsColumn { @gap: 12.0, @padding: 16.0 }
//!     Children [
//!         @WingsText { @text: "Title", @role: TypeRole::TitleLarge }
//!         --
//!         @WingsButton { @text: "Continue" }
//!     ]
//! }
//! ```

use bevy::prelude::*;
use bevy::ui_widgets::ScrollArea;

use dga_bevy_wings_ui_theme::prelude::{
    ColorRole, ShapeRole, WingsBackground, WingsBorderPaint, WingsContentColor,
    WingsElevationLevel, WingsPaint, WingsShape,
};

/// Props shared by [`WingsColumn`], [`WingsRow`] and [`WingsBox`].
#[derive(Clone)]
pub struct WingsStackProps {
    /// Space between children in pixels.
    pub gap: f32,
    /// Uniform inner padding in pixels.
    pub padding: f32,
    /// Main axis arrangement.
    pub justify: JustifyContent,
    /// Cross axis alignment. Defaults to stretch for columns and center for rows.
    pub align: Option<AlignItems>,
    /// Width of the container.
    pub width: Val,
    /// Height of the container.
    pub height: Val,
    /// Wrap children onto new lines.
    pub wrap: bool,
}

impl Default for WingsStackProps {
    fn default() -> Self {
        Self {
            gap: 0.0,
            padding: 0.0,
            justify: JustifyContent::Start,
            align: None,
            width: Val::Auto,
            height: Val::Auto,
            wrap: false,
        }
    }
}

impl WingsStackProps {
    fn node(&self, direction: FlexDirection) -> Node {
        Node {
            display: Display::Flex,
            flex_direction: direction,
            row_gap: px(self.gap),
            column_gap: px(self.gap),
            padding: UiRect::all(px(self.padding)),
            justify_content: self.justify,
            align_items: self.align.unwrap_or(match direction {
                FlexDirection::Row | FlexDirection::RowReverse => AlignItems::Center,
                _ => AlignItems::Stretch,
            }),
            width: self.width,
            height: self.height,
            flex_wrap: if self.wrap {
                FlexWrap::Wrap
            } else {
                FlexWrap::NoWrap
            },
            ..default()
        }
    }
}

/// Vertical stack of children.
#[derive(SceneComponent, Default, Clone)]
#[scene(WingsStackProps)]
pub struct WingsColumn;

impl WingsColumn {
    fn scene(props: WingsStackProps) -> impl Scene {
        let node = props.node(FlexDirection::Column);
        bsn! { node }
    }
}

/// Horizontal stack of children. Children are centered vertically by default.
#[derive(SceneComponent, Default, Clone)]
#[scene(WingsStackProps)]
pub struct WingsRow;

impl WingsRow {
    fn scene(props: WingsStackProps) -> impl Scene {
        let node = props.node(FlexDirection::Row);
        bsn! { node }
    }
}

/// A container that stacks its children on top of each other (all children share the same
/// grid cell), useful for overlays and badges.
#[derive(SceneComponent, Default, Clone)]
#[scene(WingsStackProps)]
pub struct WingsBox;

impl WingsBox {
    fn scene(props: WingsStackProps) -> impl Scene {
        let node = Node {
            display: Display::Grid,
            grid_template_columns: vec![RepeatedGridTrack::fr(1, 1.0)],
            grid_template_rows: vec![RepeatedGridTrack::fr(1, 1.0)],
            padding: UiRect::all(px(props.padding)),
            justify_items: JustifyItems::Center,
            align_items: props.align.unwrap_or(AlignItems::Center),
            width: props.width,
            height: props.height,
            ..default()
        };
        bsn! { node WingsBoxLayout }
    }
}

/// Marker that places every direct child of a [`WingsBox`] in the same grid cell.
#[derive(Component, Default, Clone, Copy)]
pub struct WingsBoxLayout;

/// Places all children of [`WingsBox`] into grid cell (1, 1).
pub fn box_layout_system(
    boxes: Query<&Children, (With<WingsBoxLayout>, Changed<Children>)>,
    mut nodes: Query<&mut Node, Without<WingsBoxLayout>>,
) {
    for children in &boxes {
        for child in children.iter() {
            if let Ok(mut node) = nodes.get_mut(child)
                && (node.grid_row != GridPlacement::start(1)
                    || node.grid_column != GridPlacement::start(1))
            {
                node.grid_row = GridPlacement::start(1);
                node.grid_column = GridPlacement::start(1);
            }
        }
    }
}

/// Flexible empty space. With `weight > 0` it grows to fill the remaining space, otherwise it
/// has a fixed `size` along both axes.
#[derive(SceneComponent, Default, Clone)]
#[scene(WingsSpacerProps)]
pub struct WingsSpacer;

/// Props of [`WingsSpacer`].
#[derive(Default, Clone)]
pub struct WingsSpacerProps {
    /// Fixed size in pixels.
    pub size: f32,
    /// Flex grow factor.
    pub weight: f32,
}

impl WingsSpacer {
    fn scene(props: WingsSpacerProps) -> impl Scene {
        let node = Node {
            min_width: px(props.size),
            min_height: px(props.size),
            flex_grow: props.weight,
            ..default()
        };
        bsn! { node }
    }
}

/// Thin themed separator line.
#[derive(SceneComponent, Default, Clone)]
#[scene(WingsDividerProps)]
pub struct WingsDivider;

/// Props of [`WingsDivider`].
#[derive(Default, Clone)]
pub struct WingsDividerProps {
    /// Draw a vertical line (for rows) instead of a horizontal one.
    pub vertical: bool,
    /// Inset at both ends in pixels.
    pub inset: f32,
}

impl WingsDivider {
    fn scene(props: WingsDividerProps) -> impl Scene {
        let node = if props.vertical {
            Node {
                width: px(1),
                align_self: AlignSelf::Stretch,
                margin: UiRect::vertical(px(props.inset)),
                ..default()
            }
        } else {
            Node {
                height: px(1),
                align_self: AlignSelf::Stretch,
                margin: UiRect::horizontal(px(props.inset)),
                ..default()
            }
        };
        bsn! { node WingsBackground({WingsPaint::Role(ColorRole::OutlineVariant)}) }
    }
}

/// Vertically scrolling column (mouse wheel and touchpad).
#[derive(SceneComponent, Default, Clone)]
#[scene(WingsStackProps)]
pub struct WingsScrollColumn;

impl WingsScrollColumn {
    fn scene(props: WingsStackProps) -> impl Scene {
        let mut node = props.node(FlexDirection::Column);
        node.overflow = Overflow::scroll_y();
        bsn! { node ScrollArea }
    }
}

/// Fixed column grid. Combine with
/// [`WingsResponsiveColumns`](dga_bevy_wings_ui_responsive::WingsResponsiveColumns) for column
/// counts that follow the window width class.
#[derive(SceneComponent, Default, Clone)]
#[scene(WingsGridProps)]
pub struct WingsGrid;

/// Props of [`WingsGrid`].
#[derive(Clone)]
pub struct WingsGridProps {
    /// Number of equally sized columns.
    pub columns: u16,
    /// Gap between cells in pixels.
    pub gap: f32,
}

impl Default for WingsGridProps {
    fn default() -> Self {
        Self {
            columns: 2,
            gap: 8.0,
        }
    }
}

impl WingsGrid {
    fn scene(props: WingsGridProps) -> impl Scene {
        let node = Node {
            display: Display::Grid,
            grid_template_columns: RepeatedGridTrack::flex(props.columns.max(1), 1.0),
            row_gap: px(props.gap),
            column_gap: px(props.gap),
            ..default()
        };
        bsn! { node }
    }
}

/// A themed surface: fill, content color, shape and elevation. The base of cards, sheets,
/// dialogs and bars.
#[derive(SceneComponent, Default, Clone)]
#[scene(WingsSurfaceProps)]
pub struct WingsSurface;

/// Props of [`WingsSurface`].
#[derive(Clone)]
pub struct WingsSurfaceProps {
    /// Fill role. Its matching `on` role becomes the content color.
    pub color: ColorRole,
    /// Corner shape.
    pub shape: ShapeRole,
    /// Elevation level `0..=5`.
    pub elevation: u8,
    /// Inner padding in pixels.
    pub padding: f32,
    /// Gap between children in pixels.
    pub gap: f32,
    /// Draw an outline.
    pub outlined: bool,
}

impl Default for WingsSurfaceProps {
    fn default() -> Self {
        Self {
            color: ColorRole::Surface,
            shape: ShapeRole::None,
            elevation: 0,
            padding: 0.0,
            gap: 0.0,
            outlined: false,
        }
    }
}

impl WingsSurfaceProps {
    fn into_scene(self) -> impl Scene {
        let node = Node {
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(px(self.padding)),
            row_gap: px(self.gap),
            border: UiRect::all(px(if self.outlined { 1.0 } else { 0.0 })),
            ..default()
        };
        let border = if self.outlined {
            WingsPaint::Role(ColorRole::OutlineVariant)
        } else {
            WingsPaint::Transparent
        };
        bsn! {
            node
            WingsBackground({WingsPaint::Role(self.color)})
            WingsContentColor({WingsPaint::Role(self.color.on().unwrap_or(ColorRole::OnSurface))})
            WingsBorderPaint({border})
            WingsShape({self.shape})
            WingsElevationLevel({self.elevation})
        }
    }
}

impl WingsSurface {
    fn scene(props: WingsSurfaceProps) -> impl Scene {
        props.into_scene()
    }
}

/// Card style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WingsCardStyle {
    /// Raised card on the low surface container.
    #[default]
    Elevated,
    /// Flat card on the highest surface container.
    Filled,
    /// Outlined card on the surface color.
    Outlined,
}

/// Rounded container grouping related content.
#[derive(SceneComponent, Default, Clone)]
#[scene(WingsCardProps)]
pub struct WingsCard;

/// Props of [`WingsCard`].
#[derive(Clone)]
pub struct WingsCardProps {
    /// Visual style.
    pub style: WingsCardStyle,
    /// Inner padding in pixels.
    pub padding: f32,
    /// Gap between children in pixels.
    pub gap: f32,
}

impl Default for WingsCardProps {
    fn default() -> Self {
        Self {
            style: WingsCardStyle::Elevated,
            padding: 16.0,
            gap: 8.0,
        }
    }
}

impl WingsCard {
    fn scene(props: WingsCardProps) -> impl Scene {
        let (color, elevation, outlined) = match props.style {
            WingsCardStyle::Elevated => (ColorRole::SurfaceContainerLow, 1, false),
            WingsCardStyle::Filled => (ColorRole::SurfaceContainerHighest, 0, false),
            WingsCardStyle::Outlined => (ColorRole::Surface, 0, true),
        };
        WingsSurfaceProps {
            color,
            shape: ShapeRole::Medium,
            elevation,
            padding: props.padding,
            gap: props.gap,
            outlined,
        }
        .into_scene()
    }
}
