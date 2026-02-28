//! An example showing a visualization rendered for two buttons. One button is located in the center,
//! while the other rotates around the center.
use bevy::input_focus::{
    InputDispatchPlugin, InputFocus, InputFocusVisible,
    directional_navigation::DirectionalNavigationPlugin,
};
use bevy::prelude::*;
use bevy::ui::auto_directional_navigation::AutoDirectionalNavigation;
use bevy_auto_nav_viz::AutoNavVizPlugin;

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
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, mut input_focus: ResMut<InputFocus>) {
    commands.spawn(Camera2d);
    let root_id = commands
        .spawn((Node {
            width: percent(100),
            height: percent(100),
            ..default()
        },))
        .id();

    let positions = [
        [(100, 100), (300, 100), (500, 100)],
        [(100, 300), (300, 300), (500, 300)],
        [(100, 500), (300, 500), (500, 500)],
    ];
    for (i, row) in positions.iter().enumerate() {
        for (j, (left, top)) in row.iter().enumerate() {
            let button = spawn_button(&mut commands, *left, *top, i, j);
            commands.entity(root_id).add_child(button);
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
                border: UiRect::all(px(5)),
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
