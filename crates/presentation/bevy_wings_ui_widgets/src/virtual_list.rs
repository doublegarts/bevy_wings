//! Virtualized lists and grids: [`WingsVirtualList`] and [`WingsVirtualGrid`].
//!
//! Only the rows in view (plus a few above and below) exist as entities, so a list of a
//! hundred thousand items costs about as much as one screen of rows. Rows are built on
//! demand by a closure and despawned when they scroll away.
//!
//! ```ignore
//! commands.spawn((
//!     Node { height: percent(100), ..default() },
//!     WingsVirtualList::new(100_000, 52.0, |index| {
//!         Box::new(bsn! { @WingsListItem { @headline: {format!("Row {index}")} } })
//!     }),
//! ));
//! ```

use std::sync::Arc;

use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::ui_widgets::ScrollArea;

use bevy_wings_ui_responsive::WingsViewport;

type RowBuilder = Arc<dyn Fn(usize) -> Box<dyn Scene> + Send + Sync>;

/// A vertically scrolling list that only spawns visible rows.
#[derive(Component, Clone)]
#[require(Node, ScrollArea, ScrollPosition)]
pub struct WingsVirtualList {
    /// Number of rows.
    pub count: usize,
    /// Height of every row in logical pixels.
    pub row_height: f32,
    /// Columns (1 for a list, more for a grid).
    pub columns: usize,
    /// Extra rows kept above and below the visible ones.
    pub overscan: usize,
    builder: RowBuilder,
}

impl WingsVirtualList {
    /// A list of `count` rows of `row_height` built by `builder`.
    pub fn new(
        count: usize,
        row_height: f32,
        builder: impl Fn(usize) -> Box<dyn Scene> + Send + Sync + 'static,
    ) -> Self {
        Self {
            count,
            row_height: row_height.max(1.0),
            columns: 1,
            overscan: 4,
            builder: Arc::new(builder),
        }
    }

    /// A grid of `count` cells in `columns` columns of `row_height` tall rows.
    pub fn grid(
        count: usize,
        columns: usize,
        row_height: f32,
        builder: impl Fn(usize) -> Box<dyn Scene> + Send + Sync + 'static,
    ) -> Self {
        Self::new(count, row_height, builder).with_columns(columns)
    }

    /// Lays the items out in `columns` columns.
    pub fn with_columns(mut self, columns: usize) -> Self {
        self.columns = columns.max(1);
        self
    }

    /// Number of row lines (items divided by columns, rounded up).
    pub fn lines(&self) -> usize {
        self.count.div_ceil(self.columns)
    }

    /// Item indices to show for a scroll offset and viewport height.
    pub fn visible_range(&self, scroll: f32, viewport: f32) -> core::ops::Range<usize> {
        let first_line = (scroll.max(0.0) / self.row_height).floor() as usize;
        let lines_in_view = (viewport.max(0.0) / self.row_height).ceil() as usize + 1;
        let start_line = first_line.saturating_sub(self.overscan);
        let end_line = (first_line + lines_in_view + self.overscan).min(self.lines());
        (start_line * self.columns).min(self.count)..(end_line * self.columns).min(self.count)
    }
}

/// A virtualized grid: a [`WingsVirtualList`] with columns, built with
/// [`WingsVirtualList::grid`].
pub type WingsVirtualGrid = WingsVirtualList;

/// Spawned rows of a virtual list.
#[derive(Component, Debug, Default)]
pub struct WingsVirtualRows {
    rows: HashMap<usize, Entity>,
    content: Option<Entity>,
}

impl WingsVirtualRows {
    /// Number of spawned rows.
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// True when no row is spawned.
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// The row entity of an item, if it is spawned.
    pub fn row(&self, index: usize) -> Option<Entity> {
        self.rows.get(&index).copied()
    }
}

/// Marks a spawned virtual row and its item index.
#[derive(Component, Debug, Clone, Copy)]
pub struct WingsVirtualRow(pub usize);

/// Spawns rows that scrolled into view and despawns rows that left it.
pub fn virtual_list_system(
    mut commands: Commands,
    viewport: Res<WingsViewport>,
    mut lists: Query<(
        Entity,
        Ref<WingsVirtualList>,
        &mut Node,
        &ScrollPosition,
        &ComputedNode,
        Option<&mut WingsVirtualRows>,
    )>,
    mut contents: Query<&mut Node, Without<WingsVirtualList>>,
) {
    for (entity, list, mut node, scroll, computed, rows) in &mut lists {
        if list.is_added() {
            node.overflow = Overflow::scroll_y();
            node.flex_direction = FlexDirection::Column;
        }
        let Some(mut rows) = rows else {
            let content = commands
                .spawn((
                    Node {
                        width: percent(100),
                        height: px(list.lines() as f32 * list.row_height),
                        flex_shrink: 0.0,
                        ..default()
                    },
                    ChildOf(entity),
                ))
                .id();
            commands.entity(entity).insert(WingsVirtualRows {
                rows: HashMap::default(),
                content: Some(content),
            });
            continue;
        };
        let Some(content) = rows.content else {
            continue;
        };
        if list.is_changed()
            && let Ok(mut c) = contents.get_mut(content)
        {
            c.height = px(list.lines() as f32 * list.row_height);
            // Rebuild everything when the data changed.
            for (_, row) in rows.rows.drain() {
                commands.entity(row).despawn();
            }
        }
        let height = computed.size.y * computed.inverse_scale_factor;
        let view = if height > 0.0 {
            height
        } else {
            viewport.height
        };
        let range = list.visible_range(scroll.0.y, view);
        rows.rows.retain(|index, row| {
            let keep = range.contains(index);
            if !keep {
                commands.entity(*row).despawn();
            }
            keep
        });
        let width_pct = 100.0 / list.columns as f32;
        for index in range {
            if rows.rows.contains_key(&index) {
                continue;
            }
            let line = index / list.columns;
            let column = index % list.columns;
            let row = commands
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        top: px(line as f32 * list.row_height),
                        left: percent(column as f32 * width_pct),
                        width: percent(width_pct),
                        height: px(list.row_height),
                        flex_direction: FlexDirection::Column,
                        ..default()
                    },
                    WingsVirtualRow(index),
                    ChildOf(content),
                ))
                .id();
            let scene = commands.spawn_scene((list.builder)(index)).id();
            commands.entity(scene).insert(ChildOf(row));
            rows.rows.insert(index, row);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list(count: usize) -> WingsVirtualList {
        WingsVirtualList::new(count, 50.0, |_| Box::new(bevy::scene::prelude::bsn! {}))
    }

    /// The visible range covers the viewport plus overscan and stays inside the data.
    #[test]
    fn ranges() {
        let l = list(10_000);
        assert_eq!(l.visible_range(0.0, 500.0), 0..15);
        assert_eq!(l.visible_range(5_000.0, 500.0), 96..115);
        assert_eq!(l.visible_range(1e9, 500.0), 10_000..10_000);
        assert_eq!(list(3).visible_range(0.0, 500.0), 0..3);
        let grid = list(10).with_columns(3);
        assert_eq!(grid.lines(), 4);
        assert_eq!(grid.visible_range(0.0, 100.0), 0..10);
    }
}
