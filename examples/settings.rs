//! An example showcasing the different settings of the [`AutoNavVizPlugin`],
//! available for tuning via the [`AutoNavVizGizmoConfigGroup`].
//!
//! A navigation visualization is rendered for a simple 3x3 grid of buttons.
//! The arrow keys / D-pad can be used to change focus between buttons. If the draw
//! mode is set to "Enabled for Current Focus", this will update the navigation
//! visualization.
//! The plugin settings and the example itself can be changed with certain key-presses.
//! Run the example and feel free to play around with the different settings
//! to see which one best fits your style/application.
//! If you have an idea for any additional settings, feel free to make a Github
//! issue requesting your feature.
use bevy::ecs::system::SystemParam;
use bevy::input::keyboard::Key;
use bevy::input_focus::{
    InputDispatchPlugin, InputFocus, InputFocusVisible,
    directional_navigation::{DirectionalNavigationMap, DirectionalNavigationPlugin},
};
use bevy::math::CompassOctant;
use bevy::platform::collections::HashSet;
use bevy::prelude::*;
use bevy::ui::auto_directional_navigation::{AutoDirectionalNavigation, AutoDirectionalNavigator};
use bevy_auto_nav_viz::{
    AutoNavVizColorMode, AutoNavVizDrawMode, AutoNavVizGizmoConfigGroup, AutoNavVizPlugin,
    SymmetricalEdgeSettings,
};

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

/// Marker component describing the current draw mode
#[derive(Component)]
struct DrawModeText;

/// Marker component describing the state of directional colors
#[derive(Component)]
struct DirectionalColorsText;

/// Marker component describing the current color mode
#[derive(Component)]
struct ColorModeText;

/// Marker component describing the state of looping edges
#[derive(Component)]
struct LoopingEdgesText;

/// System param for easily getting text.
#[derive(SystemParam)]
struct GetTextParam<'w, 's> {
    draw_mode_query: Query<
        'w,
        's,
        &'static mut Text,
        (
            With<DrawModeText>,
            Without<DirectionalColorsText>,
            Without<ColorModeText>,
            Without<LoopingEdgesText>,
        ),
    >,
    dir_color_query: Query<
        'w,
        's,
        &'static mut Text,
        (
            With<DirectionalColorsText>,
            Without<DrawModeText>,
            Without<ColorModeText>,
            Without<LoopingEdgesText>,
        ),
    >,
    color_mode_query: Query<
        'w,
        's,
        &'static mut Text,
        (
            With<ColorModeText>,
            Without<DrawModeText>,
            Without<DirectionalColorsText>,
            Without<LoopingEdgesText>,
        ),
    >,
    looping_edges_query: Query<
        'w,
        's,
        &'static mut Text,
        (
            With<LoopingEdgesText>,
            Without<DrawModeText>,
            Without<DirectionalColorsText>,
            Without<ColorModeText>,
        ),
    >,
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
                width: px(300),
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
            margin: UiRect {
                left: px(50),
                right: px(50),
                ..default()
            },
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

    let settings_container = commands
        .spawn((
            Node {
                width: px(300),
                display: Display::Block,
                ..default()
            },
            children![
                (
                    Node {
                        width: percent(100),
                        ..default()
                    },
                    DrawModeText,
                    Text::new("Draw Mode: Enabled for Current Focus\n\n")
                ),
                (
                    Node {
                        width: percent(100),
                        ..default()
                    },
                    DirectionalColorsText,
                    Text::new("Directional Colors: Default\n\n")
                ),
                (
                    Node {
                        width: percent(100),
                        ..default()
                    },
                    ColorModeText,
                    Text::new("Color Mode: Directional Only\n\n")
                ),
                (
                    Node {
                        width: percent(100),
                        ..default()
                    },
                    LoopingEdgesText,
                    Text::new("Looping Edges: Inactive\n\n")
                ),
            ],
        ))
        .id();
    commands.entity(root_id).add_child(settings_container);
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
    mut get_text_param: GetTextParam,
) {
    // update config
    let (_, group_config) = config_store.config_mut::<AutoNavVizGizmoConfigGroup>();
    if keyboard.just_pressed(Key::Character("1".into())) {
        let mut draw_mode_text = get_text_param.draw_mode_query.single_mut().unwrap();
        group_config.draw_mode = match group_config.draw_mode {
            AutoNavVizDrawMode::EnabledForCurrentFocus => {
                draw_mode_text.0 = "Draw Mode: Enabled For All - Merge and Gradient\n\n".into();
                AutoNavVizDrawMode::EnabledForAll(SymmetricalEdgeSettings::MergeAndGradient)
            }
            AutoNavVizDrawMode::EnabledForAll(symm_settings) => match symm_settings {
                SymmetricalEdgeSettings::MergeAndGradient => {
                    draw_mode_text.0 =
                        "Draw Mode: Enabled For All - Merge and Mix Evenly\n\n".into();
                    AutoNavVizDrawMode::EnabledForAll(
                        SymmetricalEdgeSettings::merge_and_mix_evenly(),
                    )
                }
                SymmetricalEdgeSettings::MergeAndMix(_) => {
                    draw_mode_text.0 =
                        "Draw Mode: Enabled For All - Spacing Between Single Arrows\n\n".into();
                    AutoNavVizDrawMode::EnabledForAll(
                        SymmetricalEdgeSettings::SpacingBetweenSingleArrows,
                    )
                }
                SymmetricalEdgeSettings::SpacingBetweenSingleArrows => {
                    draw_mode_text.0 =
                        "Draw Mode: Enabled For All - Overlapping Single Arrows\n\n".into();
                    AutoNavVizDrawMode::EnabledForAll(
                        SymmetricalEdgeSettings::OverlappingSingleArrows,
                    )
                }
                SymmetricalEdgeSettings::OverlappingSingleArrows => {
                    draw_mode_text.0 = "Draw Mode: Enabled For Current Focus\n\n".into();
                    AutoNavVizDrawMode::EnabledForCurrentFocus
                }
            },
        };
    }
    if keyboard.just_pressed(Key::Character("2".into())) {
        let mut directional_colors_text = get_text_param.dir_color_query.single_mut().unwrap();
        if colors_toggle.0 {
            group_config.set_directional_colors_to_defaults();
            colors_toggle.0 = false;
            directional_colors_text.0 = "Directional Colors: Default\n\n".into();
        } else {
            group_config.set_directional_colors_to_none();
            colors_toggle.0 = true;
            directional_colors_text.0 = "Directional Colors: None\n\n".into();
        }
    }
    if keyboard.just_pressed(Key::Character("3".into())) {
        let mut color_mode_text = get_text_param.color_mode_query.single_mut().unwrap();
        group_config.color_mode = match group_config.color_mode {
            AutoNavVizColorMode::DirectionalOnly => {
                color_mode_text.0 =
                    "Color Mode: Mix Directional Coolor with Source Entity Evenly\n\n".into();
                AutoNavVizColorMode::mix_with_source_entity_evenly()
            }
            AutoNavVizColorMode::MixedWithSourceEntity(factor) => {
                if factor <= 0.5 {
                    color_mode_text.0 = "Color Mode: Only Source Entity\n\n".into();
                    AutoNavVizColorMode::source_entity_color_only()
                } else {
                    color_mode_text.0 =
                        "Color Mode: Mix Directional Color with Destination Entity Evenly\n\n"
                            .into();
                    AutoNavVizColorMode::mix_with_destination_entity_evenly()
                }
            }
            AutoNavVizColorMode::MixedWithDestinationEntity(factor) => {
                if factor <= 0.5 {
                    color_mode_text.0 = "Color Mode: Only Destination Entity\n\n".into();
                    AutoNavVizColorMode::destination_entity_color_only()
                } else {
                    color_mode_text.0 = "Color Mode: Directional only\n\n".into();
                    AutoNavVizColorMode::DirectionalOnly
                }
            }
        };
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
        let mut looping_edges_text = get_text_param.looping_edges_query.single_mut().unwrap();
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
            looping_edges_text.0 = "Looping Edges: Active\n\n".into();
        } else {
            override_map.clear();
            looping_edges_text.0 = "Looping Edges: Inactive\n\n".into();
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
