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

    // Right now, this loop is double drawing symmetric edges instead of utilizing
    // double headed arrows.
    // Unfortunately, when trying to implement double headed arrows, interpolating
    // colors of opposite directions mix to become gray with their default values.
    // The default values were picked to be opposites on the color wheel after all.
    // If we want to have a double headed arrow, we should allow a way to gradient
    // the color of the double headed arrow for gizmos so it still looks pretty.
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
                // A future case to handle appropriately, perhaps with a white arrow
                // to indicate that it goes elsewhere.
                // A text gizmo could also be used here to indicate where it goes
                // (available in Bevy 0.19)
                continue;
            };

            let is_symmetrical =
                nav_viz_map.map.get_neighbor(*neighbor, dir.opposite()) == Some(*entity);

            let (start, end) = get_arrow_endpoints(
                from_pos,
                from_size,
                dir,
                to_pos,
                to_size,
                is_symmetrical,
                config,
            );
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
    is_symmetrical: bool,
    config: &AutoNavVizGizmoConfigGroup,
) -> (Vec2, Vec2) {
    let mut start = get_position_in_direction(from_pos, from_size, dir);
    let mut end = get_closest_point(to_pos, to_size, start);
    // TODO handle the case when !dir.is_in_direction(start, end)
    // This means that it is a wrapping end. Need to do some arcing

    if is_symmetrical && config.drawing_mode == AutoNavVizDrawMode::EnabledForAll {
        let nudge = config.symmetrical_edge_spacing / 2.;
        match dir {
            CompassOctant::North => {
                // Nudge West
                start -= Vec2::new(nudge, 0.);
                end -= Vec2::new(nudge, 0.);
            }
            CompassOctant::South => {
                // Nudge East
                start += Vec2::new(nudge, 0.);
                end += Vec2::new(nudge, 0.);
            }
            CompassOctant::East => {
                // Nudge North
                start += Vec2::new(0., nudge);
                end += Vec2::new(0., nudge);
            }
            CompassOctant::West => {
                // Nudge South
                start -= Vec2::new(0., nudge);
                end -= Vec2::new(0., nudge);
            }
            CompassOctant::NorthEast => {
                // Nudge East
                start -= Vec2::new(nudge, 0.);
                // Nudge North
                end += Vec2::new(0., nudge);
            }
            CompassOctant::SouthWest => {
                // Nudge West
                start += Vec2::new(nudge, 0.);
                // Nudge South
                end -= Vec2::new(0., nudge);
            }
            CompassOctant::NorthWest => {
                // Nudge South
                start -= Vec2::new(0., nudge);
                // Nudge East
                end -= Vec2::new(nudge, 0.);
            }
            CompassOctant::SouthEast => {
                // Nudge North
                start += Vec2::new(0., nudge);
                // Nudge West
                end += Vec2::new(nudge, 0.);
            }
        }
    }
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
    fn test_arrow_endpoints_for_single_entity_in_dir_asymmetrical() {
        let config = AutoNavVizGizmoConfigGroup::default();
        let from_pos = Vec2::new(0., 0.);
        let from_size = Vec2::new(10., 20.);
        let to_pos = Vec2::new(20., 0.);
        let to_size = Vec2::new(8., 12.);

        let (start, end) = get_arrow_endpoints(
            from_pos,
            from_size,
            CompassOctant::East,
            to_pos,
            to_size,
            false,
            &config,
        );

        assert_eq!(start, Vec2::new(5., 0.));
        assert_eq!(end, Vec2::new(16., 0.));

        let (start, end) = get_arrow_endpoints(
            from_pos,
            from_size,
            CompassOctant::NorthEast,
            to_pos,
            to_size,
            false,
            &config,
        );

        assert_eq!(start, Vec2::new(5., 10.));
        assert_eq!(end, Vec2::new(16., 6.));

        let (start, end) = get_arrow_endpoints(
            from_pos,
            from_size,
            CompassOctant::SouthEast,
            to_pos,
            to_size,
            false,
            &config,
        );

        assert_eq!(start, Vec2::new(5., -10.));
        assert_eq!(end, Vec2::new(16., -6.));
    }
}
