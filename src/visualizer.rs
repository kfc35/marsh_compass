use bevy::input_focus::{InputFocus, directional_navigation::NavNeighbors};
use bevy::math::CompassOctant;
use bevy::prelude::*;

use crate::{AutoNavVizGizmoConfigGroup, AutoNavVizMode, NavVizMap};

/// The system that draws the visualizations of the auto navigation
/// system. It uses gizmos to draw arrows between entities.
pub fn draw_viz_for_current_focus(
    mut gizmos: Gizmos<AutoNavVizGizmoConfigGroup>,
    nav_viz_map: Res<NavVizMap>,
    input_focus: Res<InputFocus>,
    config: Res<GizmoConfigStore>,
) {
    let entities_to_draw_nav = match config.config::<AutoNavVizGizmoConfigGroup>().1.drawing_mode {
        AutoNavVizMode::EnabledForAll => nav_viz_map
            .map
            .neighbors
            .iter()
            .collect::<Vec<(&Entity, &NavNeighbors)>>(),
        AutoNavVizMode::EnabledForCurrentFocus => {
            if let Some(entity) = &input_focus.0
                && let Some(neighbors) = nav_viz_map.map.get_neighbors(*entity)
            {
                vec![(entity, neighbors)]
            } else {
                return;
            }
        }
        AutoNavVizMode::Disabled => return,
    };

    for (entity, neighbors) in entities_to_draw_nav.into_iter() {
        for (i, maybe_neighbor) in neighbors.neighbors.iter().enumerate() {
            let Some(neighbor) = maybe_neighbor else {
                continue;
            };

            let Some((entity_pos, entity_size)) = nav_viz_map
                .entity_viz_data
                .get(entity)
                .map(|fa| (fa.world_position, fa.size))
            else {
                continue;
            };
            let Some(direction) = CompassOctant::from_index(i) else {
                continue;
            };
            let shifted_entity_pos = shift_position(entity_pos, entity_size, direction);

            let Some((neighbor_pos, neighbor_size)) = nav_viz_map
                .entity_viz_data
                .get(neighbor)
                .map(|fa| (fa.world_position, fa.size))
            else {
                continue;
            };
            let shifted_neighbor_pos =
                shift_position(neighbor_pos, neighbor_size, direction.opposite());

            gizmos.arrow_2d(
                shifted_entity_pos,
                shifted_neighbor_pos,
                Color::Srgba(Srgba::RED),
            );
        }
    }
}

fn shift_position(pos: Vec2, size: Vec2, dir: CompassOctant) -> Vec2 {
    match dir {
        CompassOctant::North => pos + Vec2::new(0., size.y / 2.),
        CompassOctant::NorthEast => pos + (size / 2.),
        CompassOctant::East => pos + Vec2::new(size.x / 2., 0.),
        CompassOctant::SouthEast => pos + (Vec2::new(size.x, -size.y) / 2.),
        CompassOctant::South => pos + Vec2::new(0., -size.y / 2.),
        CompassOctant::SouthWest => pos - (size / 2.),
        CompassOctant::West => pos + Vec2::new(-size.x / 2., 0.),
        CompassOctant::NorthWest => pos + (Vec2::new(-size.x, size.y) / 2.),
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
