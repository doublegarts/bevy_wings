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

use bevy_wings_ui_theme::prelude::{
    ColorRole, ShapeRole, WingsBackground, WingsBorderPaint, WingsContentColor, WingsDesign,
    WingsElevationLevel, WingsPaint, WingsShape,
};

use crate::skin::{WingsSkin, skin_targets};

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

/// Vertically scrolling column (mouse wheel and touchpad). Its children keep their size
/// instead of shrinking to fit, so long content scrolls.
#[derive(SceneComponent, Default, Clone)]
#[scene(WingsStackProps)]
pub struct WingsScrollColumn;

/// Keeps the children of scroll columns from shrinking (flexbox would squeeze fixed height
/// children such as progress bars to nothing once the content overflows).
pub fn scroll_children_keep_size(
    columns: Query<&Children, (With<WingsScrollColumn>, Changed<Children>)>,
    mut nodes: Query<&mut Node, Without<WingsScrollColumn>>,
) {
    for children in &columns {
        for child in children.iter() {
            if let Ok(mut node) = nodes.get_mut(child)
                && node.flex_shrink != 0.0
            {
                node.flex_shrink = 0.0;
            }
        }
    }
}

impl WingsScrollColumn {
    fn scene(props: WingsStackProps) -> impl Scene {
        let mut node = props.node(FlexDirection::Column);
        node.overflow = Overflow::scroll_y();
        bsn! { node ScrollArea }
    }
}

/// Fixed column grid. Combine with
/// [`WingsResponsiveColumns`](bevy_wings_ui_responsive::WingsResponsiveColumns) for column
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

/// Card style. Also the component the card skin uses to find cards.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
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
        let style = props.style;
        let surface = card_surface(WingsDesign::Material, style, props.padding, props.gap);
        bsn! { @{surface.into_scene()} style }
    }
}

/// Surface of a card style in a design.
fn card_surface(
    design: WingsDesign,
    style: WingsCardStyle,
    padding: f32,
    gap: f32,
) -> WingsSurfaceProps {
    use ColorRole::*;
    let (color, shape, elevation, outlined) = match (design, style) {
        (WingsDesign::Material, WingsCardStyle::Elevated) => {
            (SurfaceContainerLow, ShapeRole::Medium, 1, false)
        }
        (WingsDesign::Material, WingsCardStyle::Filled) => {
            (SurfaceContainerHighest, ShapeRole::Medium, 0, false)
        }
        (WingsDesign::Material, WingsCardStyle::Outlined) => (Surface, ShapeRole::Medium, 0, true),
        (WingsDesign::Fluent, WingsCardStyle::Elevated) => {
            (SecondaryGroupedBackground, ShapeRole::Large, 1, true)
        }
        (WingsDesign::Fluent, WingsCardStyle::Filled) => {
            (SurfaceContainer, ShapeRole::Large, 0, false)
        }
        (WingsDesign::Fluent, WingsCardStyle::Outlined) => {
            (SecondaryGroupedBackground, ShapeRole::Large, 0, true)
        }
        (_, WingsCardStyle::Elevated) => (SurfaceContainerLowest, ShapeRole::Large, 2, false),
        (_, WingsCardStyle::Filled) => (SurfaceVariant, ShapeRole::Large, 0, false),
        (_, WingsCardStyle::Outlined) => (Surface, ShapeRole::Large, 0, true),
    };
    WingsSurfaceProps {
        color,
        shape,
        elevation,
        padding,
        gap,
        outlined,
    }
}

/// Skins cards: fill, corners, outline and shadow of each card style per design.
pub fn skin_cards(
    skin: Res<WingsSkin>,
    added: Query<Entity, Added<WingsCardStyle>>,
    all: Query<Entity, With<WingsCardStyle>>,
    mut cards: Query<(
        &WingsCardStyle,
        &mut Node,
        &mut WingsBackground,
        &mut WingsContentColor,
        &mut WingsBorderPaint,
        &mut WingsShape,
        &mut WingsElevationLevel,
    )>,
) {
    for entity in skin_targets(&skin, added, all) {
        let Ok((style, mut node, mut bg, mut content, mut border, mut shape, mut elevation)) =
            cards.get_mut(entity)
        else {
            continue;
        };
        let s = card_surface(skin.design, *style, 0.0, 0.0);
        bg.set_if_neq(WingsBackground(WingsPaint::Role(s.color)));
        content.set_if_neq(WingsContentColor(WingsPaint::Role(
            s.color.on().unwrap_or(ColorRole::OnSurface),
        )));
        border.set_if_neq(WingsBorderPaint(if s.outlined {
            WingsPaint::Role(ColorRole::OutlineVariant)
        } else {
            WingsPaint::Transparent
        }));
        node.border = UiRect::all(px(if s.outlined { 1.0 } else { 0.0 }));
        shape.set_if_neq(WingsShape(s.shape));
        elevation.set_if_neq(WingsElevationLevel(s.elevation));
    }
}
