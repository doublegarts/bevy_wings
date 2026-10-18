//! Date and time pickers: [`WingsDatePicker`] and [`WingsTimePicker`].
//!
//! The date picker shows a month grid with previous and next buttons and triggers
//! [`WingsDateChanged`]. The time picker shows hour and minute fields with steppers (arrow
//! keys work while a field is focused) and triggers [`WingsTimeChanged`].
//!
//! | Design | Date picker | Time picker |
//! | --- | --- | --- |
//! | Material | 40 px days, filled selected circle, outlined today | large boxed digits |
//! | iOS | 44 px days, tinted selected circle (graphical `DatePicker`) | gray capsules |
//! | macOS | compact 24 px days, accent selection | small stepper fields |
//! | Fluent | calendar view, ringed selection, filled today | bordered fields |
//!
//! [`WingsDate`] does the calendar math without extra dependencies.

use bevy::input_focus::InputFocus;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::picking::cursor::EntityCursor;
use bevy::prelude::*;
use bevy::ui_widgets::{Activate, Button};
use bevy::window::SystemCursorIcon;

use bevy_wings_ui_theme::prelude::{
    ColorRole, ShapeRole, TypeRole, WingsBackground, WingsBorderPaint, WingsContentColor,
    WingsDesign, WingsForeground, WingsPaint, WingsShape, WingsTypeStyle,
};

use crate::interaction::WingsInteractive;
use crate::scaffold::chevron_scene;
use crate::skin::{WingsSkin, pick};
use crate::text::WingsText;

// ---------------------------------------------------------------------------------------------
// Calendar math
// ---------------------------------------------------------------------------------------------

/// A calendar date (proleptic Gregorian).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WingsDate {
    /// Year.
    pub year: i32,
    /// Month, `1..=12`.
    pub month: u32,
    /// Day of the month, `1..=31`.
    pub day: u32,
}

impl Default for WingsDate {
    fn default() -> Self {
        Self::today()
    }
}

impl WingsDate {
    /// A valid date, or `None`.
    pub fn new(year: i32, month: u32, day: u32) -> Option<Self> {
        (1..=12)
            .contains(&month)
            .then_some(())
            .filter(|_| day >= 1 && day <= Self::days_in_month(year, month))
            .map(|_| Self { year, month, day })
    }

    /// True for leap years.
    pub fn is_leap(year: i32) -> bool {
        (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
    }

    /// Days in a month.
    pub fn days_in_month(year: i32, month: u32) -> u32 {
        match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if Self::is_leap(year) => 29,
            2 => 28,
            _ => 0,
        }
    }

    /// Days since 1970-01-01.
    pub fn to_days(self) -> i64 {
        let y = i64::from(self.year) - i64::from(self.month <= 2);
        let era = y.div_euclid(400);
        let yoe = y - era * 400;
        let m = i64::from(self.month);
        let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + i64::from(self.day) - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146_097 + doe - 719_468
    }

    /// The date `days` after 1970-01-01.
    pub fn from_days(days: i64) -> Self {
        let z = days + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let y = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
        let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
        Self {
            year: (y + i64::from(month <= 2)) as i32,
            month,
            day,
        }
    }

    /// Weekday, `0` = Monday ... `6` = Sunday.
    pub fn weekday(self) -> u32 {
        // 1970-01-01 was a Thursday.
        ((self.to_days() + 3).rem_euclid(7)) as u32
    }

    /// Today in UTC.
    pub fn today() -> Self {
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs() as i64);
        Self::from_days(secs.div_euclid(86_400))
    }

    /// The same day `n` months later (clamped to the month's length).
    pub fn add_months(self, n: i32) -> Self {
        let index = self.year * 12 + self.month as i32 - 1 + n;
        let year = index.div_euclid(12);
        let month = (index.rem_euclid(12) + 1) as u32;
        Self {
            year,
            month,
            day: self.day.min(Self::days_in_month(year, month)),
        }
    }

    /// English month name.
    pub fn month_name(month: u32) -> &'static str {
        const NAMES: [&str; 12] = [
            "January",
            "February",
            "March",
            "April",
            "May",
            "June",
            "July",
            "August",
            "September",
            "October",
            "November",
            "December",
        ];
        NAMES[(month.clamp(1, 12) - 1) as usize]
    }
}

// ---------------------------------------------------------------------------------------------
// Date picker
// ---------------------------------------------------------------------------------------------

/// State of a [`WingsDatePicker`].
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct WingsDatePickerState {
    /// Selected date.
    pub selected: WingsDate,
    /// Shown month (year, month).
    pub shown: (i32, u32),
    /// Weeks start on Monday (otherwise Sunday).
    pub monday_first: bool,
}

/// Triggered on a [`WingsDatePicker`] when the user picks a date.
#[derive(EntityEvent, Debug, Clone, Copy)]
pub struct WingsDateChanged {
    /// The picker.
    pub entity: Entity,
    /// The picked date.
    pub date: WingsDate,
}

/// A calendar month view.
#[derive(SceneComponent, Default, Clone)]
#[scene(WingsDatePickerProps)]
pub struct WingsDatePicker;

/// Props of [`WingsDatePicker`].
#[derive(Clone)]
pub struct WingsDatePickerProps {
    /// Initially selected date.
    pub date: WingsDate,
    /// Weeks start on Monday.
    pub monday_first: bool,
}

impl Default for WingsDatePickerProps {
    fn default() -> Self {
        Self {
            date: WingsDate::today(),
            monday_first: true,
        }
    }
}

/// The month title of a date picker.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct WingsDatePickerTitle;

/// The day grid of a date picker.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct WingsDatePickerGrid;

/// Moves the shown month by this many months.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct WingsMonthStep(pub i32);

/// A day button.
#[derive(Component, Debug, Clone, Copy)]
pub struct WingsDayCell {
    /// Its date.
    pub date: WingsDate,
}

fn month_button(step: i32) -> impl Scene {
    let rotation = if step < 0 { 180.0_f32 } else { 0.0 };
    let marker = WingsMonthStep(step);
    bsn! {
        Node {
            width: px(32),
            height: px(32),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
        }
        Button
        WingsInteractive
        TabIndex(0)
        EntityCursor::System(SystemCursorIcon::Pointer)
        WingsShape(ShapeRole::Full)
        WingsBackground
        WingsContentColor({WingsPaint::Role(ColorRole::Primary)})
        marker
        on(step_month)
        Children [
            @{chevron_scene(ColorRole::Primary)}
            UiTransform { rotation: {Rot2::degrees(rotation)} }
        ]
    }
}

impl WingsDatePicker {
    fn scene(props: WingsDatePickerProps) -> impl Scene {
        let state = WingsDatePickerState {
            selected: props.date,
            shown: (props.date.year, props.date.month),
            monday_first: props.monday_first,
        };
        bsn! {
            Node { flex_direction: FlexDirection::Column, row_gap: px(8), flex_shrink: 0.0 }
            state
            Children [
                Node { align_items: AlignItems::Center, column_gap: px(4) }
                Children [
                    @WingsText { @text: "", @role: TypeRole::TitleMedium }
                    WingsDatePickerTitle
                    Node { flex_grow: 1.0 }
                    --
                    @{month_button(-1)}
                    --
                    @{month_button(1)}
                ]
                --
                Node { display: Display::Grid }
                WingsDatePickerGrid
            ]
        }
    }
}

/// Observer: previous and next month.
pub fn step_month(
    ev: On<Activate>,
    steps: Query<&WingsMonthStep>,
    parents: Query<&ChildOf>,
    mut pickers: Query<&mut WingsDatePickerState>,
) {
    let Ok(step) = steps.get(ev.event_target()) else {
        return;
    };
    for a in bevy_wings_ui_core::hierarchy::ancestors(ev.event_target(), &parents) {
        if let Ok(mut state) = pickers.get_mut(a) {
            let shown = WingsDate {
                year: state.shown.0,
                month: state.shown.1,
                day: 1,
            }
            .add_months(step.0);
            state.shown = (shown.year, shown.month);
            return;
        }
    }
}

/// Observer: picks a day.
pub fn pick_day(
    ev: On<Activate>,
    cells: Query<&WingsDayCell>,
    parents: Query<&ChildOf>,
    mut pickers: Query<&mut WingsDatePickerState>,
    mut commands: Commands,
) {
    let Ok(cell) = cells.get(ev.event_target()) else {
        return;
    };
    for a in bevy_wings_ui_core::hierarchy::ancestors(ev.event_target(), &parents) {
        if let Ok(mut state) = pickers.get_mut(a) {
            state.selected = cell.date;
            state.shown = (cell.date.year, cell.date.month);
            commands.trigger(WingsDateChanged {
                entity: a,
                date: cell.date,
            });
            return;
        }
    }
}

/// Date picker look of a design: `(cell size, selected shape, day role)`.
pub fn date_metrics(design: WingsDesign) -> (f32, ShapeRole, TypeRole) {
    match design {
        WingsDesign::Material => (40.0, ShapeRole::Full, TypeRole::BodyMedium),
        WingsDesign::Ios => (44.0, ShapeRole::Full, TypeRole::BodyLarge),
        WingsDesign::MacOs => (24.0, ShapeRole::ExtraSmall, TypeRole::BodyMedium),
        WingsDesign::Fluent => (40.0, ShapeRole::Full, TypeRole::BodyMedium),
    }
}

/// Rebuilds the title, weekday row and day grid of pickers whose state or design changed.
pub fn date_picker_system(
    mut commands: Commands,
    skin: Res<WingsSkin>,
    pickers: Query<(Entity, Ref<WingsDatePickerState>)>,
    children: Query<&Children>,
    mut titles: Query<&mut Text, With<WingsDatePickerTitle>>,
    mut grids: Query<&mut Node, With<WingsDatePickerGrid>>,
) {
    let (cell, selected_shape, role) = date_metrics(skin.design);
    let today = WingsDate::today();
    for (picker, state) in &pickers {
        if !state.is_changed() && !skin.is_changed() {
            continue;
        }
        let tree: Vec<Entity> = children.iter_descendants(picker).collect();
        for e in &tree {
            if let Ok(mut title) = titles.get_mut(*e) {
                let text = format!("{} {}", WingsDate::month_name(state.shown.1), state.shown.0);
                if title.0 != text {
                    title.0 = text;
                }
            }
        }
        let Some(grid) = tree.iter().copied().find(|e| grids.contains(*e)) else {
            continue;
        };
        if let Ok(mut node) = grids.get_mut(grid) {
            node.grid_template_columns = vec![RepeatedGridTrack::px(7, cell)];
            node.row_gap = px(pick(skin.design, 4.0, 2.0, 2.0, 2.0));
            node.column_gap = px(pick(skin.design, 4.0, 2.0, 2.0, 2.0));
        }
        commands.entity(grid).despawn_children();
        // Weekday header.
        let names = if state.monday_first {
            ["M", "T", "W", "T", "F", "S", "S"]
        } else {
            ["S", "M", "T", "W", "T", "F", "S"]
        };
        for name in names {
            let header = commands
                .spawn((
                    Node {
                        width: px(cell),
                        height: px(cell * 0.6),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    ChildOf(grid),
                ))
                .id();
            let text = commands
                .spawn((
                    Text::new(name),
                    WingsTypeStyle(TypeRole::LabelSmall),
                    WingsForeground(WingsPaint::Role(ColorRole::OnSurfaceVariant)),
                ))
                .id();
            commands.entity(text).insert(ChildOf(header));
        }
        let first = WingsDate {
            year: state.shown.0,
            month: state.shown.1,
            day: 1,
        };
        let offset = if state.monday_first {
            first.weekday()
        } else {
            (first.weekday() + 1) % 7
        };
        for _ in 0..offset {
            commands.spawn((
                Node {
                    width: px(cell),
                    height: px(cell),
                    ..default()
                },
                ChildOf(grid),
            ));
        }
        for day in 1..=WingsDate::days_in_month(first.year, first.month) {
            let date = WingsDate { day, ..first };
            let is_selected = date == state.selected;
            let is_today = date == today;
            let (fill, text, border) = day_paint(skin.design, is_selected, is_today);
            let cell_entity = commands
                .spawn((
                    Node {
                        width: px(cell),
                        height: px(cell),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(px(if border == WingsPaint::Transparent {
                            0.0
                        } else {
                            1.0
                        })),
                        ..default()
                    },
                    Button,
                    WingsInteractive,
                    TabIndex(0),
                    WingsDayCell { date },
                    WingsShape(selected_shape),
                    WingsBackground(fill),
                    WingsBorderPaint(border),
                    WingsContentColor(text),
                    ChildOf(grid),
                ))
                .observe(pick_day)
                .id();
            let label = commands
                .spawn_scene(bsn! { @{crate::text::wings_text(day.to_string(), role)} })
                .id();
            commands.entity(label).insert(ChildOf(cell_entity));
        }
    }
}

/// Paints of a day: `(fill, text, border)`.
pub fn day_paint(
    design: WingsDesign,
    selected: bool,
    today: bool,
) -> (WingsPaint, WingsPaint, WingsPaint) {
    use ColorRole::*;
    let r = WingsPaint::Role;
    let t = WingsPaint::Transparent;
    match (design, selected, today) {
        (WingsDesign::Ios, true, _) => (r(PrimaryContainer), r(OnPrimaryContainer), t),
        (WingsDesign::Ios, false, true) => (t, r(Primary), t),
        (WingsDesign::Fluent, true, _) => (t, r(OnSurface), r(Primary)),
        (WingsDesign::Fluent, false, true) => (r(Primary), r(OnPrimary), t),
        (_, true, _) => (r(Primary), r(OnPrimary), t),
        (WingsDesign::Material, false, true) => (t, r(Primary), r(Primary)),
        (_, false, true) => (t, r(Primary), t),
        _ => (t, r(OnSurface), t),
    }
}

// ---------------------------------------------------------------------------------------------
// Time picker
// ---------------------------------------------------------------------------------------------

/// State of a [`WingsTimePicker`].
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct WingsTimePickerState {
    /// Hour, `0..24`.
    pub hour: u32,
    /// Minute, `0..60`.
    pub minute: u32,
    /// Show 24 hour time.
    pub h24: bool,
}

/// Triggered on a [`WingsTimePicker`] when the time changes.
#[derive(EntityEvent, Debug, Clone, Copy)]
pub struct WingsTimeChanged {
    /// The picker.
    pub entity: Entity,
    /// Hour, `0..24`.
    pub hour: u32,
    /// Minute.
    pub minute: u32,
}

/// Which part of the time a field edits.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum WingsTimeField {
    /// Hours.
    Hour,
    /// Minutes.
    Minute,
    /// AM or PM.
    Meridiem,
}

/// A stepper arrow of a time field.
#[derive(Component, Debug, Clone, Copy)]
pub struct WingsTimeStep {
    /// Field it changes.
    pub field: WingsTimeField,
    /// `1` up, `-1` down.
    pub delta: i32,
}

/// A time of day picker.
#[derive(SceneComponent, Default, Clone)]
#[scene(WingsTimePickerProps)]
pub struct WingsTimePicker;

/// Props of [`WingsTimePicker`].
#[derive(Clone, Default)]
pub struct WingsTimePickerProps {
    /// Initial hour, `0..24`.
    pub hour: u32,
    /// Initial minute.
    pub minute: u32,
    /// Show 24 hour time.
    pub h24: bool,
}

impl WingsTimePickerState {
    /// The state after stepping a field.
    pub fn stepped(self, field: WingsTimeField, delta: i32) -> Self {
        let mut s = self;
        match field {
            WingsTimeField::Hour => s.hour = (self.hour as i32 + delta).rem_euclid(24) as u32,
            WingsTimeField::Minute => s.minute = (self.minute as i32 + delta).rem_euclid(60) as u32,
            WingsTimeField::Meridiem => s.hour = (self.hour + 12) % 24,
        }
        s
    }

    /// Text of a field.
    pub fn text(&self, field: WingsTimeField) -> String {
        match field {
            WingsTimeField::Hour if self.h24 => format!("{:02}", self.hour),
            WingsTimeField::Hour => {
                let h = self.hour % 12;
                format!("{}", if h == 0 { 12 } else { h })
            }
            WingsTimeField::Minute => format!("{:02}", self.minute),
            WingsTimeField::Meridiem => (if self.hour < 12 { "AM" } else { "PM" }).to_string(),
        }
    }
}

fn time_field(field: WingsTimeField) -> impl Scene {
    let up = WingsTimeStep { field, delta: 1 };
    let down = WingsTimeStep { field, delta: -1 };
    bsn! {
        Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
        }
        Children [
            @{step_button(up)}
            --
            Node {
                min_width: px(44),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect::axes(px(10), px(4)),
            }
            field
            Button
            WingsInteractive
            TabIndex(0)
            WingsShape(ShapeRole::Small)
            WingsBackground
            WingsBorderPaint
            on(step_field)
            Children [ @WingsText { @text: "", @role: TypeRole::BodyLarge } ]
            --
            @{step_button(down)}
        ]
    }
}

fn step_button(step: WingsTimeStep) -> impl Scene {
    let angle = if step.delta > 0 { -90.0_f32 } else { 90.0 };
    bsn! {
        Node { width: px(28), height: px(20), justify_content: JustifyContent::Center, align_items: AlignItems::Center }
        Button
        WingsInteractive
        WingsShape(ShapeRole::Small)
        WingsBackground
        step
        on(step_time)
        Children [
            @{chevron_scene(ColorRole::OnSurfaceVariant)}
            UiTransform { rotation: {Rot2::degrees(angle)}, scale: {Vec2::splat(0.7)} }
        ]
    }
}

impl WingsTimePicker {
    fn scene(props: WingsTimePickerProps) -> impl Scene {
        let state = WingsTimePickerState {
            hour: props.hour.min(23),
            minute: props.minute.min(59),
            h24: props.h24,
        };
        let meridiem: Box<dyn Scene> = if props.h24 {
            Box::new(bsn! {})
        } else {
            Box::new(bsn! { Children [ @{time_field(WingsTimeField::Meridiem)} ] })
        };
        bsn! {
            Node { align_items: AlignItems::Center, column_gap: px(6), flex_shrink: 0.0 }
            state
            Children [
                @{time_field(WingsTimeField::Hour)}
                --
                @WingsText { @text: ":", @role: TypeRole::HeadlineSmall }
                --
                @{time_field(WingsTimeField::Minute)}
            ]
            @{meridiem}
        }
    }
}

fn apply_step(
    origin: Entity,
    field: WingsTimeField,
    delta: i32,
    parents: &Query<&ChildOf>,
    pickers: &mut Query<&mut WingsTimePickerState>,
    commands: &mut Commands,
) {
    for a in bevy_wings_ui_core::hierarchy::ancestors(origin, parents) {
        if let Ok(mut state) = pickers.get_mut(a) {
            *state = state.stepped(field, delta);
            commands.trigger(WingsTimeChanged {
                entity: a,
                hour: state.hour,
                minute: state.minute,
            });
            return;
        }
    }
}

/// Observer: stepper arrows.
pub fn step_time(
    ev: On<Activate>,
    steps: Query<&WingsTimeStep>,
    parents: Query<&ChildOf>,
    mut pickers: Query<&mut WingsTimePickerState>,
    mut commands: Commands,
) {
    if let Ok(step) = steps.get(ev.event_target()) {
        apply_step(
            ev.event_target(),
            step.field,
            step.delta,
            &parents,
            &mut pickers,
            &mut commands,
        );
    }
}

/// Observer: activating a field steps it up (AM and PM toggle).
pub fn step_field(
    ev: On<Activate>,
    fields: Query<&WingsTimeField>,
    parents: Query<&ChildOf>,
    mut pickers: Query<&mut WingsTimePickerState>,
    mut commands: Commands,
) {
    if let Ok(field) = fields.get(ev.event_target()) {
        apply_step(
            ev.event_target(),
            *field,
            1,
            &parents,
            &mut pickers,
            &mut commands,
        );
    }
}

/// Arrow keys step the focused time field.
pub fn time_picker_keys(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    focus: Option<Res<InputFocus>>,
    fields: Query<&WingsTimeField>,
    parents: Query<&ChildOf>,
    mut pickers: Query<&mut WingsTimePickerState>,
    mut commands: Commands,
) {
    let (Some(keys), Some(focus)) = (keys, focus) else {
        return;
    };
    let Some(focused) = focus.get() else {
        return;
    };
    let Ok(field) = fields.get(focused) else {
        return;
    };
    let delta = if keys.just_pressed(KeyCode::ArrowUp) {
        1
    } else if keys.just_pressed(KeyCode::ArrowDown) {
        -1
    } else {
        return;
    };
    apply_step(
        focused,
        *field,
        delta,
        &parents,
        &mut pickers,
        &mut commands,
    );
}

/// Keeps the field texts on the state and styles them per design.
pub fn time_picker_system(
    skin: Res<WingsSkin>,
    pickers: Query<(Entity, Ref<WingsTimePickerState>)>,
    children: Query<&Children>,
    mut fields: Query<(
        &WingsTimeField,
        &mut Node,
        &mut WingsBackground,
        &mut WingsBorderPaint,
        &mut WingsShape,
    )>,
    mut texts: Query<(&mut Text, &mut WingsTypeStyle)>,
) {
    use ColorRole::*;
    let r = WingsPaint::Role;
    let (fill, line, corners, role, pad_y) = match skin.design {
        WingsDesign::Material => (
            r(SurfaceContainerHighest),
            WingsPaint::Transparent,
            ShapeRole::Small,
            TypeRole::HeadlineLarge,
            8.0,
        ),
        WingsDesign::Ios => (
            r(TertiaryFill),
            WingsPaint::Transparent,
            ShapeRole::Full,
            TypeRole::BodyLarge,
            6.0,
        ),
        WingsDesign::MacOs => (
            r(SurfaceContainerLowest),
            r(Outline),
            ShapeRole::ExtraSmall,
            TypeRole::BodyMedium,
            2.0,
        ),
        WingsDesign::Fluent => (
            r(SurfaceContainerLowest),
            r(OutlineVariant),
            ShapeRole::Small,
            TypeRole::BodyMedium,
            5.0,
        ),
    };
    for (picker, state) in &pickers {
        if !state.is_changed() && !skin.is_changed() {
            continue;
        }
        for e in children.iter_descendants(picker) {
            let Ok((field, mut node, mut bg, mut border, mut shape)) = fields.get_mut(e) else {
                continue;
            };
            node.padding = UiRect::axes(px(10), px(pad_y));
            node.border = UiRect::all(px(if line == WingsPaint::Transparent {
                0.0
            } else {
                1.0
            }));
            bg.set_if_neq(WingsBackground(fill));
            border.set_if_neq(WingsBorderPaint(line));
            shape.set_if_neq(WingsShape(corners));
            let value = state.text(*field);
            for t in children.iter_descendants(e) {
                if let Ok((mut text, mut style)) = texts.get_mut(t) {
                    if text.0 != value {
                        text.0.clone_from(&value);
                    }
                    style.set_if_neq(WingsTypeStyle(role));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Calendar math round trips and knows weekdays and leap years.
    #[test]
    fn calendar() {
        let d = WingsDate::new(2026, 10, 19).unwrap();
        assert_eq!(WingsDate::from_days(d.to_days()), d);
        assert_eq!(d.weekday(), 0, "2026-10-19 is a Monday");
        assert_eq!(WingsDate::new(1970, 1, 1).unwrap().to_days(), 0);
        assert_eq!(WingsDate::days_in_month(2024, 2), 29);
        assert_eq!(WingsDate::days_in_month(2100, 2), 28);
        assert!(WingsDate::new(2026, 2, 30).is_none());
        assert_eq!(
            WingsDate::new(2026, 1, 31).unwrap().add_months(1),
            WingsDate::new(2026, 2, 28).unwrap()
        );
        assert_eq!(
            WingsDate::new(2026, 1, 15).unwrap().add_months(-2),
            WingsDate::new(2025, 11, 15).unwrap()
        );
        for days in [-1000, 0, 365, 20_000] {
            assert_eq!(WingsDate::from_days(days).to_days(), days);
        }
    }

    /// Time stepping wraps and formats like the platforms.
    #[test]
    fn time_steps() {
        let s = WingsTimePickerState {
            hour: 23,
            minute: 59,
            h24: false,
        };
        let next = s.stepped(WingsTimeField::Minute, 1);
        assert_eq!(next.minute, 0);
        assert_eq!(s.stepped(WingsTimeField::Hour, 1).hour, 0);
        assert_eq!(s.text(WingsTimeField::Hour), "11");
        assert_eq!(s.text(WingsTimeField::Meridiem), "PM");
        assert_eq!(s.stepped(WingsTimeField::Meridiem, 1).hour, 11);
        let h24 = WingsTimePickerState {
            hour: 7,
            minute: 5,
            h24: true,
        };
        assert_eq!(h24.text(WingsTimeField::Hour), "07");
        assert_eq!(h24.text(WingsTimeField::Minute), "05");
    }

    /// iOS selects with a light tinted circle, Fluent rings the selection.
    #[test]
    fn day_paints() {
        let (fill, _, _) = day_paint(WingsDesign::Ios, true, false);
        assert_eq!(fill, WingsPaint::Role(ColorRole::PrimaryContainer));
        let (_, _, border) = day_paint(WingsDesign::Fluent, true, false);
        assert_eq!(border, WingsPaint::Role(ColorRole::Primary));
    }
}
