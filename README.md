# Marsh Compass
A Bevy Plugin that draws a visualization of the auto directional navigation in Bevy's UI Framework.

## Usage
Simply add the `AutoNavVizPlugin` plugin to your app that has auto
directional navigation enabled setup.

```rust
fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            InputDispatchPlugin, // Needed for input focus
            DirectionalNavigationPlugin, // Needed for auto directional nav
            AutoNavVizPlugin // Add this plugin
        ))
        .run();
}
```

## Configuration
The plugin can be configured via its gizmo config group `AutoNavVizGizmoConfigGroup`.

``` rust
fn setup(config_store: ResMut<GizmoConfigStore>) {
    let mut config = config_store.config_mut::<AutoNavVizGizmoConfigGroup>().1;
    config.drawing_mode = AutoNavVizDrawMode::EnabledForAll;
}
```

## License
This project is dual-licensed under
- MIT License ([LICENSE-MIT](LICENSE-MIT) or https://opensource.org/license/MIT)
- Apache License Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or https://www.apache.org/licenses/LICENSE-2.0)

at your discretion.