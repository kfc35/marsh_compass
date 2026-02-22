//! # Marsh Compass
//! A Bevy Plugin that provides visualization for the auto directional navigation
//! system in Bevy's UI Framework - `bevy_ui`.
//!

use bevy::gizmos::config::GizmoConfigGroup;
use bevy::prelude::*;

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
                    .after(TransformSystems::Propagate)
                    .in_set(AutoNavVizSystems),
            );
    }
}

#[derive(Default, Reflect, GizmoConfigGroup)]
#[reflect(Default)]
pub struct AutoNavVizGizmoConfigGroup;

fn draw_viz_for_current_focus(
    mut gizmos: Gizmos<AutoNavVizGizmoConfigGroup>,
    nav_viz_map: Res<NavVizMap>,
) {
    for (entity, neighbors) in nav_viz_map.map.neighbors.iter() {
        for (_i, maybe_neighbor) in neighbors.neighbors.iter().enumerate() {
            let Some(neighbor) = maybe_neighbor else {
                continue;
            };

            let Some((entity_pos, _entity_size)) = nav_viz_map
                .entity_viz_data
                .get(entity)
                .map(|fa| (fa.position, fa.size))
            else {
                continue;
            };

            let Some((neighbor_pos, _neighbor_size)) = nav_viz_map
                .entity_viz_data
                .get(neighbor)
                .map(|fa| (fa.position, fa.size))
            else {
                continue;
            };

            gizmos.arrow_2d(entity_pos, neighbor_pos, Color::Srgba(Srgba::GREEN));
        }
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
