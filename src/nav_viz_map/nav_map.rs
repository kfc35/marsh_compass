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

#[cfg(test)]
mod tests {
    use super::*;

    use bevy::math::CompassOctant;

    // Testing Util
    fn assert_neighbors(
        nav_map: &DirectionalNavigationMap,
        entity: Entity,
        expected_neighbors: [Option<Entity>; 8],
    ) {
        assert_eq!(
            nav_map.get_neighbor(entity, CompassOctant::North),
            expected_neighbors[CompassOctant::North.to_index()]
        );
        assert_eq!(
            nav_map.get_neighbor(entity, CompassOctant::NorthEast),
            expected_neighbors[CompassOctant::NorthEast.to_index()]
        );
        assert_eq!(
            nav_map.get_neighbor(entity, CompassOctant::East),
            expected_neighbors[CompassOctant::East.to_index()]
        );
        assert_eq!(
            nav_map.get_neighbor(entity, CompassOctant::SouthEast),
            expected_neighbors[CompassOctant::SouthEast.to_index()]
        );
        assert_eq!(
            nav_map.get_neighbor(entity, CompassOctant::South),
            expected_neighbors[CompassOctant::South.to_index()]
        );
        assert_eq!(
            nav_map.get_neighbor(entity, CompassOctant::SouthWest),
            expected_neighbors[CompassOctant::SouthWest.to_index()]
        );
        assert_eq!(
            nav_map.get_neighbor(entity, CompassOctant::West),
            expected_neighbors[CompassOctant::West.to_index()]
        );
        assert_eq!(
            nav_map.get_neighbor(entity, CompassOctant::NorthWest),
            expected_neighbors[CompassOctant::NorthWest.to_index()]
        );
    }

    #[test]
    fn test_rebuild_nav_map_only_auto() {
        let mut nav_map = DirectionalNavigationMap::default();
        let manual_map = DirectionalNavigationMap::default();
        let e1 = Entity::from_raw_u32(1).unwrap();
        let e2 = Entity::from_raw_u32(2).unwrap();
        let e3 = Entity::from_raw_u32(3).unwrap();
        let focusable_areas = vec![
            FocusableArea {
                entity: e1,
                position: Vec2::new(50., 100.),
                size: Vec2::new(25., 30.),
            },
            FocusableArea {
                entity: e2,
                position: Vec2::new(50., 200.),
                size: Vec2::new(2., 5.),
            },
            FocusableArea {
                entity: e3,
                position: Vec2::new(150., 100.),
                size: Vec2::new(1., 100.),
            },
        ];
        let config = AutoNavigationConfig::default();

        rebuild_nav_map(&mut nav_map, &manual_map, &focusable_areas, &config);

        assert_neighbors(
            &nav_map,
            e1,
            [
                None,
                Some(e3), // NE
                Some(e3), // E
                Some(e2), // SE
                Some(e2), // S
                Some(e2), // SW
                None,
                None,
            ],
        );

        assert_neighbors(
            &nav_map,
            e2,
            [
                Some(e1), // N
                Some(e3), // NE
                Some(e3), // E
                None,
                None,
                None,
                None,
                Some(e1), // NW
            ],
        );

        assert_neighbors(
            &nav_map,
            e3,
            [
                None,
                None,
                None,
                None,
                Some(e2), // S
                Some(e2), // SW
                Some(e1), // W
                Some(e1), // NW
            ],
        );
    }

    #[test]
    fn test_rebuild_nav_map_auto_and_manual() {
        let mut nav_map = DirectionalNavigationMap::default();
        let mut manual_map = DirectionalNavigationMap::default();
        let e1 = Entity::from_raw_u32(1).unwrap();
        let e2 = Entity::from_raw_u32(2).unwrap();
        let e3 = Entity::from_raw_u32(3).unwrap();
        let e4 = Entity::from_raw_u32(4).unwrap();
        manual_map.add_symmetrical_edge(e3, e1, CompassOctant::East);
        manual_map.add_edge(e2, e3, CompassOctant::North);
        manual_map.add_symmetrical_edge(e4, e1, CompassOctant::NorthEast);
        let focusable_areas = vec![
            FocusableArea {
                entity: e1,
                position: Vec2::new(50., 100.),
                size: Vec2::new(25., 30.),
            },
            FocusableArea {
                entity: e2,
                position: Vec2::new(50., 200.),
                size: Vec2::new(2., 5.),
            },
            FocusableArea {
                entity: e3,
                position: Vec2::new(150., 100.),
                size: Vec2::new(1., 100.),
            },
        ];
        let config = AutoNavigationConfig::default();

        rebuild_nav_map(&mut nav_map, &manual_map, &focusable_areas, &config);

        assert_neighbors(
            &nav_map,
            e1,
            [
                None,
                Some(e3), // NE
                Some(e3), // E
                Some(e2), // SE
                Some(e2), // S
                Some(e4), // SW overrides existing, e4 not in focusable_areas
                Some(e3), // W, override, new
                None,
            ],
        );

        assert_neighbors(
            &nav_map,
            e2,
            [
                Some(e3), // N, overrides existing
                Some(e3), // NE
                Some(e3), // E
                None,
                None,
                None,
                None,
                Some(e1), // NW
            ],
        );

        assert_neighbors(
            &nav_map,
            e3,
            [
                None,
                None,
                Some(e1), // E, override, new
                None,
                Some(e2), // S
                Some(e2), // SW
                Some(e1), // W
                Some(e1), // NW
            ],
        );

        // e4 is not in focusable_areas, so it does not get any entry in the map
        // however, it can appear as a neighbor for the other entries.
        assert_neighbors(
            &nav_map,
            e4,
            [None, None, None, None, None, None, None, None],
        );
    }
}
