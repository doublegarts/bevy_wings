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

use dga_bevy_wings_ui_theme::prelude::{
    ColorRole, ShapeRole, TypeRole, WingsBorderPaint, WingsInheritContentColor, WingsPaint,
    WingsShape, WingsTheme, WingsTypeStyle,
};

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

/// Highlights the outline of the focused field and keeps cursor colors on theme.
pub fn text_field_focus_system(
    focus: Res<InputFocus>,
    theme: Res<WingsTheme>,
    inputs: Query<(Entity, &ChildOf), With<WingsTextFieldInput>>,
    mut fields: Query<(&mut WingsBorderPaint, &mut Node), With<WingsTextField>>,
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
    if !focus.is_changed() && added.is_empty() {
        return;
    }
    let focused = focus.get();
    for (input, parent) in &inputs {
        let Ok((mut border, mut node)) = fields.get_mut(parent.parent()) else {
            continue;
        };
        let is_focused = focused == Some(input);
        let (role, width) = if is_focused {
            (ColorRole::Primary, 2.0)
        } else {
            (ColorRole::Outline, 1.0)
        };
        border.set_if_neq(WingsBorderPaint(WingsPaint::Role(role)));
        // Keep the content still: thicker border, thinner padding.
        let border_rect = UiRect::all(px(width));
        if node.border != border_rect {
            node.border = border_rect;
            node.padding = UiRect::axes(px(17.0 - width), px(9.0 - width));
        }
    }
}
