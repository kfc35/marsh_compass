use bevy::camera::ViewportConversionError;
use bevy::ecs::entity::EntityHashMap;
use bevy::input_focus::directional_navigation::FocusableArea;
use bevy::math::CompassOctant;
use bevy::prelude::*;

/// The data needed to determine where to draw a visualization
/// element within the window.
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
        let Ok(world_center) = viewport_to_world_2d(focusable_area.position) else {
            continue;
        };
        let Some((rotation, obb_size)) =
            get_rotation_and_obb_size(focusable_area.entity, &position_data_query)
        else {
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

fn get_rotation_and_obb_size(
    entity: Entity,
    position_data_query: &Query<(&ComputedNode, &UiGlobalTransform)>,
) -> Option<(Rot2, Vec2)> {
    position_data_query
        .get(entity)
        .map_or(None, |(computed, transform)| {
            let (scale, angle, _) = transform.to_scale_angle_translation();
            let obb_size = computed.size() * computed.inverse_scale_factor() * scale;
            let rotation = Rot2::radians(angle);
            Some((rotation, obb_size))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    use bevy::input_focus::directional_navigation::FocusableArea;

    #[test]
    fn test_rebuild_entity_viz_data() {
        // let mut entity_viz_pos_data = EntityHashMap::new();
        // let e1 = Entity::from_raw_u32(1).unwrap();
        // let e2 = Entity::from_raw_u32(2).unwrap();
        // let e3 = Entity::from_raw_u32(3).unwrap();
        // // Old entries should be cleared / overwritten.
        // entity_viz_pos_data.insert(
        //     e1,
        //     NavVizPosData {
        //         world_position: Vec2::new(99., 234.),
        //         size: Vec2::new(33., 28.),
        //     },
        // );
        // let focusable_areas = vec![
        //     FocusableArea {
        //         entity: e1,
        //         position: Vec2::new(50., 100.),
        //         size: Vec2::new(25., 30.),
        //     },
        //     FocusableArea {
        //         entity: e2,
        //         position: Vec2::new(575., 1080.),
        //         size: Vec2::new(2., 5.),
        //     },
        //     FocusableArea {
        //         entity: e3,
        //         position: Vec2::new(500., 1000.),
        //         size: Vec2::new(1., 100.),
        //     },
        // ];

        // let viewport_to_world_2d = |viewport_position: Vec2| {
        //     if viewport_position.x > 500. {
        //         Err(ViewportConversionError::InvalidData)
        //     } else {
        //         Ok(Vec2::new(
        //             viewport_position.x - 500.,
        //             1000. - viewport_position.y,
        //         ))
        //     }
        // };

        // rebuild_entity_viz_pos_data(
        //     &mut entity_viz_pos_data,
        //     &focusable_areas,
        //     &viewport_to_world_2d,
        // );

        // assert_eq!(entity_viz_pos_data.len(), 2);
        // let viz_data = entity_viz_pos_data.get(&e1).unwrap();
        // assert_eq!(viz_data.world_position, Vec2::new(-450., 900.));
        // assert_eq!(viz_data.size, Vec2::new(25., 30.));
        // let viz_data = entity_viz_pos_data.get(&e3).unwrap();
        // assert_eq!(viz_data.world_position, Vec2::new(0., 0.));
        // assert_eq!(viz_data.size, Vec2::new(1., 100.));
    }
}
