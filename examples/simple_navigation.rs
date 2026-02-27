//! An example showing a navigation visualization rendered for a simple 3x3 grid of buttons.
//! The arrow keys / D-pad can be used to change focus between buttons, which will
//! update the visualization depending on the draw mode.
//! Plugin settings and the example itself can be changed with certain key-presses.
use bevy::input::keyboard::Key;
use bevy::input_focus::{
    InputDispatchPlugin, InputFocus, InputFocusVisible,
    directional_navigation::{DirectionalNavigationMap, DirectionalNavigationPlugin},
};
use bevy::math::CompassOctant;
use bevy::platform::collections::HashSet;
use bevy::prelude::*;
use bevy::ui::auto_directional_navigation::{AutoDirectionalNavigation, AutoDirectionalNavigator};
use marsh_compass::{AutoNavVizColorMode, AutoNavVizGizmoConfigGroup, AutoNavVizPlugin};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            InputDispatchPlugin,
            DirectionalNavigationPlugin,
        ))
        .insert_resource(InputFocusVisible(true))
        // Add this plugin for the visualization to render.
        .add_plugins(AutoNavVizPlugin)
        // Example specific resources
        .init_resource::<OrderedButtons>()
        .init_resource::<DirectionalColorsToggle>()
        .init_resource::<ActionState>()
        .add_systems(Startup, setup)
        .add_systems(
            PreUpdate,
            (process_directional_inputs, update_example, navigate),
        )
        .add_systems(Update, highlight_input_focus)
        .run();
}

/// A resource used to cache the buttons in the specific setup ordering.
/// Used to easily add overrides to the navigation map.
#[derive(Resource, Deref, DerefMut)]
struct OrderedButtons {
    buttons: Vec<Entity>,
}

impl Default for OrderedButtons {
    fn default() -> Self {
        OrderedButtons {
            buttons: Vec::with_capacity(9),
        }
    }
}

fn setup(
    mut commands: Commands,
    mut buttons: ResMut<OrderedButtons>,
    mut input_focus: ResMut<InputFocus>,
) {
    commands.spawn(Camera2d);
    let root_id = commands
        .spawn((Node {
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Center,
            align_content: AlignContent::SpaceEvenly,
            justify_content: JustifyContent::Center,
            ..default()
        },))
        .id();

    let rules_container = commands
        .spawn((
            Node {
                width: px(400),
                justify_content: JustifyContent::Center,
                ..default()
            },
            children![Text::new(
                "Use the D-Pad or Arrow Keys to navigate.\n\n\
                Press '1' to toggle the draw mode.\n\n\
                Press '2' to toggle the directional colors.\n\n\
                Press '3' to toggle mixing the entity's color into directional colors.\n\n\
                Press 'w'/'s' to increase / decrease the arrow tip size.\n\n\
                Press 'l' to toggle looping navigation edges for border buttons."
            ),],
        ))
        .id();
    commands.entity(root_id).add_child(rules_container);

    let button_container = commands
        .spawn((Node {
            width: px(500),
            height: px(500),
            ..default()
        },))
        .id();
    commands.entity(root_id).add_child(button_container);

    let positions = [
        [(0, 0), (200, 0), (400, 0)],
        [(0, 200), (200, 200), (400, 200)],
        [(0, 400), (200, 400), (400, 400)],
    ];
    for (i, row) in positions.iter().enumerate() {
        for (j, (left, top)) in row.iter().enumerate() {
            let button = spawn_button(&mut commands, *left, *top, i, j);
            commands.entity(button_container).add_child(button);
            // buttons are added left to right, top to bottom.
            buttons.push(button);
            if i == 1 && j == 1 {
                input_focus.set(button);
            }
        }
    }
}

fn spawn_button(commands: &mut Commands, left: i32, top: i32, i: usize, j: usize) -> Entity {
    commands
        .spawn((
            Button,
            Node {
                position_type: PositionType::Absolute,
                left: px(left),
                top: px(top),
                width: px(100),
                height: px(100),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(px(3)),
                border_radius: BorderRadius::all(px(12)),
                ..default()
            },
            BackgroundColor(Color::Srgba(Srgba::RED)),
            AutoDirectionalNavigation::default(),
        ))
        .with_child((
            Text::new(format!("Button {}", i * 3 + j)),
            TextLayout {
                justify: Justify::Center,
                ..default()
            },
        ))
        .id()
}

// The input focus always has a white border.
fn highlight_input_focus(
    input_focus: Res<InputFocus>,
    input_focus_visible: Res<InputFocusVisible>,
    mut query: Query<(Entity, &mut BorderColor)>,
) {
    for (entity, mut border_color) in query.iter_mut() {
        if input_focus.get() == Some(entity) && input_focus_visible.0 {
            *border_color = BorderColor::all(Color::Srgba(Srgba::WHITE));
        } else {
            *border_color = BorderColor::DEFAULT;
        }
    }
}

#[derive(Resource, Default, Deref, DerefMut)]
struct DirectionalColorsToggle(bool);

fn update_example(
    mut config_store: ResMut<GizmoConfigStore>,
    keyboard: Res<ButtonInput<Key>>,
    mut colors_toggle: ResMut<DirectionalColorsToggle>,
    buttons: Res<OrderedButtons>,
    mut override_map: ResMut<DirectionalNavigationMap>,
    real_time: Res<Time<Real>>,
) {
    // update config
    let (_, group_config) = config_store.config_mut::<AutoNavVizGizmoConfigGroup>();
    if keyboard.just_pressed(Key::Character("1".into())) {
        group_config.toggle_draw_mode();
    }
    if keyboard.just_pressed(Key::Character("2".into())) {
        if colors_toggle.0 {
            group_config.set_directional_colors_to_defaults();
            colors_toggle.0 = false;
        } else {
            group_config.set_directional_colors_to_none();
            colors_toggle.0 = true;
        }
    }
    if keyboard.just_pressed(Key::Character("3".into())) {
        if group_config.color_mode == AutoNavVizColorMode::NoMix {
            group_config.color_mode = AutoNavVizColorMode::MixWithEntity(0.5);
        } else {
            group_config.color_mode = AutoNavVizColorMode::NoMix;
        }
    }
    if keyboard.pressed(Key::Character("w".into())) {
        group_config.arrow_tip_length += 5. * real_time.delta_secs();
        group_config.arrow_tip_length = group_config.arrow_tip_length.clamp(0., 25.);
    }
    if keyboard.pressed(Key::Character("s".into())) {
        group_config.arrow_tip_length -= 5. * real_time.delta_secs();
        group_config.arrow_tip_length = group_config.arrow_tip_length.clamp(0., 25.);
    }

    // update example
    if keyboard.just_pressed(Key::Character("l".into())) {
        if override_map.neighbors.is_empty() {
            for row in 0..3 {
                override_map.add_looping_edges(&buttons[row * 3..row * 3 + 3], CompassOctant::East);
            }
            for col in 0..3 {
                let col_buttons = [buttons[col], buttons[col + 3], buttons[col + 6]];
                override_map.add_looping_edges(&col_buttons, CompassOctant::South);
            }
            override_map.add_looping_edges(
                &[buttons[0], buttons[4], buttons[8]],
                CompassOctant::SouthEast,
            );
            override_map.add_looping_edges(
                &[buttons[2], buttons[4], buttons[6]],
                CompassOctant::SouthWest,
            );
        } else {
            override_map.clear();
        }
    }
}

// Below is some boilerplate from the directional_navigation.rs example in `Bevy`
// It handles Keyboard Arrow & Enter / Gamepad D-pad & South input and executes navigation

// Action state and input handling
#[derive(Debug, PartialEq, Eq, Hash)]
enum DirectionalNavigationAction {
    Up,
    Down,
    Left,
    Right,
    Select,
}

impl DirectionalNavigationAction {
    fn variants() -> Vec<Self> {
        vec![
            DirectionalNavigationAction::Up,
            DirectionalNavigationAction::Down,
            DirectionalNavigationAction::Left,
            DirectionalNavigationAction::Right,
            DirectionalNavigationAction::Select,
        ]
    }

    fn keycode(&self) -> KeyCode {
        match self {
            DirectionalNavigationAction::Up => KeyCode::ArrowUp,
            DirectionalNavigationAction::Down => KeyCode::ArrowDown,
            DirectionalNavigationAction::Left => KeyCode::ArrowLeft,
            DirectionalNavigationAction::Right => KeyCode::ArrowRight,
            DirectionalNavigationAction::Select => KeyCode::Enter,
        }
    }

    fn gamepad_button(&self) -> GamepadButton {
        match self {
            DirectionalNavigationAction::Up => GamepadButton::DPadUp,
            DirectionalNavigationAction::Down => GamepadButton::DPadDown,
            DirectionalNavigationAction::Left => GamepadButton::DPadLeft,
            DirectionalNavigationAction::Right => GamepadButton::DPadRight,
            DirectionalNavigationAction::Select => GamepadButton::South,
        }
    }
}

#[derive(Default, Resource)]
struct ActionState {
    pressed_actions: HashSet<DirectionalNavigationAction>,
}

fn process_directional_inputs(
    mut action_state: ResMut<ActionState>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    gamepad_input: Query<&Gamepad>,
) {
    action_state.pressed_actions.clear();

    for action in DirectionalNavigationAction::variants() {
        if keyboard_input.just_pressed(action.keycode()) {
            action_state.pressed_actions.insert(action);
        }
    }

    for gamepad in gamepad_input.iter() {
        for action in DirectionalNavigationAction::variants() {
            if gamepad.just_pressed(action.gamepad_button()) {
                action_state.pressed_actions.insert(action);
            }
        }
    }
}

fn navigate(
    action_state: Res<ActionState>,
    mut auto_directional_navigator: AutoDirectionalNavigator,
) {
    let net_east_west = action_state
        .pressed_actions
        .contains(&DirectionalNavigationAction::Right) as i8
        - action_state
            .pressed_actions
            .contains(&DirectionalNavigationAction::Left) as i8;

    let net_north_south = action_state
        .pressed_actions
        .contains(&DirectionalNavigationAction::Up) as i8
        - action_state
            .pressed_actions
            .contains(&DirectionalNavigationAction::Down) as i8;

    // Use Dir2::from_xy to convert input to direction, then convert to CompassOctant
    let maybe_direction = Dir2::from_xy(net_east_west as f32, net_north_south as f32)
        .ok()
        .map(CompassOctant::from);

    if let Some(direction) = maybe_direction {
        match auto_directional_navigator.navigate(direction) {
            Ok(_entity) => {
                // Successfully navigated
            }
            Err(_e) => {
                // Navigation failed (no neighbor in that direction)
            }
        }
    }
}
