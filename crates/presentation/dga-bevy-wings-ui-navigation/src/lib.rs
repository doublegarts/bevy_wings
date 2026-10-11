//! # dga-bevy-wings-ui-navigation
//!
//! Navigation where **the back stack is just state**.
//!
//! * [`WingsBackStack<K>`](back_stack::WingsBackStack) is a resource holding a `Vec<K>` of
//!   your own key type. Push, pop or rewrite it from any system; there is no hidden controller.
//! * [`WingsNavGraph<K>`] maps each key to a [`NavDestination`]: a `bsn!` scene plus
//!   metadata (pane role, custom transitions).
//! * A node with [`WingsNavDisplay<K>`] renders the stack: it spawns destinations lazily,
//!   keeps entries below the top alive but hidden (their state and view models survive), plays
//!   push and pop transitions, and despawns popped entries.
//! * [`NavSceneStrategy`] turns the stack into one or two
//!   visible panes depending on the window width class (list and detail side by side on wide
//!   windows).
//! * Each entry root carries a [`WingsDiScope`], so scoped dependencies and view models
//!   live exactly as long as the entry.
//! * Back navigation from Escape and the mouse back button, plus [`WingsNavCommand<K>`]
//!   messages for event driven code.
//!
//! ```ignore
//! #[derive(Clone, PartialEq, Eq, Hash, Debug)]
//! enum Screen { Inbox, Message(u32) }
//!
//! app.add_plugins(WingsNavigationPlugin::new(Screen::Inbox, |key: &Screen| match key {
//!     Screen::Inbox => nav_entry(bsn! { @InboxScreen }).list_pane(),
//!     Screen::Message(id) => nav_entry(message_screen(*id)).detail_pane(),
//! }));
//!
//! commands.spawn_scene(bsn! { WingsNavDisplay::<Screen> Node { width: percent(100), height: percent(100) } });
//!
//! fn open(mut stack: ResMut<WingsBackStack<Screen>>) { stack.push(Screen::Message(7)); }
//! ```

pub mod back_stack;
pub mod scene_strategy;

use core::marker::PhantomData;
use std::sync::Arc;

use bevy::input::ButtonInput;
use bevy::input::keyboard::KeyCode;
use bevy::input::mouse::MouseButton;
use bevy::prelude::*;
use bevy::ui::{Display, Node, PositionType, UiTransform, Val};

use dga_bevy_wings_di::WingsDiScope;
use dga_bevy_wings_ui_animation::prelude::{
    OpacityProperty, ScaleProperty, TranslationProperty, WingsAnimator, WingsTransition,
};
use dga_bevy_wings_ui_core::{WingsOpacity, WingsUiSystems, ensure_core};
use dga_bevy_wings_ui_responsive::{WidthClass, WingsViewport};

pub use back_stack::{NavKey, WingsBackStack};
pub use scene_strategy::{NavSceneStrategy, PaneRole, PaneSlot};

/// Enter and exit transitions for the four navigation situations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WingsNavTransitions {
    /// The new entry after a push.
    pub push_enter: WingsTransition,
    /// The covered entry after a push.
    pub push_exit: WingsTransition,
    /// The revealed entry after a pop.
    pub pop_enter: WingsTransition,
    /// The removed entry after a pop.
    pub pop_exit: WingsTransition,
}

impl Default for WingsNavTransitions {
    fn default() -> Self {
        Self::slide()
    }
}

impl WingsNavTransitions {
    /// Horizontal slide with fade, the default.
    pub fn slide() -> Self {
        Self {
            push_enter: WingsTransition::slide_horizontally(80.0).and(WingsTransition::fade()),
            push_exit: WingsTransition::slide_horizontally(-40.0).and(WingsTransition::fade()),
            pop_enter: WingsTransition::slide_horizontally(-40.0).and(WingsTransition::fade()),
            pop_exit: WingsTransition::slide_horizontally(80.0).and(WingsTransition::fade()),
        }
    }

    /// Cross fade.
    pub fn fade() -> Self {
        Self {
            push_enter: WingsTransition::fade(),
            push_exit: WingsTransition::fade(),
            pop_enter: WingsTransition::fade(),
            pop_exit: WingsTransition::fade(),
        }
    }

    /// Scale and fade, for modal like destinations.
    pub fn zoom() -> Self {
        Self {
            push_enter: WingsTransition::scale(0.92).and(WingsTransition::fade()),
            push_exit: WingsTransition::fade(),
            pop_enter: WingsTransition::fade(),
            pop_exit: WingsTransition::scale(0.92).and(WingsTransition::fade()),
        }
    }

    /// No animation.
    pub fn none() -> Self {
        Self {
            push_enter: WingsTransition::none(),
            push_exit: WingsTransition::none(),
            pop_enter: WingsTransition::none(),
            pop_exit: WingsTransition::none(),
        }
    }
}

/// What a key resolves to.
pub struct NavDestination {
    scene: Box<dyn Scene>,
    /// Pane role used by the scene strategy.
    pub pane: PaneRole,
    /// Transitions overriding the display defaults when this entry enters or leaves.
    pub transitions: Option<WingsNavTransitions>,
}

/// Creates a destination from a scene.
pub fn nav_entry(scene: impl Scene) -> NavDestination {
    NavDestination {
        scene: Box::new(scene),
        pane: PaneRole::Single,
        transitions: None,
    }
}

impl NavDestination {
    /// Marks the destination as a list pane.
    pub fn list_pane(mut self) -> Self {
        self.pane = PaneRole::List;
        self
    }

    /// Marks the destination as a detail pane.
    pub fn detail_pane(mut self) -> Self {
        self.pane = PaneRole::Detail;
        self
    }

    /// Marks the destination as supporting content.
    pub fn supporting_pane(mut self) -> Self {
        self.pane = PaneRole::Supporting;
        self
    }

    /// Overrides the transitions for this destination.
    pub fn with_transitions(mut self, transitions: WingsNavTransitions) -> Self {
        self.transitions = Some(transitions);
        self
    }
}

type Builder<K> = Arc<dyn Fn(&K) -> NavDestination + Send + Sync>;

/// Maps keys to destinations.
#[derive(Resource)]
pub struct WingsNavGraph<K: NavKey> {
    builder: Builder<K>,
    roles: std::sync::Mutex<bevy::platform::collections::HashMap<K, PaneRole>>,
}

impl<K: NavKey> WingsNavGraph<K> {
    /// Creates a graph from a builder function.
    pub fn new(builder: impl Fn(&K) -> NavDestination + Send + Sync + 'static) -> Self {
        Self {
            builder: Arc::new(builder),
            roles: Default::default(),
        }
    }

    /// Builds the destination for `key`.
    pub fn build(&self, key: &K) -> NavDestination {
        let d = (self.builder)(key);
        self.roles
            .lock()
            .expect("nav graph lock")
            .insert(key.clone(), d.pane);
        d
    }

    /// Pane role of `key` (builds the destination once to learn it, then caches it).
    pub fn role(&self, key: &K) -> PaneRole {
        if let Some(r) = self.roles.lock().expect("nav graph lock").get(key) {
            return *r;
        }
        self.build(key).pane
    }
}

/// Renders a [`WingsBackStack<K>`] inside this node.
#[derive(Component)]
#[require(Node)]
pub struct WingsNavDisplay<K: NavKey> {
    /// Scene strategy.
    pub strategy: NavSceneStrategy,
    /// Default transitions.
    pub transitions: WingsNavTransitions,
    _marker: PhantomData<fn() -> K>,
}

impl<K: NavKey> Default for WingsNavDisplay<K> {
    fn default() -> Self {
        Self {
            strategy: NavSceneStrategy::default(),
            transitions: WingsNavTransitions::default(),
            _marker: PhantomData,
        }
    }
}

impl<K: NavKey> Clone for WingsNavDisplay<K> {
    fn clone(&self) -> Self {
        Self {
            strategy: self.strategy,
            transitions: self.transitions,
            _marker: PhantomData,
        }
    }
}

impl<K: NavKey> WingsNavDisplay<K> {
    /// A display with a strategy and transitions.
    pub fn new(strategy: NavSceneStrategy, transitions: WingsNavTransitions) -> Self {
        Self {
            strategy,
            transitions,
            _marker: PhantomData,
        }
    }
}

/// Internal state of a display.
#[derive(Component)]
pub struct WingsNavDisplayState<K: NavKey> {
    keys: Vec<K>,
    entities: Vec<Option<Entity>>,
    transitions: Vec<Option<WingsNavTransitions>>,
    visible: Vec<usize>,
    class: Option<WidthClass>,
}

impl<K: NavKey> Default for WingsNavDisplayState<K> {
    fn default() -> Self {
        Self {
            keys: Vec::new(),
            entities: Vec::new(),
            transitions: Vec::new(),
            visible: Vec::new(),
            class: None,
        }
    }
}

impl<K: NavKey> WingsNavDisplayState<K> {
    /// Spawned entry roots aligned with the back stack (`None` = not spawned yet).
    pub fn entities(&self) -> &[Option<Entity>] {
        &self.entities
    }

    /// Stack indices currently visible.
    pub fn visible(&self) -> &[usize] {
        &self.visible
    }
}

/// Placed on every entry root.
#[derive(Component, Debug, Clone)]
pub struct WingsNavEntry<K: NavKey> {
    /// The key.
    pub key: K,
    /// Position in the back stack.
    pub index: usize,
}

/// Marks an entry that plays an exit transition. When it completes the entry is hidden
/// (`despawn == false`) or despawned.
#[derive(Component, Debug, Clone, Copy)]
pub struct WingsNavExiting {
    /// Despawn instead of hiding.
    pub despawn: bool,
}

/// Navigation lifecycle notifications.
#[derive(Message, Debug, Clone, PartialEq)]
pub enum WingsNavEvent<K: NavKey> {
    /// An entry root was spawned.
    Spawned {
        /// Key.
        key: K,
        /// Root entity.
        entity: Entity,
    },
    /// An entry left the back stack (its root is despawned after its exit transition).
    Removed {
        /// Key.
        key: K,
    },
}

/// Event driven navigation commands.
#[derive(Message, Debug, Clone, PartialEq)]
pub enum WingsNavCommand<K: NavKey> {
    /// Push a destination.
    Push(K),
    /// Push unless already on top.
    PushSingleTop(K),
    /// Pop the top destination.
    Pop,
    /// Replace the top destination.
    Replace(K),
    /// Pop until `K` is on top (`true` = pop it too).
    PopTo(K, bool),
    /// Reset to a single destination.
    Reset(K),
}

/// Keys and buttons that trigger back navigation.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct WingsBackInput {
    /// Keyboard keys.
    pub keys: Vec<KeyCode>,
    /// Use the mouse back button.
    pub mouse_back: bool,
}

impl Default for WingsBackInput {
    fn default() -> Self {
        Self {
            keys: vec![KeyCode::Escape, KeyCode::BrowserBack],
            mouse_back: true,
        }
    }
}

/// Applies [`WingsNavCommand`] messages to the back stack.
pub fn apply_nav_commands<K: NavKey>(
    mut reader: MessageReader<WingsNavCommand<K>>,
    mut stack: ResMut<WingsBackStack<K>>,
) {
    for cmd in reader.read() {
        match cmd.clone() {
            WingsNavCommand::Push(k) => stack.push(k),
            WingsNavCommand::PushSingleTop(k) => stack.push_single_top(k),
            WingsNavCommand::Pop => {
                stack.pop();
            }
            WingsNavCommand::Replace(k) => stack.replace_top(k),
            WingsNavCommand::PopTo(k, inclusive) => {
                stack.pop_to(&k, inclusive);
            }
            WingsNavCommand::Reset(k) => stack.reset(k),
        }
    }
}

/// Pops the back stack on back input.
pub fn back_input<K: NavKey>(
    input: Res<WingsBackInput>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    mut stack: ResMut<WingsBackStack<K>>,
) {
    let key_back = keys.is_some_and(|k| k.any_just_pressed(input.keys.iter().copied()));
    let mouse_back = input.mouse_back && mouse.is_some_and(|m| m.just_pressed(MouseButton::Back));
    if (key_back || mouse_back) && stack.can_pop() {
        stack.pop();
    }
}

fn pane_node(slot: PaneSlot) -> Node {
    let (left, width) = match slot {
        PaneSlot::Full => (Val::Px(0.0), Val::Percent(100.0)),
        PaneSlot::Leading(w) => (Val::Px(0.0), Val::Percent(w)),
        PaneSlot::Trailing(start) => (Val::Percent(start), Val::Percent(100.0 - start)),
    };
    Node {
        position_type: PositionType::Absolute,
        left,
        top: Val::Px(0.0),
        width,
        height: Val::Percent(100.0),
        flex_direction: FlexDirection::Column,
        ..default()
    }
}

/// Synchronizes displays with the back stack.
pub fn sync_nav_displays<K: NavKey>(
    stack: Res<WingsBackStack<K>>,
    graph: Res<WingsNavGraph<K>>,
    viewport: Option<Res<WingsViewport>>,
    mut displays: Query<(
        Entity,
        &WingsNavDisplay<K>,
        Option<&mut WingsNavDisplayState<K>>,
    )>,
    mut roots: Query<&mut Node, With<WingsNavEntry<K>>>,
    mut commands: Commands,
    mut events: MessageWriter<WingsNavEvent<K>>,
) {
    let class = viewport
        .as_ref()
        .map_or(WidthClass::Compact, |v| v.width_class);
    for (display_entity, display, state) in &mut displays {
        let Some(mut state) = state else {
            commands
                .entity(display_entity)
                .insert(WingsNavDisplayState::<K>::default());
            continue;
        };
        let first = state.class.is_none();
        if !first && !stack.is_changed() && state.class == Some(class) {
            continue;
        }
        let new_keys = stack.entries();
        let prefix = state
            .keys
            .iter()
            .zip(new_keys)
            .take_while(|(a, b)| a == b)
            .count();
        let forward = new_keys.len() > prefix;

        // Entries removed from the stack.
        let old_visible = core::mem::take(&mut state.visible);
        for i in prefix..state.keys.len() {
            events.write(WingsNavEvent::Removed {
                key: state.keys[i].clone(),
            });
            let Some(e) = state.entities[i] else { continue };
            let transitions = state.transitions[i].unwrap_or(display.transitions);
            if old_visible.contains(&i) && !first {
                let exit = if forward {
                    transitions.push_exit
                } else {
                    transitions.pop_exit
                };
                commands
                    .entity(e)
                    .insert(WingsNavExiting { despawn: true })
                    .queue(move |mut ew: EntityWorldMut| exit.play_exit(&mut ew));
            } else {
                commands.entity(e).despawn();
            }
        }
        state.keys.truncate(prefix);
        state.entities.truncate(prefix);
        state.transitions.truncate(prefix);
        for k in &new_keys[prefix..] {
            state.keys.push(k.clone());
            state.entities.push(None);
            state.transitions.push(None);
        }

        // Plan the visible panes.
        let roles: Vec<PaneRole> = new_keys.iter().map(|k| graph.role(k)).collect();
        let plan = display.strategy.plan(&roles, class);

        for &(index, slot) in &plan {
            let was_visible = old_visible.contains(&index) && index < prefix;
            match state.entities[index] {
                Some(e) => {
                    if let Ok(mut node) = roots.get_mut(e) {
                        *node = pane_node(slot);
                    }
                    if !was_visible && !first {
                        let t = state.transitions[index].unwrap_or(display.transitions);
                        let enter = if forward { t.push_enter } else { t.pop_enter };
                        commands
                            .entity(e)
                            .remove::<WingsNavExiting>()
                            .queue(move |mut ew: EntityWorldMut| enter.play_enter(&mut ew));
                    }
                }
                None => {
                    let key = new_keys[index].clone();
                    let destination = graph.build(&key);
                    state.transitions[index] = destination.transitions;
                    let root = commands
                        .spawn((
                            Name::new(format!("WingsNavEntry({key:?})")),
                            pane_node(slot),
                            WingsOpacity::OPAQUE,
                            UiTransform::default(),
                            WingsDiScope,
                            WingsNavEntry {
                                key: key.clone(),
                                index,
                            },
                            ChildOf(display_entity),
                        ))
                        .id();
                    commands
                        .spawn_scene(destination.scene)
                        .insert(ChildOf(root));
                    state.entities[index] = Some(root);
                    events.write(WingsNavEvent::Spawned { key, entity: root });
                    if !first {
                        let t = destination.transitions.unwrap_or(display.transitions);
                        let enter = if forward { t.push_enter } else { t.pop_enter };
                        commands
                            .entity(root)
                            .queue(move |mut ew: EntityWorldMut| enter.play_enter(&mut ew));
                    }
                }
            }
        }

        // Entries that stay in the stack but are no longer visible.
        for &i in &old_visible {
            if i < prefix
                && !plan.iter().any(|(p, _)| *p == i)
                && let Some(e) = state.entities[i]
            {
                let t = state.transitions[i].unwrap_or(display.transitions);
                let exit = t.push_exit;
                commands
                    .entity(e)
                    .insert(WingsNavExiting { despawn: false })
                    .queue(move |mut ew: EntityWorldMut| exit.play_exit(&mut ew));
            }
        }

        state.visible = plan.iter().map(|(i, _)| *i).collect();
        state.class = Some(class);
    }
}

/// Hides or despawns entries whose exit transition completed.
pub fn finish_nav_exits(
    mut commands: Commands,
    mut query: Query<(
        Entity,
        &WingsNavExiting,
        &mut Node,
        Option<&WingsAnimator<OpacityProperty>>,
        Option<&WingsAnimator<TranslationProperty>>,
        Option<&WingsAnimator<ScaleProperty>>,
    )>,
) {
    for (e, exiting, mut node, a, b, c) in &mut query {
        let busy = a.is_some_and(|x| x.is_active())
            || b.is_some_and(|x| x.is_active())
            || c.is_some_and(|x| x.is_active());
        if busy {
            continue;
        }
        if exiting.despawn {
            commands.entity(e).despawn();
        } else {
            node.display = Display::None;
            commands.entity(e).remove::<WingsNavExiting>();
        }
    }
}

/// Adds navigation for key type `K`.
pub struct WingsNavigationPlugin<K: NavKey> {
    start: K,
    builder: std::sync::Mutex<Option<Builder<K>>>,
    /// Pop on Escape and mouse back.
    pub back_input: bool,
}

impl<K: NavKey> WingsNavigationPlugin<K> {
    /// Navigation starting at `start`, resolving keys with `builder`.
    pub fn new(start: K, builder: impl Fn(&K) -> NavDestination + Send + Sync + 'static) -> Self {
        Self {
            start,
            builder: std::sync::Mutex::new(Some(Arc::new(builder))),
            back_input: true,
        }
    }

    /// Disables the built in back input handling.
    pub fn without_back_input(mut self) -> Self {
        self.back_input = false;
        self
    }
}

impl<K: NavKey> Plugin for WingsNavigationPlugin<K> {
    fn build(&self, app: &mut App) {
        ensure_core(app);
        if !app.world().contains_resource::<WingsBackStack<K>>() {
            app.insert_resource(WingsBackStack::new(self.start.clone()));
        }
        let builder = self
            .builder
            .lock()
            .expect("plugin lock")
            .take()
            .expect("plugin built once");
        app.insert_resource(WingsNavGraph::<K> {
            builder,
            roles: Default::default(),
        });
        app.init_resource::<WingsBackInput>()
            .add_message::<WingsNavEvent<K>>()
            .add_message::<WingsNavCommand<K>>();
        app.add_systems(
            Update,
            (apply_nav_commands::<K>, sync_nav_displays::<K>)
                .chain()
                .in_set(WingsUiSystems::Navigation),
        );
        if self.back_input {
            app.add_systems(Update, back_input::<K>.in_set(WingsUiSystems::Input));
        }
        if !app.world().contains_resource::<FinishExitsAdded>() {
            app.insert_resource(FinishExitsAdded);
            app.add_systems(Update, finish_nav_exits.after(WingsUiSystems::Animate));
        }
    }
}

#[derive(Resource)]
struct FinishExitsAdded;

/// Commonly used items.
pub mod prelude {
    pub use crate::back_stack::{NavKey, WingsBackStack};
    pub use crate::scene_strategy::{NavSceneStrategy, PaneRole, PaneSlot};
    pub use crate::{
        NavDestination, WingsBackInput, WingsNavCommand, WingsNavDisplay, WingsNavDisplayState,
        WingsNavEntry, WingsNavEvent, WingsNavGraph, WingsNavTransitions, WingsNavigationPlugin,
        nav_entry,
    };
}

#[cfg(test)]
mod tests {
    use super::prelude::*;
    use bevy::prelude::*;
    use bevy::scene::ScenePlugin;
    use bevy::time::TimeUpdateStrategy;
    use core::time::Duration;
    use dga_bevy_wings_di::WingsDiScope;
    use dga_bevy_wings_ui_animation::WingsAnimationPlugin;
    use dga_bevy_wings_ui_responsive::{WingsResponsivePlugin, WingsViewportOverride};

    #[derive(Clone, PartialEq, Eq, Hash, Debug)]
    enum Screen {
        Home,
        Inbox,
        Message(u32),
    }

    #[derive(Component, Default, Clone, Debug, PartialEq)]
    struct ScreenTag(u32);

    fn app(width: f32) -> (App, Entity) {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            ScenePlugin,
            WingsAnimationPlugin,
            WingsResponsivePlugin::default(),
            WingsNavigationPlugin::new(Screen::Home, |key: &Screen| match key {
                Screen::Home => nav_entry(bsn! { ScreenTag(0) }),
                Screen::Inbox => nav_entry(bsn! { ScreenTag(1) }).list_pane(),
                Screen::Message(id) => {
                    let id = *id;
                    nav_entry(bsn! { ScreenTag({100 + id}) }).detail_pane()
                }
            }),
        ));
        app.insert_resource(WingsViewportOverride {
            width,
            height: 800.0,
        });
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            50,
        )));
        // Input resources without the input plugin, so tests control `just_pressed` directly.
        app.init_resource::<ButtonInput<KeyCode>>();
        app.init_resource::<ButtonInput<MouseButton>>();
        let display = app
            .world_mut()
            .spawn(WingsNavDisplay::<Screen>::default())
            .id();
        app.update();
        app.update();
        (app, display)
    }

    fn settle(app: &mut App) {
        for _ in 0..15 {
            app.update();
        }
    }

    fn state(app: &App, display: Entity) -> (Vec<Option<Entity>>, Vec<usize>) {
        let s = app
            .world()
            .get::<WingsNavDisplayState<Screen>>(display)
            .unwrap();
        (s.entities().to_vec(), s.visible().to_vec())
    }

    fn tag_of(app: &App, root: Entity) -> u32 {
        let children = app.world().get::<Children>(root).unwrap();
        app.world().get::<ScreenTag>(children[0]).unwrap().0
    }

    fn display_of(app: &App, e: Entity) -> Display {
        app.world().get::<Node>(e).unwrap().display
    }

    /// The start destination is spawned inside the display with its own DI scope.
    #[test]
    fn start_destination_is_spawned() {
        let (app, display) = app(400.0);
        let (entities, visible) = state(&app, display);
        assert_eq!(visible, vec![0]);
        let root = entities[0].unwrap();
        assert_eq!(tag_of(&app, root), 0);
        assert!(app.world().entity(root).contains::<WingsDiScope>());
        assert_eq!(app.world().get::<ChildOf>(root).unwrap().parent(), display);
    }

    /// Pushing keeps the covered entry alive (hidden) and popping reveals the very same entity,
    /// so screen state survives.
    #[test]
    fn push_hides_and_pop_restores_same_entity() {
        let (mut app, display) = app(400.0);
        let home = state(&app, display).0[0].unwrap();
        app.world_mut()
            .resource_mut::<WingsBackStack<Screen>>()
            .push(Screen::Message(7));
        settle(&mut app);
        let (entities, visible) = state(&app, display);
        assert_eq!(visible, vec![1]);
        let message = entities[1].unwrap();
        assert_eq!(tag_of(&app, message), 107);
        assert_eq!(display_of(&app, home), Display::None);

        app.world_mut()
            .resource_mut::<WingsBackStack<Screen>>()
            .pop();
        settle(&mut app);
        let (entities, visible) = state(&app, display);
        assert_eq!(visible, vec![0]);
        assert_eq!(entities, vec![Some(home)]);
        assert_ne!(display_of(&app, home), Display::None);
        assert!(
            app.world().get_entity(message).is_err(),
            "popped entry despawned"
        );
    }

    /// On a wide window a list and a detail entry are shown side by side; shrinking the
    /// window collapses to the detail only.
    #[test]
    fn list_detail_adapts_to_width() {
        let (mut app, display) = app(1300.0);
        {
            let mut stack = app.world_mut().resource_mut::<WingsBackStack<Screen>>();
            stack.push(Screen::Inbox);
            stack.push(Screen::Message(1));
        }
        settle(&mut app);
        let (entities, visible) = state(&app, display);
        assert_eq!(visible, vec![1, 2]);
        let list = entities[1].unwrap();
        let detail = entities[2].unwrap();
        assert_eq!(
            app.world().get::<Node>(list).unwrap().width,
            Val::Percent(38.0)
        );
        assert_eq!(
            app.world().get::<Node>(detail).unwrap().left,
            Val::Percent(38.0)
        );

        app.insert_resource(WingsViewportOverride {
            width: 500.0,
            height: 800.0,
        });
        settle(&mut app);
        let (_, visible) = state(&app, display);
        assert_eq!(visible, vec![2]);
        assert_eq!(display_of(&app, list), Display::None);
        assert_eq!(
            app.world().get::<Node>(detail).unwrap().width,
            Val::Percent(100.0)
        );
    }

    /// Messages drive the stack, and lifecycle events are reported.
    #[test]
    fn commands_and_events() {
        let (mut app, _display) = app(400.0);
        app.world_mut()
            .write_message(WingsNavCommand::Push(Screen::Inbox));
        app.update();
        assert_eq!(
            app.world().resource::<WingsBackStack<Screen>>().top(),
            Some(&Screen::Inbox)
        );
        app.world_mut()
            .write_message(WingsNavCommand::<Screen>::Pop);
        app.update();
        let msgs = app.world().resource::<Messages<WingsNavEvent<Screen>>>();
        let mut cursor = msgs.get_cursor();
        let all: Vec<_> = cursor.read(msgs).cloned().collect();
        assert!(all.contains(&WingsNavEvent::Removed { key: Screen::Inbox }));
        assert!(all.iter().any(|e| matches!(
            e,
            WingsNavEvent::Spawned {
                key: Screen::Inbox,
                ..
            }
        )));
    }

    /// Escape pops the stack but never the last entry.
    #[test]
    fn escape_pops() {
        let (mut app, _display) = app(400.0);
        app.world_mut()
            .resource_mut::<WingsBackStack<Screen>>()
            .push(Screen::Inbox);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        app.update();
        assert_eq!(app.world().resource::<WingsBackStack<Screen>>().len(), 1);
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.release(KeyCode::Escape);
            keys.clear();
            keys.press(KeyCode::Escape);
        }
        app.update();
        assert_eq!(app.world().resource::<WingsBackStack<Screen>>().len(), 1);
    }

    /// Replacing the top entry spawns the new destination and removes the old one.
    #[test]
    fn replace_top() {
        let (mut app, display) = app(400.0);
        app.world_mut()
            .resource_mut::<WingsBackStack<Screen>>()
            .push(Screen::Message(1));
        settle(&mut app);
        let old = state(&app, display).0[1].unwrap();
        app.world_mut()
            .resource_mut::<WingsBackStack<Screen>>()
            .replace_top(Screen::Message(2));
        settle(&mut app);
        let new = state(&app, display).0[1].unwrap();
        assert_ne!(old, new);
        assert_eq!(tag_of(&app, new), 102);
        assert!(app.world().get_entity(old).is_err());
    }
}
