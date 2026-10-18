//! Grouped lists: [`WingsList`] and [`WingsListSection`], like SwiftUI's `List` and
//! `Section` (and settings pages on every platform).
//!
//! ```ignore
//! bsn! {
//!     @WingsList
//!     Children [
//!         @WingsListSection { @header: "Network", @footer: "Used for updates." }
//!         Children [
//!             @WingsListItem { @headline: "Wi-Fi", @detail: "Home", @accessory: WingsListAccessory::Chevron }
//!             --
//!             @WingsListItem { @headline: "Bluetooth", @detail: "On" }
//!         ]
//!     ]
//! }
//! ```
//!
//! | Design | Look |
//! | --- | --- |
//! | Material | plain list, section titles in the primary color, dividers between rows |
//! | iOS | inset grouped: rounded white groups on a gray background, inset separators |
//! | macOS | grouped form: rounded light gray groups, inset separators |
//! | Fluent | settings cards: one rounded card per row, 4 px apart |
//!
//! Rows are the section's children (usually [`WingsListItem`](crate::WingsListItem)s, but any
//! node works). The section gives them their background, corners and separators.

use bevy::prelude::*;

use bevy_wings_ui_theme::prelude::{
    ColorRole, TypeRole, WingsBackground, WingsBorderPaint, WingsDesign, WingsPaint, WingsShape,
    WingsTheme, WingsTypeOverride,
};

use crate::skin::{WingsSkin, WingsSkinPart, pick};
use crate::text::WingsText;

/// Visual style of a [`WingsList`].
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WingsListStyle {
    /// Plain in Material, grouped in the other designs.
    #[default]
    Automatic,
    /// Rows directly on the background with dividers.
    Plain,
    /// Rows grouped in rounded sections.
    Grouped,
}

impl WingsListStyle {
    /// The concrete style in a design.
    pub fn resolve(self, design: WingsDesign) -> WingsListStyle {
        match self {
            Self::Automatic if design == WingsDesign::Material => Self::Plain,
            Self::Automatic => Self::Grouped,
            other => other,
        }
    }
}

/// A vertical list of [`WingsListSection`]s.
#[derive(SceneComponent, Default, Clone)]
#[scene(WingsListProps)]
pub struct WingsList;

/// Props of [`WingsList`].
#[derive(Default, Clone)]
pub struct WingsListProps {
    /// Visual style.
    pub style: WingsListStyle,
}

impl WingsList {
    fn scene(props: WingsListProps) -> impl Scene {
        let style = props.style;
        bsn! {
            Node { flex_direction: FlexDirection::Column, flex_shrink: 0.0 }
            style
            WingsBackground
        }
    }
}

/// A group of rows with an optional header and footer.
#[derive(SceneComponent, Default, Clone)]
#[scene(WingsListSectionProps)]
pub struct WingsListSection;

/// Props of [`WingsListSection`].
#[derive(Default, Clone)]
pub struct WingsListSectionProps {
    /// Text above the group. Empty for none.
    pub header: String,
    /// Text below the group. Empty for none.
    pub footer: String,
}

/// Header and footer texts of a section.
#[derive(Component, Debug, Default, Clone, PartialEq)]
pub struct WingsListSectionText {
    /// Header text.
    pub header: String,
    /// Footer text.
    pub footer: String,
}

/// The header of a [`WingsListSection`].
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct WingsListSectionHeader;

/// The footer of a [`WingsListSection`].
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct WingsListSectionFooter;

/// A separator line drawn inside a section row.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct WingsListSeparator;

impl WingsListSection {
    fn scene(props: WingsListSectionProps) -> impl Scene {
        let text = WingsListSectionText {
            header: props.header,
            footer: props.footer,
        };
        bsn! {
            Node { flex_direction: FlexDirection::Column, flex_shrink: 0.0 }
            text
        }
    }
}

/// Skins lists: background and spacing per design.
pub fn skin_lists(
    skin: Res<WingsSkin>,
    mut lists: Query<(Ref<WingsListStyle>, &mut Node, &mut WingsBackground), With<WingsList>>,
) {
    for (style, mut node, mut bg) in &mut lists {
        if !style.is_changed() && !skin.is_changed() {
            continue;
        }
        let grouped = style.resolve(skin.design) == WingsListStyle::Grouped;
        let (pad, gap, fill) = if grouped {
            (
                pick(skin.design, 16.0, 16.0, 20.0, 16.0),
                pick(skin.design, 16.0, 24.0, 20.0, 24.0),
                pick(
                    skin.design,
                    ColorRole::Background,
                    ColorRole::GroupedBackground,
                    ColorRole::Background,
                    ColorRole::Background,
                ),
            )
        } else {
            (0.0, 8.0, ColorRole::Background)
        };
        node.padding = UiRect::axes(px(pad), px(if grouped { 12.0 } else { 0.0 }));
        node.row_gap = px(gap);
        bg.set_if_neq(WingsBackground(if grouped {
            WingsPaint::Role(fill)
        } else {
            WingsPaint::Transparent
        }));
    }
}

type RowParts<'a> = (
    Option<&'a mut Node>,
    Option<&'a mut WingsBackground>,
    Option<&'a mut WingsBorderPaint>,
);

/// Lays out sections: header first and footer last, row backgrounds, corners and separators.
/// Runs when a section's rows change and after design or theme shape changes.
pub fn layout_list_sections(
    mut commands: Commands,
    skin: Res<WingsSkin>,
    theme: Res<WingsTheme>,
    sections: Query<(
        Entity,
        Ref<WingsListSectionText>,
        Option<Ref<Children>>,
        &ChildOf,
    )>,
    lists: Query<&WingsListStyle>,
    headers: Query<(), With<WingsListSectionHeader>>,
    footers: Query<(), With<WingsListSectionFooter>>,
    skin_parts: Query<(), With<WingsSkinPart>>,
    separators: Query<(), With<WingsListSeparator>>,
    children_of: Query<&Children>,
    icons: Query<&Node, With<crate::scaffold::WingsListItemIcon>>,
    mut rows: Query<
        RowParts,
        (
            Without<WingsListSectionText>,
            Without<crate::scaffold::WingsListItemIcon>,
        ),
    >,
) {
    let force = skin.is_changed() || theme.is_changed();
    for (section, text, children, parent) in &sections {
        let changed = text.is_changed() || children.as_ref().is_some_and(|c| c.is_changed());
        if !changed && !force {
            continue;
        }
        let style = lists
            .get(parent.parent())
            .copied()
            .unwrap_or_default()
            .resolve(skin.design);
        let all: Vec<Entity> = children.map(|c| c.iter().collect()).unwrap_or_default();
        let header = all.iter().copied().find(|e| headers.contains(*e));
        let footer = all.iter().copied().find(|e| footers.contains(*e));
        let order_ok = header.is_none_or(|h| all.first() == Some(&h))
            && footer.is_none_or(|f| all.last() == Some(&f))
            && header.is_some() == !text.header.is_empty()
            && footer.is_some() == !text.footer.is_empty();
        if !order_ok || force || text.is_changed() {
            // Rebuild the header and footer at the right places.
            for e in header.into_iter().chain(footer) {
                commands.entity(e).despawn();
            }
            let header_style = header_style(skin.design, style);
            if !text.header.is_empty() {
                let label = text.header.clone();
                let (role, color, adjust, pad) = header_style;
                let header = commands
                    .spawn_scene(bsn! {
                        @WingsText { @text: label, @role: role, @color: color }
                        WingsListSectionHeader
                    })
                    .id();
                commands.entity(header).insert((
                    adjust,
                    Node {
                        margin: pad,
                        ..default()
                    },
                ));
                commands.entity(section).insert_children(0, &[header]);
            }
            if !text.footer.is_empty() {
                let label = text.footer.clone();
                let footer = commands
                    .spawn_scene(bsn! {
                        @WingsText {
                            @text: label,
                            @role: TypeRole::BodySmall,
                            @color: ColorRole::OnSurfaceVariant,
                        }
                        WingsListSectionFooter
                    })
                    .id();
                let inset = pick(skin.design, 16.0, 16.0, 10.0, 0.0);
                commands.entity(footer).insert(Node {
                    margin: UiRect {
                        left: px(inset),
                        right: px(inset),
                        top: px(6),
                        ..default()
                    },
                    ..default()
                });
                commands.entity(section).add_child(footer);
            }
        }
        let body: Vec<Entity> = all
            .iter()
            .copied()
            .filter(|e| !headers.contains(*e) && !footers.contains(*e) && !skin_parts.contains(*e))
            .collect();
        let radius = theme.shapes.px(match skin.design {
            WingsDesign::Fluent => bevy_wings_ui_theme::prelude::ShapeRole::Small,
            WingsDesign::MacOs => bevy_wings_ui_theme::prelude::ShapeRole::Medium,
            _ => bevy_wings_ui_theme::prelude::ShapeRole::Large,
        });
        let count = body.len();
        for (i, row) in body.iter().copied().enumerate() {
            // Remove separators from earlier layouts.
            if let Ok(kids) = children_of.get(row) {
                for k in kids.iter() {
                    if separators.contains(k) {
                        commands.entity(k).despawn();
                    }
                }
            }
            let Ok((node, bg, border)) = rows.get_mut(row) else {
                continue;
            };
            let first = i == 0;
            let last = i + 1 == count;
            let (fill, corners, outlined, gap, separator) = match (style, skin.design) {
                (WingsListStyle::Plain, _) | (_, WingsDesign::Material) => (
                    WingsPaint::Transparent,
                    BorderRadius::ZERO,
                    false,
                    0.0,
                    !last,
                ),
                (_, WingsDesign::Fluent) => (
                    WingsPaint::Role(ColorRole::SecondaryGroupedBackground),
                    BorderRadius::all(px(radius)),
                    true,
                    if last { 0.0 } else { 4.0 },
                    false,
                ),
                _ => {
                    let corners = BorderRadius {
                        top_left: px(if first { radius } else { 0.0 }).into(),
                        top_right: px(if first { radius } else { 0.0 }).into(),
                        bottom_left: px(if last { radius } else { 0.0 }).into(),
                        bottom_right: px(if last { radius } else { 0.0 }).into(),
                    };
                    (
                        WingsPaint::Role(ColorRole::SecondaryGroupedBackground),
                        corners,
                        false,
                        0.0,
                        !last,
                    )
                }
            };
            if let Some(mut node) = node {
                node.border_radius = corners;
                node.margin.bottom = px(gap);
                node.border = UiRect::all(px(if outlined { 1.0 } else { 0.0 }));
                if style == WingsListStyle::Grouped && skin.design.is_apple() {
                    node.overflow = Overflow::clip();
                }
            }
            if let Some(mut bg) = bg {
                bg.set_if_neq(WingsBackground(fill));
            }
            if let Some(mut border) = border {
                border.set_if_neq(WingsBorderPaint(if outlined {
                    WingsPaint::Role(ColorRole::OutlineVariant)
                } else {
                    WingsPaint::Transparent
                }));
            }
            commands.entity(row).remove::<WingsShape>();
            if separator {
                // Inset separators start at the text, after any leading icon.
                let icon = children_of
                    .iter_descendants(row)
                    .find_map(|d| icons.get(d).ok())
                    .and_then(|n| match n.width {
                        Val::Px(w) => Some(w),
                        _ => None,
                    });
                let pad = pick(skin.design, 16.0, 16.0, 10.0, 12.0);
                let gap = pick(skin.design, 16.0, 12.0, 8.0, 12.0);
                let left = if skin.design == WingsDesign::Material {
                    0.0
                } else {
                    pad + icon.map_or(0.0, |w| w + gap)
                };
                commands.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(left),
                        right: px(0),
                        bottom: px(0),
                        height: px(1),
                        ..default()
                    },
                    WingsBackground(WingsPaint::Role(ColorRole::OutlineVariant)),
                    WingsListSeparator,
                    WingsSkinPart,
                    Pickable::IGNORE,
                    ChildOf(row),
                ));
            }
        }
    }
}

/// Header text style: `(role, color, weight override, margin)`.
fn header_style(
    design: WingsDesign,
    style: WingsListStyle,
) -> (TypeRole, ColorRole, WingsTypeOverride, UiRect) {
    match (design, style) {
        (WingsDesign::Material, _) | (_, WingsListStyle::Plain) => (
            TypeRole::TitleSmall,
            ColorRole::Primary,
            WingsTypeOverride::default(),
            UiRect::axes(px(16), px(8)),
        ),
        (WingsDesign::Ios, _) => (
            TypeRole::TitleSmall,
            ColorRole::OnSurfaceVariant,
            WingsTypeOverride::default(),
            UiRect {
                left: px(16),
                bottom: px(6),
                ..default()
            },
        ),
        (WingsDesign::MacOs, _) => (
            TypeRole::TitleLarge,
            ColorRole::OnSurface,
            WingsTypeOverride::default(),
            UiRect {
                left: px(10),
                bottom: px(6),
                ..default()
            },
        ),
        (WingsDesign::Fluent, _) => (
            TypeRole::TitleMedium,
            ColorRole::OnSurface,
            WingsTypeOverride::default(),
            UiRect {
                bottom: px(6),
                ..default()
            },
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Automatic lists are plain in Material and grouped elsewhere.
    #[test]
    fn automatic_style() {
        assert_eq!(
            WingsListStyle::Automatic.resolve(WingsDesign::Material),
            WingsListStyle::Plain
        );
        for d in [WingsDesign::Ios, WingsDesign::MacOs, WingsDesign::Fluent] {
            assert_eq!(
                WingsListStyle::Automatic.resolve(d),
                WingsListStyle::Grouped
            );
        }
        assert_eq!(
            WingsListStyle::Plain.resolve(WingsDesign::Ios),
            WingsListStyle::Plain
        );
    }
}
