//! # Marsh Compass
//! A Bevy Plugin that provides visualizations of the auto directional navigation
//! system in Bevy's UI Framework - `bevy_ui`.
//!
//! # Usage
//! Simply add the [`AutoNavVizPlugin`] plugin to your app that has auto
//! directional navigation enabled setup.
//!
//! ```no_run
//! # use bevy::prelude::*;
//! # use bevy::input_focus::{
//! #   InputDispatchPlugin,
//! #   directional_navigation::DirectionalNavigationPlugin
//! # };
//! # use crate::AutoNavVizPlugin;
//! fn main() {
//!     App::new()
//!         .add_plugins((
//!             DefaultPlugins,
//!             InputDispatchPlugin, // Needed for input focus
//!             DirectionalNavigationPlugin, // Needed for auto directional nav
//!             AutoNavVizPlugin // Add this plugin
//!         ))
//!         .run();
//! }
//! ```
//!
//! # Configuration
//! The plugin can be configured via its gizmo config group [`AutoNavVizGizmoConfigGroup`].
//! ```
//! # use bevy::gizmos::config::GizmoConfigGroup;
//! # use bevy::prelude::*;
//! fn setup(config_store: ResMut<GizmoConfigStore>) {
//!     let mut config = config_store.config_mut::<AutoNavVizGizmoConfigGroup>().1;
//!     config.drawing_mode = AutoNavVizMode::EnabledForAll;
//! }
//! ```

use bevy::gizmos::config::GizmoConfigGroup;
use bevy::prelude::*;

mod nav_viz_map;
pub use nav_viz_map::*;
mod visualizer;
pub use visualizer::*;

/// System set for the visualization systems executed in the [`AutoNavVizPlugin`].
///
/// This system set runs in the [`PostUpdate`] schedule after [`TransformSystems::Propagate`].
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AutoNavVizSystems;

/// A [`Plugin`] that adds visualizations for auto navigation systems.
#[derive(Default)]
pub struct AutoNavVizPlugin;

impl Plugin for AutoNavVizPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NavVizMap>()
            .init_gizmo_group::<AutoNavVizGizmoConfigGroup>()
            .add_systems(
                PostUpdate,
                (
                    nav_viz_map::rebuild_nav_viz_map,
                    visualizer::draw_viz_for_current_focus.run_if(
                        |config: Res<GizmoConfigStore>| {
                            config
                                .config::<AutoNavVizGizmoConfigGroup>()
                                .1
                                .drawing_mode
                                .is_enabled()
                        },
                    ),
                )
                    .chain()
                    .after(TransformSystems::Propagate)
                    .in_set(AutoNavVizSystems),
            );
    }
}

/// Setting for whether the navigation visualization should be:
/// - disabled,
/// - drawn for the current focus only, or
/// - drawn for all [`AutoDirectionalNavigation`](bevy::ui::auto_directional_navigation::AutoDirectionalNavigation)
///   entities rendered to the same target as the current focus.
#[derive(Clone, Default, Debug, Reflect, PartialEq, Eq, Hash)]
pub enum AutoNavVizMode {
    Disabled,
    #[default]
    EnabledForCurrentFocus,
    EnabledForAll,
}

impl AutoNavVizMode {
    pub fn is_enabled(&self) -> bool {
        *self != AutoNavVizMode::Disabled
    }
}

/// The [`GizmoConfigGroup`] for the auto navigation visualizations
/// that will be drawn.
#[derive(Default, Reflect, GizmoConfigGroup)]
#[reflect(Default)]
pub struct AutoNavVizGizmoConfigGroup {
    /// The drawing mode for auto navigation visualizations.
    /// See [`AutoNavVizMode`] for more details.
    pub drawing_mode: AutoNavVizMode,
}
