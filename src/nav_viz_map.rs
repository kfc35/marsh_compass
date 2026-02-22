use bevy::ecs::entity::EntityHashMap;
use bevy::ecs::query::QueryEntityError;
use bevy::input_focus::InputFocus;
use bevy::input_focus::directional_navigation::{
    AutoNavigationConfig, DirectionalNavigationMap, FocusableArea, NavNeighbors,
    auto_generate_navigation_edges,
};
use bevy::prelude::*;
use bevy::ui::auto_directional_navigation::AutoDirectionalNavigation;

/// Resource used to cache the complete navigation map: auto navigation edges combined
/// with overridden manual edges. It also includes position and size data of each
/// entity in a separate map for visualization purposes.
#[derive(Resource, Default)]
pub struct NavVizMap {
    /// This map contains the navigation edges that would be taken by the
    /// [`AutoDirectionalNavigator`](bevy::ui::auto_directional_navigation::AutoDirectionalNavigator)
    /// with overridden manual edges. This map is scoped to only include entities that are either:
    ///
    /// - under the same camera as the current [`InputFocus`](bevy::input_focus::InputFocus) or
    ///
    /// - manually defined in the existing [`DirectionalNavigationMap`] resource.
    ///
    /// Despite it having the same type, this is not to be confused with the
    /// [`DirectionalNavigationMap`] resource, which is a resource that contains only manually
    /// defined navigation edges.
    pub map: DirectionalNavigationMap,

    /// A cache map that stores an entity's FocusableArea (position and size).
    /// The information is used when drawing the navigation edges.
    pub entity_viz_data: EntityHashMap<NavVizData>,
}

/// A System that rebuilds the [`NavVizMap`] resource with the
/// [`AutoDirectionalNavigation`] entities that share the camera with the current [`InputFocus`]
/// and any manual edges defined in the [`DirectionalNavigationMap`] resource.
pub fn rebuild_nav_viz_map(
    mut nav_viz_map: ResMut<NavVizMap>,
    manual_edge_map: Res<DirectionalNavigationMap>,
    current_focus: Res<InputFocus>,
    config: Res<AutoNavigationConfig>,
    navigable_entities_query: Query<
        (
            Entity,
            &'static ComputedUiTargetCamera,
            &'static ComputedNode,
            &'static UiGlobalTransform,
            &'static InheritedVisibility,
        ),
        With<AutoDirectionalNavigation>,
    >,
    camera_and_focusable_area_query: Query<
        (
            Entity,
            &'static ComputedUiTargetCamera,
            &'static ComputedNode,
            &'static UiGlobalTransform,
        ),
        With<AutoDirectionalNavigation>,
    >,
    viewport_logical_size_query: Query<
        &'static ComputedUiRenderTargetInfo,
        With<AutoDirectionalNavigation>,
    >,
) {
    // Get the focusable areas related to the current focus and the entities
    // it shares the camera with. This is what the `AutoDirectionalNavigator`
    // does under the hood
    let Some(focus) = current_focus.get() else {
        return;
    };
    // The viewport logical size is needed to figure out where to draw the viz.
    let Ok(logical_size) = viewport_logical_size(focus, viewport_logical_size_query) else {
        return;
    };
    let Some((camera, current_focusable_area)) =
        entity_to_camera_and_focusable_area(focus, camera_and_focusable_area_query)
    else {
        return;
    };
    let mut focusable_areas = get_navigable_nodes(camera, navigable_entities_query);
    focusable_areas.push(current_focusable_area);

    // Use the `auto_generate_navigation_edges` utility to generate the visualization
    // map. It will find the best candidate in each direction for each node in `focusable_areas`,
    // using the same configuration that it uses in the `AutoDirectionalNavigator`.
    nav_viz_map.map.clear();
    auto_generate_navigation_edges(&mut nav_viz_map.map, &focusable_areas, &config);

    for (entity, neighbors) in manual_edge_map.neighbors.iter() {
        add_overrides_to_nav_viz_map(&mut nav_viz_map.map, entity, neighbors);
    }

    // Add position and size data to the nav_viz_map
    nav_viz_map.entity_viz_data.clear();
    for focusable_area in focusable_areas {
        nav_viz_map.entity_viz_data.insert(
            focusable_area.entity,
            to_nav_viz_data(focusable_area, logical_size),
        );
    }
}

fn add_overrides_to_nav_viz_map(
    nav_viz_map: &mut DirectionalNavigationMap,
    entity: &Entity,
    override_neighbors: &NavNeighbors,
) {
    if let Some(existing_nav_neighbors) = nav_viz_map.neighbors.get_mut(entity) {
        for (i, maybe_neighbor_override) in override_neighbors.neighbors.iter().enumerate() {
            if let Some(neighbor_override) = maybe_neighbor_override {
                existing_nav_neighbors.neighbors[i] = Some(*neighbor_override);
            }
        }
    }
}

/// The position and size data needed to figure out what to draw in the window.
pub struct NavVizData {
    pub world_position: Vec2,
    pub size: Vec2,
}

/// UI logical coordinates use a different coordinate system from the viz system (gizmos).
/// UI coordinates are oriented with the origin at the top left of the window.
/// X-coordinates increase rightward and Y-coordinates increase downward.
/// Gizmos, on the other hand, require a coordinate where
/// the center of the window is (0, 0), commonly known as world-space coordinates.
/// This function converts a UI logical coordinate position to its
/// equivalent in 2d world-space coordinates.
pub fn ui_to_viz_position(ui_logical_coords: Vec2, viewport_logical_size: Vec2) -> Vec2 {
    let viewport_origin = viewport_logical_size / Vec2::splat(2.);
    Vec2::new(
        ui_logical_coords.x - viewport_origin.x,
        viewport_origin.y - ui_logical_coords.y,
    )
}

/// Converts a [`FocusableArea`] into a [`NavVizData`]
fn to_nav_viz_data(focusable_area: FocusableArea, viewport_logical_size: Vec2) -> NavVizData {
    NavVizData {
        world_position: ui_to_viz_position(focusable_area.position, viewport_logical_size),
        size: focusable_area.size,
    }
}

/// Get the render target's viewport size in logical pixels.
/// This is used to convert from UI logical coordinates to world coordinates.
fn viewport_logical_size(
    entity: Entity,
    viewport_logical_size_query: Query<
        &'static ComputedUiRenderTargetInfo,
        With<AutoDirectionalNavigation>,
    >,
) -> Result<Vec2, QueryEntityError> {
    viewport_logical_size_query
        .get(entity)
        .map(|render_target_info| render_target_info.logical_size())
}

// The three functions below this comment,
// [get_navigable_nodes], [entity_to_camera_and_focusable_area], and [get_rotated_bounds]
// were taken from the Bevy codebase for ease of use since they are currently private there.
// They are used to fetch and convert UI nodes into `FocusableArea`s. They have not been modified.
// Possible todo: Make a PR in Bevy to make these pub and maybe put them in an easily accessible place
// outside of the SystemParam if it makes sense.

/// Returns a vec of [`FocusableArea`] representing nodes that are eligible to be automatically navigated to.
/// The camera of any navigable nodes will equal the desired `target_camera`.
fn get_navigable_nodes(
    target_camera: Entity,
    navigable_entities_query: Query<
        (
            Entity,
            &'static ComputedUiTargetCamera,
            &'static ComputedNode,
            &'static UiGlobalTransform,
            &'static InheritedVisibility,
        ),
        With<AutoDirectionalNavigation>,
    >,
) -> Vec<FocusableArea> {
    navigable_entities_query
        .iter()
        .filter_map(
            |(entity, computed_target_camera, computed, transform, inherited_visibility)| {
                // Skip hidden or zero-size nodes
                if computed.is_empty() || !inherited_visibility.get() {
                    return None;
                }
                // Accept nodes that have the same target camera as the desired target camera
                if let Some(tc) = computed_target_camera.get()
                    && tc == target_camera
                {
                    let (scale, rotation, translation) = transform.to_scale_angle_translation();
                    let scaled_size = computed.size() * computed.inverse_scale_factor() * scale;
                    let rotated_size = get_rotated_bounds(scaled_size, rotation);
                    Some(FocusableArea {
                        entity,
                        position: translation * computed.inverse_scale_factor(),
                        size: rotated_size,
                    })
                } else {
                    // The node either does not have a target camera or it is not the same as the desired one.
                    None
                }
            },
        )
        .collect()
}

/// Gets the target camera and the [`FocusableArea`] of the provided entity, if it exists.
///
/// Returns None if there was a [`QueryEntityError`](bevy_ecs::query::QueryEntityError) or
/// if the entity does not have a target camera.
fn entity_to_camera_and_focusable_area(
    entity: Entity,
    camera_and_focusable_area_query: Query<
        (
            Entity,
            &'static ComputedUiTargetCamera,
            &'static ComputedNode,
            &'static UiGlobalTransform,
        ),
        With<AutoDirectionalNavigation>,
    >,
) -> Option<(Entity, FocusableArea)> {
    camera_and_focusable_area_query.get(entity).map_or(
        None,
        |(entity, computed_target_camera, computed, transform)| {
            if let Some(target_camera) = computed_target_camera.get() {
                let (scale, rotation, translation) = transform.to_scale_angle_translation();
                let scaled_size = computed.size() * computed.inverse_scale_factor() * scale;
                let rotated_size = get_rotated_bounds(scaled_size, rotation);
                Some((
                    target_camera,
                    FocusableArea {
                        entity,
                        position: translation * computed.inverse_scale_factor(),
                        size: rotated_size,
                    },
                ))
            } else {
                None
            }
        },
    )
}

/// Util used to get the resulting bounds of a UI entity after applying its rotation.
///
/// This is necessary to apply because navigation should only use the final screen position
/// of an entity in automatic navigation calculations. These bounds are used as the entity's size in
/// [`FocusableArea`].
fn get_rotated_bounds(size: Vec2, rotation: f32) -> Vec2 {
    if rotation == 0.0 {
        return size;
    }
    let cos_r = ops::cos(rotation).abs();
    let sin_r = ops::sin(rotation).abs();
    Vec2::new(
        size.x * cos_r + size.y * sin_r,
        size.x * sin_r + size.y * cos_r,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
