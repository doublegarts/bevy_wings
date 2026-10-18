//! [`WingsTextField`]: a themed single line text input.
//!
//! The field wraps the headless `TextInput` widget (cursor, selection, clipboard, IME) and
//! adds a label, theme colors and a focus outline. Every edit that changes the text triggers
//! [`WingsTextChanged`] on the field entity, which is easy to forward into a view model:
//!
//! ```ignore
//! bsn! {
//!     @WingsTextField { @label: "Name", @value: "Ada" }
//!     on(|ev: On<WingsTextChanged>, mut vm: Query<&mut ProfileViewModel>| {
//!         if let Ok(mut vm) = vm.single_mut() { vm.name = ev.value.clone(); }
//!     })
//! }
//! ```

use bevy::ecs::template::template;
use bevy::input_focus::InputFocus;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::picking::cursor::EntityCursor;
use bevy::prelude::*;
use bevy::text::{EditableText, TextCursorStyle, TextEditChange};
use bevy::ui_widgets::TextInput;
use bevy::window::SystemCursorIcon;

use bevy_wings_ui_theme::prelude::{
    ColorRole, ShapeRole, TypeRole, WingsBackground, WingsBorderPaint, WingsDesign,
    WingsForeground, WingsInheritContentColor, WingsPaint, WingsShape, WingsTheme, WingsTypeStyle,
};

use crate::skin::{WingsSkin, WingsSkinPart, clear_skin_parts, skin_targets};
use crate::text::WingsText;

/// Triggered on a [`WingsTextField`] whenever its text changes.
#[derive(EntityEvent, Debug, Clone)]
pub struct WingsTextChanged {
    /// The text field.
    pub entity: Entity,
    /// The new text.
    pub value: String,
}

/// Last text value seen on a field, used to filter cursor only edits.
#[derive(Component, Debug, Default, Clone, PartialEq)]
pub struct WingsTextValue(pub String);

/// Marks the editable child of a [`WingsTextField`].
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct WingsTextFieldInput;

/// Marks the label of a [`WingsTextField`].
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct WingsTextFieldLabel;

/// Marks the focus underline a Fluent text field shows.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct WingsTextFieldUnderline;

/// How a focused field is highlighted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WingsFocusStyle {
    /// Thicker border in the primary color (Material).
    #[default]
    Border,
    /// A soft accent ring around the field (macOS).
    Ring,
    /// An accent line along the bottom edge (Fluent).
    Underline,
    /// Only the caret shows focus (iOS).
    Caret,
}

/// Text field layout chosen by the skin.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct WingsTextFieldStyle {
    /// Inner padding (horizontal, vertical).
    pub padding: Vec2,
    /// Focus highlight.
    pub focus: WingsFocusStyle,
    /// The label is a placeholder inside the field, hidden while there is text.
    pub placeholder: bool,
}

impl Default for WingsTextFieldStyle {
    fn default() -> Self {
        Self::for_design(WingsDesign::Material)
    }
}

impl WingsTextFieldStyle {
    /// Style of a design.
    pub fn for_design(design: WingsDesign) -> Self {
        match design {
            WingsDesign::Material => Self {
                padding: Vec2::new(16.0, 8.0),
                focus: WingsFocusStyle::Border,
                placeholder: false,
            },
            WingsDesign::Ios => Self {
                padding: Vec2::new(8.0, 6.0),
                focus: WingsFocusStyle::Caret,
                placeholder: true,
            },
            WingsDesign::MacOs => Self {
                padding: Vec2::new(6.0, 3.0),
                focus: WingsFocusStyle::Ring,
                placeholder: true,
            },
            WingsDesign::Fluent => Self {
                padding: Vec2::new(10.0, 5.0),
                focus: WingsFocusStyle::Underline,
                placeholder: false,
            },
        }
    }
}

/// A themed text field.
#[derive(SceneComponent, Default, Clone)]
#[scene(WingsTextFieldProps)]
pub struct WingsTextField;

/// Props of [`WingsTextField`].
#[derive(Clone)]
pub struct WingsTextFieldProps {
    /// Initial text.
    pub value: String,
    /// Label shown above the text.
    pub label: String,
    /// Width of the field.
    pub width: Val,
    /// Maximum number of characters.
    pub max_characters: Option<usize>,
}

impl Default for WingsTextFieldProps {
    fn default() -> Self {
        Self {
            value: String::new(),
            label: String::new(),
            width: Val::Px(280.0),
            max_characters: None,
        }
    }
}

impl WingsTextField {
    fn scene(props: WingsTextFieldProps) -> impl Scene {
        let label: Box<dyn Scene> = if props.label.is_empty() {
            Box::new(bsn! {})
        } else {
            let text = props.label.clone();
            Box::new(bsn! {
                Children [
                    @WingsText {
                        @text: text,
                        @role: TypeRole::BodySmall,
                        @color: ColorRole::OnSurfaceVariant,
                    }
                    WingsTextFieldLabel
                    Pickable::IGNORE
                ]
            })
        };
        let initial = props.value.clone();
        let max = props.max_characters;
        let width = props.width;
        let value = WingsTextValue(props.value);
        bsn! {
            Node {
                width: width,
                flex_direction: FlexDirection::Column,
                row_gap: px(4),
                padding: UiRect::axes(px(16), px(8)),
                border: px(1),
            }
            WingsShape(ShapeRole::ExtraSmall)
            WingsBorderPaint({WingsPaint::Role(ColorRole::Outline)})
            WingsBackground
            WingsTextFieldStyle
            Outline
            value
            @{label}
            Children [
                Node { min_height: px(24) }
                WingsTextFieldInput
                TextInput
                template(move |_| {
                    let mut editable = EditableText::new(&initial);
                    editable.max_characters = max;
                    Ok(editable)
                })
                TextLayout { linebreak: LineBreak::NoWrap }
                WingsTypeStyle(TypeRole::BodyLarge)
                WingsInheritContentColor
                TextCursorStyle
                TabIndex(0)
                EntityCursor::System(SystemCursorIcon::Text)
                on(forward_text_edits)
            ]
        }
    }
}

/// Observer: turns raw edits into [`WingsTextChanged`] when the text really changed.
pub fn forward_text_edits(
    edit: On<TextEditChange>,
    inputs: Query<(&EditableText, &ChildOf), With<WingsTextFieldInput>>,
    mut fields: Query<&mut WingsTextValue>,
    mut commands: Commands,
) {
    let Ok((editable, parent)) = inputs.get(edit.event_target()) else {
        return;
    };
    let field = parent.parent();
    let Ok(mut last) = fields.get_mut(field) else {
        return;
    };
    let text = editable.value().to_string();
    if last.0 != text {
        last.0.clone_from(&text);
        commands.trigger(WingsTextChanged {
            entity: field,
            value: text,
        });
    }
}

/// Highlights the focused field the way its design does and keeps cursor colors on theme.
pub fn text_field_focus_system(
    focus: Res<InputFocus>,
    theme: Res<WingsTheme>,
    inputs: Query<(Entity, &ChildOf), With<WingsTextFieldInput>>,
    mut fields: Query<
        (
            &WingsTextFieldStyle,
            Ref<WingsTextFieldStyle>,
            &Children,
            &mut WingsBorderPaint,
            &mut Node,
            &mut Outline,
        ),
        With<WingsTextField>,
    >,
    mut underlines: Query<&mut Node, (With<WingsTextFieldUnderline>, Without<WingsTextField>)>,
    mut cursors: Query<&mut TextCursorStyle, With<WingsTextFieldInput>>,
    added: Query<(), Added<WingsTextFieldInput>>,
) {
    if theme.is_changed() || !added.is_empty() {
        let primary = theme.color(ColorRole::Primary);
        for mut style in &mut cursors {
            style.color = primary;
            style.selection_color = primary.with_alpha(0.35);
            style.unfocused_selection_color = primary.with_alpha(0.15);
        }
    }
    let restyled = fields.iter().any(|f| f.1.is_changed());
    if !focus.is_changed() && added.is_empty() && !restyled && !theme.is_changed() {
        return;
    }
    let focused = focus.get();
    let ring = theme.color(ColorRole::Primary).with_alpha(0.45);
    for (input, parent) in &inputs {
        let Ok((style, _, children, mut border, mut node, mut outline)) =
            fields.get_mut(parent.parent())
        else {
            continue;
        };
        let is_focused = focused == Some(input);
        let rest = if style.focus == WingsFocusStyle::Border {
            ColorRole::Outline
        } else {
            ColorRole::OutlineVariant
        };
        let (role, width) = match (style.focus, is_focused) {
            (WingsFocusStyle::Border, true) => (ColorRole::Primary, 2.0),
            _ => (rest, 1.0),
        };
        border.set_if_neq(WingsBorderPaint(WingsPaint::Role(role)));
        // Keep the content still: thicker border, thinner padding.
        let border_rect = UiRect::all(px(width));
        if node.border != border_rect || node.padding.left != px(style.padding.x + 1.0 - width) {
            node.border = border_rect;
            node.padding = UiRect::axes(
                px(style.padding.x + 1.0 - width),
                px(style.padding.y + 1.0 - width),
            );
        }
        let ring_color = if style.focus == WingsFocusStyle::Ring && is_focused {
            ring
        } else {
            Color::NONE
        };
        if outline.color != ring_color {
            *outline = Outline {
                width: px(3),
                offset: px(0),
                color: ring_color,
            };
        }
        for child in children.iter() {
            if let Ok(mut line) = underlines.get_mut(child) {
                let display = if is_focused && style.focus == WingsFocusStyle::Underline {
                    Display::Flex
                } else {
                    Display::None
                };
                if line.display != display {
                    line.display = display;
                }
            }
        }
    }
}

/// Skins text fields: padding, fill, corners, label placement and focus style.
pub fn skin_text_fields(
    mut commands: Commands,
    skin: Res<WingsSkin>,
    added: Query<Entity, Added<WingsTextField>>,
    all: Query<Entity, With<WingsTextField>>,
    children: Query<&Children>,
    parts: Query<(), With<WingsSkinPart>>,
    mut fields: Query<
        (
            &mut WingsTextFieldStyle,
            &mut Node,
            &mut WingsShape,
            &mut WingsBackground,
            &WingsTextValue,
        ),
        With<WingsTextField>,
    >,
    mut labels: Query<
        (&mut Node, &mut WingsTypeStyle, &mut WingsForeground),
        (With<WingsTextFieldLabel>, Without<WingsTextField>),
    >,
) {
    let style = WingsTextFieldStyle::for_design(skin.design);
    let (min_height, shape, fill) = match skin.design {
        WingsDesign::Material => (Val::Auto, ShapeRole::ExtraSmall, WingsPaint::Transparent),
        WingsDesign::Ios => (
            px(34),
            ShapeRole::ExtraSmall,
            WingsPaint::Role(ColorRole::Surface),
        ),
        WingsDesign::MacOs => (
            px(24),
            ShapeRole::Small,
            WingsPaint::Role(ColorRole::Surface),
        ),
        WingsDesign::Fluent => (
            px(32),
            ShapeRole::Small,
            WingsPaint::Role(ColorRole::SurfaceContainerLowest),
        ),
    };
    for entity in skin_targets(&skin, added, all) {
        let Ok((mut current, mut node, mut s, mut bg, value)) = fields.get_mut(entity) else {
            continue;
        };
        current.set_if_neq(style);
        node.min_height = min_height;
        node.justify_content = JustifyContent::Center;
        node.margin.top = px(if skin.fluent() { 24.0 } else { 0.0 });
        s.set_if_neq(WingsShape(shape));
        bg.set_if_neq(WingsBackground(fill));
        let empty = value.0.is_empty();
        clear_skin_parts(&mut commands, entity, &children, &parts);
        for child in children.iter_descendants(entity) {
            let Ok((mut label, mut role, mut color)) = labels.get_mut(child) else {
                continue;
            };
            match skin.design {
                WingsDesign::Material => {
                    label.position_type = PositionType::Relative;
                    label.left = Val::Auto;
                    label.top = Val::Auto;
                    label.display = Display::Flex;
                    role.set_if_neq(WingsTypeStyle(TypeRole::BodySmall));
                    color.set_if_neq(WingsForeground(WingsPaint::Role(
                        ColorRole::OnSurfaceVariant,
                    )));
                }
                WingsDesign::Fluent => {
                    label.position_type = PositionType::Absolute;
                    label.left = px(0);
                    label.top = px(-24);
                    label.display = Display::Flex;
                    role.set_if_neq(WingsTypeStyle(TypeRole::BodyMedium));
                    color.set_if_neq(WingsForeground(WingsPaint::Role(ColorRole::OnSurface)));
                }
                _ => {
                    label.position_type = PositionType::Absolute;
                    label.left = px(style.padding.x);
                    label.top = px(style.padding.y);
                    label.display = if empty { Display::Flex } else { Display::None };
                    role.set_if_neq(WingsTypeStyle(TypeRole::BodyLarge));
                    color.set_if_neq(WingsForeground(WingsPaint::Role(ColorRole::TertiaryLabel)));
                }
            }
        }
        if skin.fluent() {
            commands.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(-1),
                    right: px(-1),
                    bottom: px(-1),
                    height: px(2),
                    display: Display::None,
                    ..default()
                },
                WingsBackground(WingsPaint::Role(ColorRole::Primary)),
                WingsTextFieldUnderline,
                Pickable::IGNORE,
                WingsSkinPart,
                ChildOf(entity),
            ));
        }
    }
}

/// Shows placeholder labels only while their field is empty.
pub fn text_field_placeholder_system(
    fields: Query<(&WingsTextValue, &WingsTextFieldStyle, &Children), Changed<WingsTextValue>>,
    mut labels: Query<&mut Node, With<WingsTextFieldLabel>>,
) {
    for (value, style, children) in &fields {
        if !style.placeholder {
            continue;
        }
        for child in children.iter() {
            if let Ok(mut node) = labels.get_mut(child) {
                let display = if value.0.is_empty() {
                    Display::Flex
                } else {
                    Display::None
                };
                if node.display != display {
                    node.display = display;
                }
            }
        }
    }
}
