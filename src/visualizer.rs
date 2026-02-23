use bevy::input_focus::{InputFocus, directional_navigation::NavNeighbors};
use bevy::math::CompassOctant;
use bevy::prelude::*;

use crate::{AutoNavVizColorMode, AutoNavVizDrawMode, AutoNavVizGizmoConfigGroup, NavVizMap};

/// The system that draws the visualizations of the auto navigation
/// system. It uses gizmos to draw arrows between entities.
pub fn draw_nav_viz(
    config_store: Res<GizmoConfigStore>,
    input_focus: Res<InputFocus>,
    nav_viz_map: Res<NavVizMap>,
    mut gizmos: Gizmos<AutoNavVizGizmoConfigGroup>,
) {
    let config = config_store.config::<AutoNavVizGizmoConfigGroup>().1;
    let entries_to_draw_nav = match config.drawing_mode {
        AutoNavVizDrawMode::EnabledForCurrentFocus => {
            if let Some(entity) = &input_focus.0
                && let Some(neighbors) = nav_viz_map.map.get_neighbors(*entity)
            {
                vec![(entity, neighbors)]
            } else {
                return;
            }
        }
        AutoNavVizDrawMode::EnabledForAll => nav_viz_map
            .map
            .neighbors
            .iter()
            .collect::<Vec<(&Entity, &NavNeighbors)>>(),
    };

    for (entity, neighbors) in entries_to_draw_nav.into_iter() {
        let entity_color = Oklcha::sequential_dispersed(entity.index_u32()).into();
        for (i, maybe_neighbor) in neighbors.neighbors.iter().enumerate() {
            let Some(neighbor) = maybe_neighbor else {
                continue;
            };
            let Some((from_pos, from_size)) = nav_viz_map
                .entity_viz_data
                .get(entity)
                .map(|fa| (fa.world_position, fa.size))
            else {
                continue;
            };
            let Some(dir) = CompassOctant::from_index(i) else {
                continue;
            };
            let Some((to_pos, to_size)) = nav_viz_map
                .entity_viz_data
                .get(neighbor)
                .map(|fa| (fa.world_position, fa.size))
            else {
                continue;
            };

            let (start, end) = get_arrow_endpoints(from_pos, from_size, dir, to_pos, to_size);
            let color = config
                .get_color_for_direction(dir)
                .map(|color| {
                    if let AutoNavVizColorMode::MixWithEntity(factor) = config.color_mode {
                        color.mix(&entity_color, factor)
                    } else {
                        color
                    }
                })
                .unwrap_or(entity_color);

            gizmos.arrow_2d(start, end, color);
        }
    }
}

fn get_arrow_endpoints(
    from_pos: Vec2,
    from_size: Vec2,
    dir: CompassOctant,
    to_pos: Vec2,
    to_size: Vec2,
) -> (Vec2, Vec2) {
    let start = get_position_in_direction(from_pos, from_size, dir);
    let end = get_closest_point(to_pos, to_size, start);
    (start, end)
}

/// Gets the point on the rectangle defined by its center `pos` and `size` that is
/// closest in distance squared to `point`
fn get_closest_point(pos: Vec2, size: Vec2, point: Vec2) -> Vec2 {
    let mut closest_point = get_position_in_direction(pos, size, CompassOctant::North);
    let mut squared_dist = closest_point.distance_squared(point);
    for dir in [
        CompassOctant::NorthEast,
        CompassOctant::East,
        CompassOctant::SouthEast,
        CompassOctant::South,
        CompassOctant::SouthWest,
        CompassOctant::West,
        CompassOctant::NorthWest,
    ] {
        let candidate = get_position_in_direction(pos, size, dir);
        let candidate_dist = candidate.distance_squared(point);
        if candidate_dist < squared_dist {
            closest_point = candidate;
            squared_dist = candidate_dist;
        }
    }
    closest_point
}

/// Gets the point on the rectangle defined by its center `pos` and `size` that is in the direction of `dir`.
fn get_position_in_direction(pos: Vec2, size: Vec2, dir: CompassOctant) -> Vec2 {
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
