//! # Marsh Compass
//! A Bevy Plugin that provides visualization for the auto directional navigation
//! system in Bevy's UI Framework - `bevy_ui`. 
//! 

use bevy::prelude::{App, Plugin};

/// Adds visualizations for auto navigation systems.
#[derive(Default)]
pub struct AutoNavVizPlugin;

impl Plugin for AutoNavVizPlugin {
    fn build(&self, app: &mut App) {
        todo!()
    }
}

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
