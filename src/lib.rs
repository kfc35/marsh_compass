//! # Marsh Compass
//! A Bevy Plugin that provides visualization for the auto directional navigation
//! system in Bevy's UI Framework - `bevy_ui`. 
//! 

use bevy::prelude::*;
use bevy::input_focus::directional_navigation::FocusableArea;
use bevy::ui::auto_directional_navigation::{AutoDirectionalNavigator, AutoDirectionalNavigation};

mod nav_map;
pub use nav_map::*;

/// Adds visualizations for auto navigation systems.
#[derive(Default)]
pub struct AutoNavVizPlugin;

impl Plugin for AutoNavVizPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NavVizMap>();
    }
}

fn draw_viz_for_current_focus(mut gizmos: Gizmos,
    mut navigator: AutoDirectionalNavigator,

) {
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
