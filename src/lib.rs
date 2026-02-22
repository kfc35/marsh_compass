//! # Marsh Compass
//! A Bevy Plugin that provides visualization for the auto directional navigation
//! system in Bevy's UI Framework - `bevy_ui`.
//!

use bevy::input_focus::directional_navigation::FocusableArea;
use bevy::prelude::*;
use bevy::ui::auto_directional_navigation::{AutoDirectionalNavigation, AutoDirectionalNavigator};

mod nav_viz_map;
pub use nav_viz_map::*;

/// Adds visualizations for auto navigation systems.
#[derive(Default)]
pub struct AutoNavVizPlugin;

/// Settings Resource for navigation visualization
#[derive(Resource, Default, Deref, DerefMut)]
pub struct AutoNavVizSettings(AutoNavVizMode);

/// Whether the navigation visualization should be:
/// - disabled,
/// - drawn for the current focus only, or
/// - drawn for all [`AutoDirectionalNavigation`] entities
///
/// The "all entities" setting is restricted to entities rendered to the
/// same camera as the current focus.
#[derive(Clone, Default, Debug)]
pub enum AutoNavVizMode {
    Disabled,
    #[default]
    EnabledForCurrentFocus,
    EnabledForAll,
}

impl Plugin for AutoNavVizPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NavVizMap>();
    }
}

fn draw_viz_for_current_focus(mut gizmos: Gizmos, mut navigator: AutoDirectionalNavigator) {
    let Some(focus) = navigator.input_focus() else {
        return;
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
