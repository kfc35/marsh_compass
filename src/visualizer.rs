use core::f32::consts::{FRAC_PI_2, PI};

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
                .entity_viz_pos_data
                .get(entity)
                .map(|fa| (fa.world_position, fa.size))
            else {
                continue;
            };
            let Some(dir) = CompassOctant::from_index(i) else {
                continue;
            };
            let Some((to_pos, to_size)) = nav_viz_map
                .entity_viz_pos_data
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

            let nav_viz_draw_data = get_nav_viz_draw_data(
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
            if let Some((iso, radius)) = nav_viz_draw_data.maybe_arc {
                gizmos.arc_2d(iso, PI, radius, color);
            }
            gizmos
                .arrow_2d(
                    nav_viz_draw_data.arrow_start,
                    nav_viz_draw_data.arrow_end,
                    color,
                )
                .with_tip_length(config.arrow_tip_length);
        }
    }
}

fn get_nav_viz_draw_data(
    from_pos: Vec2,
    from_size: Vec2,
    dir: CompassOctant,
    to_pos: Vec2,
    to_size: Vec2,
    is_symmetrical: bool,
    config: &AutoNavVizGizmoConfigGroup,
) -> NavVizDrawData {
    let mut start = get_position_in_direction(from_pos, from_size, dir);
    let mut end = get_closest_point(to_pos, to_size, start);

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
                // Nudge West
                start -= Vec2::new(nudge, 0.);
                // Nudge North
                end += Vec2::new(0., nudge);
            }
            CompassOctant::SouthWest => {
                // Nudge East
                start += Vec2::new(nudge, 0.);
                // Nudge South
                end -= Vec2::new(0., nudge);
            }
            CompassOctant::NorthWest => {
                // Nudge South
                start -= Vec2::new(0., nudge);
                // Nudge West
                end -= Vec2::new(nudge, 0.);
            }
            CompassOctant::SouthEast => {
                // Nudge North
                start += Vec2::new(0., nudge);
                // Nudge East
                end += Vec2::new(nudge, 0.);
            }
        }
    }

    let maybe_arc = calculate_arc(start, from_size, end, dir);
    if let Some((iso, radius, new_arrow_start)) = maybe_arc {
        NavVizDrawData {
            maybe_arc: Some((iso, radius)),
            arrow_start: new_arrow_start,
            arrow_end: end,
        }
    } else {
        NavVizDrawData::new(start, end)
    }
}

/// If the end does not lie in the start's direction, a 180 degree gizmo arc must
/// be made. If applicable, this function returns the arc's isometry and radius,
/// alongside the new arrow_start to compensate for the additional arc.
fn calculate_arc(
    start: Vec2,
    from_size: Vec2,
    end: Vec2,
    dir: CompassOctant,
) -> Option<(Isometry2d, f32, Vec2)> {
    let nudge = from_size / 8.;
    if !dir.is_in_direction(start, end) {
        match dir {
            CompassOctant::North => Some((
                Isometry2d {
                    rotation: Rot2::radians(PI + FRAC_PI_2),
                    translation: Vec2::new(start.x - nudge.x, start.y),
                },
                nudge.x,
                Vec2::new(start.x - 2. * nudge.x, start.y),
            )),
            CompassOctant::NorthEast => Some((
                Isometry2d {
                    rotation: Rot2::radians(PI + FRAC_PI_2),
                    translation: Vec2::new(start.x - nudge.x, start.y),
                },
                nudge.x,
                Vec2::new(start.x - 2. * nudge.x, start.y),
            )),
            CompassOctant::East => Some((
                Isometry2d {
                    rotation: Rot2::PI,
                    translation: Vec2::new(start.x, start.y + nudge.y),
                },
                nudge.y,
                Vec2::new(start.x, start.y + 2. * nudge.y),
            )),
            CompassOctant::SouthEast => Some((
                Isometry2d {
                    rotation: Rot2::radians(PI),
                    translation: Vec2::new(start.x, start.y + nudge.y),
                },
                nudge.y,
                Vec2::new(start.x, start.y + 2. * nudge.y),
            )),
            CompassOctant::South => Some((
                Isometry2d {
                    rotation: Rot2::FRAC_PI_2,
                    translation: Vec2::new(start.x + nudge.x, start.y),
                },
                nudge.x,
                Vec2::new(start.x + 2. * nudge.x, start.y),
            )),
            CompassOctant::SouthWest => Some((
                Isometry2d {
                    rotation: Rot2::FRAC_PI_2,
                    translation: Vec2::new(start.x + nudge.x, start.y),
                },
                nudge.x,
                Vec2::new(start.x + 2. * nudge.x, start.y),
            )),
            CompassOctant::West => Some((
                Isometry2d {
                    rotation: Rot2::IDENTITY,
                    translation: Vec2::new(start.x, start.y - nudge.y),
                },
                nudge.y,
                Vec2::new(start.x, start.y - 2. * nudge.y),
            )),
            CompassOctant::NorthWest => Some((
                Isometry2d {
                    rotation: Rot2::IDENTITY,
                    translation: Vec2::new(start.x, start.y - nudge.y),
                },
                nudge.y,
                Vec2::new(start.x, start.y - 2. * nudge.y),
            )),
        }
    } else {
        None
    }
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

struct NavVizDrawData {
    // If the gizmo needs an arc before the arrow, this will be Some((iso, radius))
    maybe_arc: Option<(Isometry2d, f32)>,
    arrow_start: Vec2,
    arrow_end: Vec2,
}

impl NavVizDrawData {
    fn new(arrow_start: Vec2, arrow_end: Vec2) -> Self {
        NavVizDrawData {
            maybe_arc: None,
            arrow_start,
            arrow_end,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    // fn test_arrow_endpoints_for_single_entity_in_dir() {
    //     let config = AutoNavVizGizmoConfigGroup::default();
    //     let from_pos = Vec2::new(0., 0.);
    //     let from_size = Vec2::new(10., 20.);
    //     let to_pos = Vec2::new(20., 0.);
    //     let to_size = Vec2::new(8., 12.);

    //     // since we are only drawing for the current focus, the symmetrical
    //     // aspect of the edge should not apply nudges.
    //     for is_symmetrical in [true, false] {
    //         let (start, end) = get_arrow_endpoints(
    //             from_pos,
    //             from_size,
    //             CompassOctant::NorthEast,
    //             to_pos,
    //             to_size,
    //             is_symmetrical,
    //             &config,
    //         );

    //         assert_eq!(start, Vec2::new(5., 10.));
    //         // to's NW Corner
    //         assert_eq!(end, Vec2::new(16., 6.));

    //         let (start, end) = get_arrow_endpoints(
    //             from_pos,
    //             from_size,
    //             CompassOctant::East,
    //             to_pos,
    //             to_size,
    //             is_symmetrical,
    //             &config,
    //         );

    //         assert_eq!(start, Vec2::new(5., 0.));
    //         // to's Western midpoint
    //         assert_eq!(end, Vec2::new(16., 0.));

    //         let (start, end) = get_arrow_endpoints(
    //             from_pos,
    //             from_size,
    //             CompassOctant::SouthEast,
    //             to_pos,
    //             to_size,
    //             is_symmetrical,
    //             &config,
    //         );

    //         assert_eq!(start, Vec2::new(5., -10.));
    //         // to's SW Corner
    //         assert_eq!(end, Vec2::new(16., -6.));
    //     }
    // }

    // #[test]
    // fn test_arrow_endpoints_for_draw_for_all_in_dir_symmetrical() {
    //     let config = AutoNavVizGizmoConfigGroup {
    //         symmetrical_edge_spacing: 2.,
    //         drawing_mode: AutoNavVizDrawMode::EnabledForAll,
    //         ..default()
    //     };
    //     let from_pos = Vec2::new(10., 0.);
    //     let from_size = Vec2::new(5., 6.);
    //     let to_pos = Vec2::new(12., -20.);
    //     let to_size = Vec2::new(3., 9.);

    //     let (start, end) = get_arrow_endpoints(
    //         from_pos,
    //         from_size,
    //         CompassOctant::SouthWest,
    //         to_pos,
    //         to_size,
    //         true,
    //         &config,
    //     );
    //     // Nudged one unit east
    //     assert_eq!(start, Vec2::new(7.5 + 1., -3.));
    //     // Uses NW corner nudged one unit south
    //     assert_eq!(end, Vec2::new(10.5, -15.5 - 1.));

    //     let (start, end) = get_arrow_endpoints(
    //         from_pos,
    //         from_size,
    //         CompassOctant::South,
    //         to_pos,
    //         to_size,
    //         true,
    //         &config,
    //     );
    //     // Southern point, Nudged one unit east
    //     assert_eq!(start, Vec2::new(10. + 1., -3.));
    //     // Uses NW Corner because it is closer
    //     // Nudged one unit east
    //     assert_eq!(end, Vec2::new(10.5 + 1., -15.5));

    //     let (start, end) = get_arrow_endpoints(
    //         from_pos,
    //         from_size,
    //         CompassOctant::SouthEast,
    //         to_pos,
    //         to_size,
    //         true,
    //         &config,
    //     );
    //     // SE point, nudged one unit north
    //     assert_eq!(start, Vec2::new(12.5, -3. + 1.));
    //     // Uses the Northern point because it is closer
    //     // Nudged one unit east
    //     assert_eq!(end, Vec2::new(12. + 1., -15.5));
    // }
}
