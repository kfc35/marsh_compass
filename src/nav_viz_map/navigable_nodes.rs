#[doc(hidden)]
// The three functions in this module,
// [`get_navigable_nodes`], [`entity_to_camera_and_focusable_area`], and [`get_rotated_bounds`]
// were taken from the Bevy codebase for ease of use since they are currently private there.
// They are used to fetch and convert UI nodes into [`FocusableArea`]s. The lifetimes of the
// query fields are the only things that were modified.
// Possible todo: Make a PR in Bevy to make these pub and maybe put them in an easily accessible place
// outside of the SystemParam if it makes sense.
use bevy::input_focus::directional_navigation::FocusableArea;
use bevy::prelude::*;
use bevy::ui::auto_directional_navigation::AutoDirectionalNavigation;

/// Returns a vec of [`FocusableArea`] representing nodes that are eligible to be automatically navigated to.
/// The camera of any navigable nodes will equal the desired `target_camera`.
pub(crate) fn get_navigable_nodes(
    target_camera: Entity,
    navigable_entities_query: Query<
        (
            Entity,
            &ComputedUiTargetCamera,
            &ComputedNode,
            &UiGlobalTransform,
            &InheritedVisibility,
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
pub(crate) fn entity_to_camera_and_focusable_area(
    entity: Entity,
    camera_and_focusable_area_query: Query<
        (
            Entity,
            &ComputedUiTargetCamera,
            &ComputedNode,
            &UiGlobalTransform,
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
