//! # Marsh Compass
//! A Bevy Plugin that provides visualization for the auto directional navigation
//! system in Bevy's UI Framework - `bevy_ui`.
//!

use bevy::gizmos::config::GizmoConfigGroup;
use bevy::input_focus::{InputFocus, directional_navigation::NavNeighbors};
use bevy::math::CompassOctant;
use bevy::prelude::*;

mod nav_viz_map;
pub use nav_viz_map::*;

/// A [`Plugin`] that adds visualizations for auto navigation systems.
#[derive(Default)]
pub struct AutoNavVizPlugin;

/// Setting for whether the navigation visualization should be:
/// - disabled,
/// - drawn for the current focus only, or
/// - drawn for all [`AutoDirectionalNavigation`] entities
///
/// The "all entities" setting is restricted to entities rendered to the
/// same camera as the current focus.
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

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AutoNavVizSystems;

impl Plugin for AutoNavVizPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NavVizMap>()
            .init_gizmo_group::<AutoNavVizGizmoConfigGroup>()
            .add_systems(
                PostUpdate,
                (rebuild_nav_viz_map, draw_viz_for_current_focus)
                    .chain()
                    .run_if(|config: Res<GizmoConfigStore>| {
                        config
                            .config::<AutoNavVizGizmoConfigGroup>()
                            .1
                            .drawing_mode
                            .is_enabled()
                    })
                    .after(TransformSystems::Propagate)
                    .in_set(AutoNavVizSystems),
            );
    }
}

/// The gizmo config group for the auto navigation visualizations
/// that will be drawn.
#[derive(Default, Reflect, GizmoConfigGroup)]
#[reflect(Default)]
pub struct AutoNavVizGizmoConfigGroup {
    /// The drawing mode for auto navigation visualizations.
    /// See [`AutoNavVizMode`] for more details.
    pub drawing_mode: AutoNavVizMode,
}

/// The system that draws the visualizations of the auto navigation
/// system. It uses gizmos to draw arrows between entities.
pub fn draw_viz_for_current_focus(
    mut gizmos: Gizmos<AutoNavVizGizmoConfigGroup>,
    nav_viz_map: Res<NavVizMap>,
    input_focus: Res<InputFocus>,
    config: Res<GizmoConfigStore>,
) {
    let entities_to_draw_nav = match config.config::<AutoNavVizGizmoConfigGroup>().1.drawing_mode {
        AutoNavVizMode::EnabledForAll => nav_viz_map
            .map
            .neighbors
            .iter()
            .collect::<Vec<(&Entity, &NavNeighbors)>>(),
        AutoNavVizMode::EnabledForCurrentFocus => {
            if let Some(entity) = &input_focus.0
                && let Some(neighbors) = nav_viz_map.map.get_neighbors(*entity)
            {
                vec![(entity, neighbors)]
            } else {
                vec![]
            }
        }
        AutoNavVizMode::Disabled => vec![],
    };

    for (entity, neighbors) in entities_to_draw_nav.into_iter() {
        for (i, maybe_neighbor) in neighbors.neighbors.iter().enumerate() {
            let Some(neighbor) = maybe_neighbor else {
                continue;
            };

            let Some((entity_pos, entity_size)) = nav_viz_map
                .entity_viz_data
                .get(entity)
                .map(|fa| (fa.world_position, fa.size))
            else {
                continue;
            };
            let Some(direction) = CompassOctant::from_index(i) else {
                continue;
            };
            let shifted_entity_pos = shift_position(entity_pos, entity_size, direction);

            let Some((neighbor_pos, neighbor_size)) = nav_viz_map
                .entity_viz_data
                .get(neighbor)
                .map(|fa| (fa.world_position, fa.size))
            else {
                continue;
            };
            let shifted_neighbor_pos =
                shift_position(neighbor_pos, neighbor_size, direction.opposite());

            gizmos.arrow_2d(
                shifted_entity_pos,
                shifted_neighbor_pos,
                Color::Srgba(Srgba::RED),
            );
        }
    }
}

fn shift_position(pos: Vec2, size: Vec2, dir: CompassOctant) -> Vec2 {
    match dir {
        CompassOctant::North => pos + Vec2::new(0., size.y / 2.),
        CompassOctant::NorthEast => pos + (size / 2.),
        CompassOctant::East => pos + Vec2::new(size.x / 2., 0.),
        CompassOctant::SouthEast => pos + (Vec2::new(size.x, -size.y) / 2.),
        CompassOctant::South => pos + Vec2::new(0., -size.y / 2.),
        CompassOctant::SouthWest => pos - (size / 2.),
        CompassOctant::West => pos + Vec2::new(-size.x / 2., 0.),
        CompassOctant::NorthWest => pos + (Vec2::new(-size.x, size.y) / 2.),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
