//! Scene strategies: which back stack entries are visible, and where.
//!
//! On a phone only the top entry is shown. On a wide window a list entry and its detail
//! entry can be shown side by side. The strategy is a pure function of the back stack metadata
//! and the current width class, which keeps it trivially testable.

use dga_bevy_wings_ui_responsive::WidthClass;

/// The role an entry plays in multi pane layouts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PaneRole {
    /// A regular full screen destination.
    #[default]
    Single,
    /// A list or master pane.
    List,
    /// A detail pane shown next to a [`PaneRole::List`] entry on wide windows.
    Detail,
    /// Supporting content shown next to the main content on wide windows.
    Supporting,
}

/// Where a visible entry is placed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PaneSlot {
    /// Fills the display.
    Full,
    /// Left pane with the given width in percent.
    Leading(f32),
    /// Right pane starting at `start` percent and taking the rest.
    Trailing(f32),
}

/// Strategy deciding the visible entries.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NavSceneStrategy {
    /// Only the top entry is visible.
    SinglePane,
    /// Shows a list and a detail entry side by side from `min_class` upwards.
    ListDetail {
        /// Minimum width class for the two pane layout.
        min_class: WidthClass,
        /// Width of the list pane in percent.
        list_width: f32,
    },
}

impl Default for NavSceneStrategy {
    fn default() -> Self {
        Self::ListDetail {
            min_class: WidthClass::Expanded,
            list_width: 38.0,
        }
    }
}

impl NavSceneStrategy {
    /// Computes visible entries as `(stack index, slot)` pairs, bottom first.
    pub fn plan(&self, roles: &[PaneRole], class: WidthClass) -> Vec<(usize, PaneSlot)> {
        let Some(top) = roles.len().checked_sub(1) else {
            return Vec::new();
        };
        match *self {
            NavSceneStrategy::SinglePane => vec![(top, PaneSlot::Full)],
            NavSceneStrategy::ListDetail {
                min_class,
                list_width,
            } => {
                let wide = class >= min_class;
                let top_role = roles[top];
                if wide && top > 0 {
                    let below = roles[top - 1];
                    let pair = matches!(
                        (below, top_role),
                        (PaneRole::List, PaneRole::Detail)
                            | (PaneRole::Single, PaneRole::Supporting)
                            | (PaneRole::List, PaneRole::Supporting)
                    );
                    if pair {
                        return vec![
                            (top - 1, PaneSlot::Leading(list_width)),
                            (top, PaneSlot::Trailing(list_width)),
                        ];
                    }
                }
                vec![(top, PaneSlot::Full)]
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Single pane always shows only the top entry.
    #[test]
    fn single_pane_shows_top() {
        let plan = NavSceneStrategy::SinglePane
            .plan(&[PaneRole::List, PaneRole::Detail], WidthClass::ExtraLarge);
        assert_eq!(plan, vec![(1, PaneSlot::Full)]);
    }

    /// List and detail are side by side on wide windows only.
    #[test]
    fn list_detail_depends_on_width() {
        let s = NavSceneStrategy::default();
        let roles = [PaneRole::Single, PaneRole::List, PaneRole::Detail];
        assert_eq!(
            s.plan(&roles, WidthClass::Compact),
            vec![(2, PaneSlot::Full)]
        );
        assert_eq!(
            s.plan(&roles, WidthClass::Expanded),
            vec![(1, PaneSlot::Leading(38.0)), (2, PaneSlot::Trailing(38.0))]
        );
    }

    /// A detail without a list below falls back to full screen.
    #[test]
    fn detail_without_list_is_full() {
        let s = NavSceneStrategy::default();
        assert_eq!(
            s.plan(&[PaneRole::Single, PaneRole::Detail], WidthClass::Large),
            vec![(1, PaneSlot::Full)]
        );
    }

    /// Empty stacks have nothing visible.
    #[test]
    fn empty_plan() {
        assert!(
            NavSceneStrategy::default()
                .plan(&[], WidthClass::Large)
                .is_empty()
        );
    }
}
