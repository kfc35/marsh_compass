use bevy::camera::ViewportConversionError;
use bevy::ecs::entity::EntityHashMap;
use bevy::input_focus::directional_navigation::FocusableArea;
use bevy::math::CompassOctant;
use bevy::prelude::*;

/// The data needed to determine where to draw a visualization
/// element within the window.
#[derive(Clone, Copy, PartialEq)]
pub struct NavVizPosData {
    /// The width and height of this entity's axis-aligned bounding box in
    /// the world. This does not hug tightly to the node.
    pub aabb_size: Vec2,
    /// Contains the rotation of this entity in world space and
    /// the translation of this entity's center from local space (at [`Vec2::ZERO`])
    /// to world space.
    pub transformation: Isometry2d,
    /// The width and height of this entity's oriented bounding box
    /// in world units. This hugs tightly to the node.
    pub obb_size: Vec2,
}

impl NavVizPosData {
    /// Returns the center of this entity in world coordinates.
    pub fn get_center(&self) -> Vec2 {
        self.transformation.translation
    }

    /// Returns the point on this entity in world coordinates that is
    /// most in the direction of `dir`.
    pub fn get_point_in_direction(&self, dir: CompassOctant) -> Vec2 {
        let world_center = self.get_center();
        let world_point_on_aabb = match dir {
            CompassOctant::North => world_center + Vec2::new(0., self.aabb_size.y / 2.),
            CompassOctant::NorthEast => world_center + (self.aabb_size / 2.),
            CompassOctant::East => world_center + Vec2::new(self.aabb_size.x / 2., 0.),
            CompassOctant::SouthEast => {
                world_center + (Vec2::new(self.aabb_size.x, -self.aabb_size.y) / 2.)
            }
            CompassOctant::South => world_center + Vec2::new(0., -self.aabb_size.y / 2.),
            CompassOctant::SouthWest => world_center - (self.aabb_size / 2.),
            CompassOctant::West => world_center + Vec2::new(-self.aabb_size.x / 2., 0.),
            CompassOctant::NorthWest => {
                world_center + (Vec2::new(-self.aabb_size.x, self.aabb_size.y) / 2.)
            }
        };

        // Clamp the aabb point onto the well-fitting obb.
        let local_point_on_aabb = self.world_to_local(world_point_on_aabb);
        let local_point_on_obb = local_point_on_aabb.clamp(-self.obb_size / 2., self.obb_size / 2.);

        // Transform the local point back to world coordinates.
        self.local_to_world(local_point_on_obb)
    }

    /// Transforms the point in this entity's local coordinates into world coordinates.
    pub fn local_to_world(&self, local_point: Vec2) -> Vec2 {
        self.transformation * local_point
    }

    /// Transforms the point in world coordinate's into this entity's local coordinates
    pub fn world_to_local(&self, world_point: Vec2) -> Vec2 {
        self.transformation.inverse() * world_point
    }

    /// Applies a translation written in this entity's local coordinates to a world coordinate.
    /// Returns the translated point in world_coordinates.
    pub fn apply_local_translation(&self, world_point: Vec2, local_nudge: Vec2) -> Vec2 {
        self.local_to_world(self.world_to_local(world_point) + local_nudge)
    }
}

/// Rebuilds the `entity_viz_data` field in [`NavVizMap`](crate::nav_viz_map::NavVizMap).
///
/// It clears the map and adds a new entry for every `auto_nav_ui_focusable_areas`.
/// The position data in the `FocusableArea` is converted from UI logical coords to
/// 2D world-space coords. Size data does not need to be converted.
pub(crate) fn rebuild_entity_viz_pos_data(
    viz_pos_data: &mut EntityHashMap<NavVizPosData>,
    auto_nav_ui_focusable_areas: &[FocusableArea],
    viewport_to_world_2d: &impl Fn(Vec2) -> Result<Vec2, ViewportConversionError>,
    position_data_query: Query<(&ComputedNode, &UiGlobalTransform)>,
) {
    viz_pos_data.clear();
    viz_pos_data.shrink_to(auto_nav_ui_focusable_areas.len());

    for focusable_area in auto_nav_ui_focusable_areas {
        let Some((rotation, obb_size)) =
            get_rotation_and_obb_size(focusable_area.entity, &position_data_query)
        else {
            continue;
        };
        let Ok(world_center) = viewport_to_world_2d(focusable_area.position) else {
            continue;
        };
        let transformation = Isometry2d::new(world_center, rotation);

        viz_pos_data.insert(
            focusable_area.entity,
            NavVizPosData {
                aabb_size: focusable_area.size,
                transformation,
                obb_size,
            },
        );
    }
}

/// This is somewhat redundant with `entity_to_camera_and_focusable_area`
/// Possible todo: merge the two so we are not querying twice.
fn get_rotation_and_obb_size(
    entity: Entity,
    position_data_query: &Query<(&ComputedNode, &UiGlobalTransform)>,
) -> Option<(Rot2, Vec2)> {
    position_data_query
        .get(entity)
        .map_or(None, |(computed, transform)| {
            let (scale, angle, _) = transform.to_scale_angle_translation();
            let obb_size = computed.size() * computed.inverse_scale_factor() * scale;
            // This is -angle because UiTransform defines rotations clockwise.
            let rotation = Rot2::radians(-angle);
            Some((rotation, obb_size))
        })
}

#[cfg(test)]
mod tests {
    use std::f32::consts::FRAC_PI_2;

    use super::*;

    fn assert_eq_vec2(left: Vec2, right: Vec2) {
        let difference = left - right;
        assert!(difference.x.abs() <= 1e-6);
        assert!(difference.y.abs() <= 1e-6);
    }

    #[test]
    fn test_nav_viz_pos_data() {
        let data = NavVizPosData {
            aabb_size: Vec2::new(30., 20.),
            transformation: Isometry2d::new(Vec2::new(3., 7.), Rot2::radians(FRAC_PI_2)),
            obb_size: Vec2::new(20., 30.),
        };

        assert_eq_vec2(data.get_center(), Vec2::new(3., 7.));
        assert_eq_vec2(
            data.get_point_in_direction(CompassOctant::North),
            Vec2::new(3., 17.),
        );
        assert_eq_vec2(
            data.get_point_in_direction(CompassOctant::NorthEast),
            Vec2::new(18., 17.),
        );
        assert_eq_vec2(
            data.get_point_in_direction(CompassOctant::East),
            Vec2::new(18., 7.),
        );
        assert_eq_vec2(
            data.get_point_in_direction(CompassOctant::SouthEast),
            Vec2::new(18., -3.),
        );
        assert_eq_vec2(
            data.get_point_in_direction(CompassOctant::South),
            Vec2::new(3., -3.),
        );
        assert_eq_vec2(
            data.get_point_in_direction(CompassOctant::SouthWest),
            Vec2::new(-12., -3.),
        );
        assert_eq_vec2(
            data.get_point_in_direction(CompassOctant::West),
            Vec2::new(-12., 7.),
        );
        assert_eq_vec2(
            data.get_point_in_direction(CompassOctant::NorthWest),
            Vec2::new(-12., 17.),
        );

        assert_eq_vec2(
            data.apply_local_translation(Vec2::new(18., 7.), Vec2::new(3., 0.)),
            Vec2::new(18., 10.),
        );
        assert_eq_vec2(
            data.apply_local_translation(Vec2::new(18., 7.), Vec2::new(0., 7.)),
            Vec2::new(11., 7.),
        );
    }
}
