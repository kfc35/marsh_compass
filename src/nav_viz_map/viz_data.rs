use bevy::ecs::{entity::EntityHashMap, query::QueryEntityError};
use bevy::input_focus::directional_navigation::FocusableArea;
use bevy::prelude::*;
use bevy::ui::auto_directional_navigation::AutoDirectionalNavigation;

/// The position and size data needed to determine where to draw a visualization
/// element within the window.
pub struct NavVizData {
    pub world_position: Vec2,
    pub size: Vec2,
}

/// Rebuilds the `entity_viz_data` field in [`NavVizMap`](crate::nav_viz_map::NavVizMap).
///
/// It clears the map and adds a new entry for every `auto_nav_ui_focusable_areas`.
/// The position data in the `FocusableArea` is converted from UI logical coords to
/// 2D world-space coords. Size data does not need to be converted.
pub(crate) fn rebuild_entity_viz_data(
    viz_data: &mut EntityHashMap<NavVizData>,
    auto_nav_ui_focusable_areas: &[FocusableArea],
    viewport_logical_size: Vec2,
) {
    viz_data.clear();
    for focusable_area in auto_nav_ui_focusable_areas {
        viz_data.insert(
            focusable_area.entity,
            to_nav_viz_data(focusable_area, viewport_logical_size),
        );
    }
}

/// Get the render target's viewport size in logical pixels.
///
/// This value must be fetched in order to be able to convert UI logical
/// coordinates to 2D world-space coordinates.
pub(crate) fn viewport_logical_size(
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

/// Converts a [`FocusableArea`] into a [`NavVizData`]
fn to_nav_viz_data(focusable_area: &FocusableArea, viewport_logical_size: Vec2) -> NavVizData {
    NavVizData {
        world_position: ui_logical_to_world(focusable_area.position, viewport_logical_size),
        size: focusable_area.size,
    }
}

/// Converts UI logical coordinates to 2D world-space coordinates.
///
/// UI logical coordinates use a different coordinate system from the viz system (gizmos).
/// UI coordinates are oriented with the origin at the top left of the window.
/// X-coordinates increase rightward and Y-coordinates increase downward.
/// Gizmos, on the other hand, require a coordinate where
/// the center of the window is (0, 0), commonly known as world-space coordinates.
fn ui_logical_to_world(ui_logical_coords: Vec2, viewport_logical_size: Vec2) -> Vec2 {
    let viewport_origin = viewport_logical_size / Vec2::splat(2.);
    Vec2::new(
        ui_logical_coords.x - viewport_origin.x,
        viewport_origin.y - ui_logical_coords.y,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    use bevy::input_focus::directional_navigation::FocusableArea;

    #[test]
    fn test_rebuild_entity_viz_data() {
        let mut entity_viz_data = EntityHashMap::new();
        let e1 = Entity::from_raw_u32(1).unwrap();
        let e2 = Entity::from_raw_u32(2).unwrap();
        let e3 = Entity::from_raw_u32(3).unwrap();
        // Old entries should be cleared / overwritten.
        entity_viz_data.insert(
            e1,
            NavVizData {
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
        let viewport_logical_size = Vec2::new(1000., 2000.);

        rebuild_entity_viz_data(
            &mut entity_viz_data,
            &focusable_areas,
            viewport_logical_size,
        );

        assert_eq!(entity_viz_data.len(), 3);
        let viz_data = entity_viz_data.get(&e1).unwrap();
        assert_eq!(viz_data.world_position, Vec2::new(-450., 900.));
        assert_eq!(viz_data.size, Vec2::new(25., 30.));
        // Entity = 2
        let viz_data = entity_viz_data.get(&e2).unwrap();
        assert_eq!(viz_data.world_position, Vec2::new(75., -80.));
        assert_eq!(viz_data.size, Vec2::new(2., 5.));
        // Entity = 3
        let viz_data = entity_viz_data.get(&e3).unwrap();
        assert_eq!(viz_data.world_position, Vec2::new(0., 0.));
        assert_eq!(viz_data.size, Vec2::new(1., 100.));
    }
}
