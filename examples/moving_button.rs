//! An example showing the visualization rendered for two buttons:
//! One button is located in the center of the window ("Center Button"),
//! while the other moves around it ("Moving Button").
//! The moving button also rotates.
//!
//! This example shows what the visualization can look with certain placements of buttons.
//! This also shows the behavior of the auto navigation system itself, i.e.
//! when and where it draws an edge from one button to the other.
//!
//! This example starts off using the default
//! [`AutoNavigationConfig`](bevy::input_focus::directional_navigation::AutoNavigationConfig).
//! Changing the navigation config will result in different navigation behavior
//! (and thus will change the visualization).
use std::f32::consts::PI;

use bevy::ecs::system::SystemParam;
use bevy::input::keyboard::Key;
use bevy::input_focus::directional_navigation::AutoNavigationConfig;
use bevy::input_focus::{
    InputDispatchPlugin, InputFocus, InputFocusVisible,
    directional_navigation::DirectionalNavigationMap,
    directional_navigation::DirectionalNavigationPlugin,
};
use bevy::math::CompassOctant;
use bevy::platform::collections::HashSet;
use bevy::prelude::*;
use bevy::ui::auto_directional_navigation::{AutoDirectionalNavigation, AutoDirectionalNavigator};
use bevy_auto_nav_viz::AutoNavVizPlugin;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            InputDispatchPlugin,
            DirectionalNavigationPlugin,
        ))
        .insert_resource(InputFocusVisible(true))
        // Example specific resource
        .init_resource::<ActionState>()
        .init_resource::<TranslationToggle>()
        .init_resource::<ScaleToggle>()
        .init_resource::<OrderedButtons>()
        // Add this plugin for the visualization to render.
        .add_plugins(AutoNavVizPlugin)
        .add_systems(Startup, setup)
        .add_systems(
            PreUpdate,
            (process_directional_inputs, process_toggles, navigate),
        )
        .add_systems(Update, (highlight_input_focus, move_button))
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

/// A marker component for the button that moves
#[derive(Component)]
struct MovingButton;

/// A resource that denotes whether the [`MovingButton`] can move via translation.
#[derive(Resource)]
struct TranslationToggle(bool);

impl Default for TranslationToggle {
    fn default() -> Self {
        Self(true)
    }
}

/// A resource that denotes whether the [`MovingButton`] can is scaled long.
#[derive(Resource)]
struct ScaleToggle(bool);

impl Default for ScaleToggle {
    fn default() -> Self {
        Self(false)
    }
}

fn setup(
    mut commands: Commands,
    mut input_focus: ResMut<InputFocus>,
    mut buttons: ResMut<OrderedButtons>,
    window: Single<&Window>,
) {
    commands.spawn(Camera2d);

    let root_id = commands
        .spawn((Node {
            width: percent(100),
            height: percent(100),
            ..default()
        },))
        .id();

    let rules_container = commands
        .spawn((
            Node {
                left: px(20),
                top: px(20),
                justify_content: JustifyContent::Center,
                ..default()
            },
            children![Text::new(
                "Press `1` to toggle translation.\n\n\
                Press `2` to toggle rotation.\n\n\
                Press `3` to toggle scaling.\n\n\
                Press `4` to toggle manual looping edge E <-> W.\n\n\
                Press `5` to toggle manual looping edge N <-> S.\n\n\
                Press `6` to cycle through min alignment factors.\n\n\
                Use the D-Pad or Arrow Keys to navigate."
            ),],
        ))
        .id();
    commands.entity(root_id).add_child(rules_container);

    let settings_container = commands
        .spawn((
            Node {
                display: Display::Block,
                top: px(20),
                margin: UiRect {
                    left: px(20),
                    ..default()
                },
                ..default()
            },
            children![(
                Node { ..default() },
                MinAlignmentFactorText,
                Text::new("Min Alignment Factor: 0\n\n")
            ),],
        ))
        .id();
    commands.entity(root_id).add_child(settings_container);

    let window_logical_size = window.resolution.size();

    // Place in the middle of the screen, taking into account button size,
    let center_button = spawn_button(
        &mut commands,
        window_logical_size / 2. - 50.,
        "Center Button",
    );
    commands.entity(root_id).add_child(center_button);
    input_focus.set(center_button);

    let moving_button = spawn_button(
        &mut commands,
        window_logical_size / 2. - 50.,
        "Moving Button",
    );
    commands.entity(root_id).add_child(moving_button);
    // Translate the moving button to be 300px above the center button
    commands.entity(moving_button).insert((
        MovingButton,
        UiTransform {
            translation: Val2 {
                x: px(0.),
                y: px(-250.),
            },
            ..default()
        },
    ));

    buttons.push(center_button);
    buttons.push(moving_button);
}

fn spawn_button(commands: &mut Commands, left_and_top: Vec2, name: &str) -> Entity {
    commands
        .spawn((
            Button,
            Node {
                position_type: PositionType::Absolute,
                left: px(left_and_top.x),
                top: px(left_and_top.y),
                width: px(100),
                height: px(100),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(px(5)),
                border_radius: BorderRadius::all(px(12)),
                ..default()
            },
            BackgroundColor(Color::Srgba(Srgba::RED)),
            AutoDirectionalNavigation::default(),
        ))
        .with_child((
            Text::new(name),
            TextLayout {
                justify: Justify::Center,
                ..default()
            },
        ))
        .id()
}

fn move_button(
    time: Res<Time<Virtual>>,
    translation_toggle: Res<TranslationToggle>,
    mut moving_button_query: Query<&mut UiTransform, With<MovingButton>>,
) -> Result {
    for mut transform in &mut moving_button_query {
        // Translate the button to make it appear like it is rotating
        // around the center
        if translation_toggle.0 {
            let (x, y) = if let Val::Px(x) = transform.translation.x
                && let Val::Px(y) = transform.translation.y
            {
                (x, y)
            } else {
                panic!("UiTransform's translation must be defined in Px.");
            };
            let translation = Rot2::degrees(0.25) * Vec2::new(x, y);
            transform.translation = Val2 {
                x: px(translation.x),
                y: px(translation.y),
            }
        }

        transform.rotation = Rot2::radians(time.elapsed_secs() % (2. * PI));
    }
    Ok(())
}

fn process_toggles(
    keyboard: Res<ButtonInput<Key>>,
    mut translation_toggle: ResMut<TranslationToggle>,
    mut moving_button_query: Query<&mut UiTransform, With<MovingButton>>,
    mut scale_toggle: ResMut<ScaleToggle>,
    mut virtual_time: ResMut<Time<Virtual>>,
    buttons: Res<OrderedButtons>,
    mut override_map: ResMut<DirectionalNavigationMap>,
    mut config: ResMut<AutoNavigationConfig>,
    mut get_text: GetTextParam,
) -> Result {
    if keyboard.just_pressed(Key::Character("1".into())) {
        translation_toggle.0 ^= true;
    }
    if keyboard.just_pressed(Key::Character("2".into())) {
        if virtual_time.is_paused() {
            virtual_time.unpause()
        } else {
            virtual_time.pause()
        }
    }
    if keyboard.just_pressed(Key::Character("3".into())) {
        scale_toggle.0 ^= true;

        for mut transform in &mut moving_button_query {
            if scale_toggle.0 {
                transform.scale = Vec2::new(2., 1.);
            } else {
                transform.scale = Vec2::ONE;
            }
        }
    }
    if keyboard.just_pressed(Key::Character("4".into())) {
        if override_map
            .get_neighbor(buttons[0], CompassOctant::East)
            .is_none()
        {
            override_map.add_looping_edges(&[buttons[0], buttons[1]], CompassOctant::East);
        } else {
            override_map
                .neighbors
                .get_mut(&buttons[0])
                .unwrap()
                .neighbors[CompassOctant::East.to_index()] = None;
            override_map
                .neighbors
                .get_mut(&buttons[0])
                .unwrap()
                .neighbors[CompassOctant::West.to_index()] = None;
            override_map
                .neighbors
                .get_mut(&buttons[1])
                .unwrap()
                .neighbors[CompassOctant::East.to_index()] = None;
            override_map
                .neighbors
                .get_mut(&buttons[1])
                .unwrap()
                .neighbors[CompassOctant::West.to_index()] = None;
        }
    }
    if keyboard.just_pressed(Key::Character("5".into())) {
        if override_map
            .get_neighbor(buttons[0], CompassOctant::North)
            .is_none()
        {
            override_map.add_looping_edges(&[buttons[0], buttons[1]], CompassOctant::North);
        } else {
            override_map
                .neighbors
                .get_mut(&buttons[0])
                .unwrap()
                .neighbors[CompassOctant::North.to_index()] = None;
            override_map
                .neighbors
                .get_mut(&buttons[0])
                .unwrap()
                .neighbors[CompassOctant::South.to_index()] = None;
            override_map
                .neighbors
                .get_mut(&buttons[1])
                .unwrap()
                .neighbors[CompassOctant::North.to_index()] = None;
            override_map
                .neighbors
                .get_mut(&buttons[1])
                .unwrap()
                .neighbors[CompassOctant::South.to_index()] = None;
        }
    }
    if keyboard.just_pressed(Key::Character("6".into())) {
        if config.min_alignment_factor == 1. {
            config.min_alignment_factor = 0.;
        } else {
            config.min_alignment_factor += 0.25;
        }
        get_text.min_alignment_query.single_mut()?.0 =
            format!("Min Alignment Factor: {}\n\n", config.min_alignment_factor);
    }

    Ok(())
}

/// Marker component describing the state of looping edges
#[derive(Component)]
struct MinAlignmentFactorText;

/// System param for easily getting text.
#[derive(SystemParam)]
struct GetTextParam<'w, 's> {
    min_alignment_query: Query<'w, 's, &'static mut Text, With<MinAlignmentFactorText>>,
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
