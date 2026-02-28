use bevy::camera::ViewportConversionError;
use bevy::ecs::entity::EntityHashMap;
use bevy::input_focus::directional_navigation::FocusableArea;
use bevy::prelude::*;

/// The position and size data needed to determine where to draw a visualization
/// element within the window.
pub struct NavVizPosData {
    pub world_position: Vec2,
    pub size: Vec2,
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
) {
    viz_pos_data.clear();
    viz_pos_data.shrink_to(auto_nav_ui_focusable_areas.len());

    for focusable_area in auto_nav_ui_focusable_areas {
        let Ok(world_position) = viewport_to_world_2d(focusable_area.position) else {
            continue;
        };

        viz_pos_data.insert(
            focusable_area.entity,
            NavVizPosData {
                world_position,
                size: focusable_area.size,
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use bevy::input_focus::directional_navigation::FocusableArea;

    #[test]
    fn test_rebuild_entity_viz_data() {
        let mut entity_viz_pos_data = EntityHashMap::new();
        let e1 = Entity::from_raw_u32(1).unwrap();
        let e2 = Entity::from_raw_u32(2).unwrap();
        let e3 = Entity::from_raw_u32(3).unwrap();
        // Old entries should be cleared / overwritten.
        entity_viz_pos_data.insert(
            e1,
            NavVizPosData {
                world_position: Vec2::new(99., 234.),
                size: Vec2::new(33., 28.),
            },
        );
        let focusable_areas = vec![
            FocusableArea {
                entity: e1,
                position: Vec2::new(50., 100.),
                size: Vec2::new(25., 30.),
            },
            FocusableArea {
                entity: e2,
                position: Vec2::new(575., 1080.),
                size: Vec2::new(2., 5.),
            },
            FocusableArea {
                entity: e3,
                position: Vec2::new(500., 1000.),
                size: Vec2::new(1., 100.),
            },
        ];

        let viewport_to_world_2d = |viewport_position: Vec2| {
            if viewport_position.x > 500. {
                Err(ViewportConversionError::InvalidData)
            } else {
                Ok(Vec2::new(
                    viewport_position.x - 500.,
                    1000. - viewport_position.y,
                ))
            }
        };

        rebuild_entity_viz_pos_data(
            &mut entity_viz_pos_data,
            &focusable_areas,
            &viewport_to_world_2d,
        );

        assert_eq!(entity_viz_pos_data.len(), 2);
        let viz_data = entity_viz_pos_data.get(&e1).unwrap();
        assert_eq!(viz_data.world_position, Vec2::new(-450., 900.));
        assert_eq!(viz_data.size, Vec2::new(25., 30.));
        let viz_data = entity_viz_pos_data.get(&e3).unwrap();
        assert_eq!(viz_data.world_position, Vec2::new(0., 0.));
        assert_eq!(viz_data.size, Vec2::new(1., 100.));
    }
}
