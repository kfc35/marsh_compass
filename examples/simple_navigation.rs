use bevy::input_focus::{
    InputDispatchPlugin, InputFocus, InputFocusVisible,
    directional_navigation::DirectionalNavigationPlugin,
};
use bevy::prelude::*;
use bevy::ui::auto_directional_navigation::AutoDirectionalNavigation;
use marsh_compass::AutoNavVizPlugin;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            InputDispatchPlugin,
            DirectionalNavigationPlugin,
        ))
        .add_plugins(AutoNavVizPlugin)
        .insert_resource(InputFocusVisible(true))
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

    let positions = [(100, 100), (300, 100)];
    for (left, top) in positions {
        let button = spawn_button(&mut commands, left, top);
        commands.entity(root_id).add_child(button);
        if input_focus.get().is_none() {
            input_focus.set(button);
        }
    }
}

fn spawn_button(commands: &mut Commands, left: i32, top: i32) -> Entity {
    commands
        .spawn((
            Button,
            Node {
                position_type: PositionType::Absolute,
                left: px(left),
                top: px(top),
                width: px(100),
                height: px(50),
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
            Text::new("Hello"),
            TextLayout {
                justify: Justify::Center,
                ..default()
            },
        ))
        .id()
}
