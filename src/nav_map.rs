use bevy::prelude::*;
use bevy::input_focus::directional_navigation::{DirectionalNavigationMap, FocusableArea};
use bevy::ui::auto_directional_navigation::{AutoDirectionalNavigation};

/// Resource used to cache the current auto navigation edges with overridden
/// manual edges.
/// 
/// This is not to be confused with the [`DirectionalNavigationMap`] resource,
/// which is a resource that contains only manually defined navigation edges. 
/// This resource wraps a separate instance of the [`DirectionalNavigationMap`]
/// struct to take advantage of existing utilities.
/// 
/// This map contains the navigation edges that would be taken by the
/// [`AutoDirectionalNavigator`](bevy::ui::auto_directional_navigation::AutoDirectionalNavigator)
/// with overridden manual edges. This map is scoped to only include entities that are either:
/// 
/// - under the same camera as the current [`InputFocus`](bevy::input_focus::InputFocus) or
/// 
/// - manually defined in the existing [`DirectionalNavigationMap`] resource.
#[derive(Resource, Default, Deref, DerefMut)]
pub struct NavVizMap(DirectionalNavigationMap);

// We also need a resource that maps Entities to directions and points on the UI so that gizmos can be drawn.

impl NavVizMap {

  /// Rebuilds the navigation visualization map.
  pub fn rebuild(&mut self) {
    self.clear();

    // Only rebuild the map around the current input focus.
  }
}

/// The three functions below this comment,
/// [get_navigable_nodes], [entity_to_camera_and_focusable_area], and [get_rotated_bounds] 
/// were taken from the Bevy codebase for ease of use since they are currently private there.
/// Todo: Make a PR in Bevy to make these pub and maybe put them in an easily accessible place
/// outside of the SystemParam if it makes sense.

/// Returns a vec of [`FocusableArea`] representing nodes that are eligible to be automatically navigated to.
/// The camera of any navigable nodes will equal the desired `target_camera`.
fn get_navigable_nodes(target_camera: Entity, navigable_entities_query: Query<
    (
        Entity,
        &'static ComputedUiTargetCamera,
        &'static ComputedNode,
        &'static UiGlobalTransform,
        &'static InheritedVisibility,
    ),
    With<AutoDirectionalNavigation>,
>) -> Vec<FocusableArea> {
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
