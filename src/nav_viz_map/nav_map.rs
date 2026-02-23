use bevy::input_focus::directional_navigation::{
    AutoNavigationConfig, DirectionalNavigationMap, FocusableArea, NavNeighbors,
    auto_generate_navigation_edges,
};
use bevy::prelude::*;

/// Rebuilds the navigation map field in [`NavVizMap`](crate::nav_viz_map::NavVizMap).
///
/// It clears the map, adds the navigation edges that the automatic directional navigation
/// system would use for all the entities in `focusable_areas`, and then applies any
/// overrides from the manual edge map resource.
pub(crate) fn rebuild_nav_map(
    nav_map: &mut DirectionalNavigationMap,
    manual_edge_map: &DirectionalNavigationMap,
    focusable_areas: &[FocusableArea],
    config: &AutoNavigationConfig,
) {
    nav_map.clear();

    // Use the `auto_generate_navigation_edges` utility to generate the visualization
    // map. It will find the best candidate in each direction for each node in `focusable_areas`,
    // using the same configuration that it uses in the `AutoDirectionalNavigator`.
    auto_generate_navigation_edges(nav_map, focusable_areas, config);

    for (entity, neighbors) in manual_edge_map.neighbors.iter() {
        add_overrides_to_nav_viz_map(nav_map, entity, neighbors);
    }
}

/// Adds navigation override edges from `override_neighbors` for the given `entity` in `nav_map`.
fn add_overrides_to_nav_viz_map(
    nav_map: &mut DirectionalNavigationMap,
    entity: &Entity,
    override_neighbors: &NavNeighbors,
) {
    if let Some(existing_nav_neighbors) = nav_map.neighbors.get_mut(entity) {
        for (i, maybe_neighbor_override) in override_neighbors.neighbors.iter().enumerate() {
            if let Some(neighbor_override) = maybe_neighbor_override {
                existing_nav_neighbors.neighbors[i] = Some(*neighbor_override);
            }
        }
    }
}
