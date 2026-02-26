use core::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI, SQRT_2};

use bevy::ecs::entity::{EntityHashMap, EntityHashSet};
use bevy::input_focus::{InputFocus, directional_navigation::NavNeighbors};
use bevy::math::CompassOctant;
use bevy::prelude::*;

use crate::{
    AutoNavVizColorMode, AutoNavVizDrawMode, AutoNavVizGizmoConfigGroup, NavVizMap,
    SymmetricalEdgeSettings,
};

/// The system that draws the visualizations of the auto navigation
/// system. It uses gizmos to draw arrows between entities.
pub fn draw_nav_viz(
    config_store: Res<GizmoConfigStore>,
    input_focus: Res<InputFocus>,
    nav_viz_map: Res<NavVizMap>,
    mut gizmos: Gizmos<AutoNavVizGizmoConfigGroup>,
    mut processed_entities: Local<EntityHashSet>,
    mut entity_to_color: Local<EntityHashMap<Color>>,
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
        AutoNavVizDrawMode::EnabledForAll(_) => nav_viz_map
            .map
            .neighbors
            .iter()
            .collect::<Vec<(&Entity, &NavNeighbors)>>(),
    };

    processed_entities.clear();
    for (entity, neighbors) in entries_to_draw_nav.into_iter() {
        let from_color = *entity_to_color
            .entry(*entity)
            .or_insert(Oklcha::sequential_dispersed(entity.index_u32()).into());
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
                // A future case to handle more appropriately, perhaps with a white arrow
                // to indicate that it goes elsewhere.
                // A text gizmo could also be used here to indicate where it goes
                // (available in Bevy 0.19)
                continue;
            };
            let nav_edge_is_symmetrical =
                nav_viz_map.map.get_neighbor(*neighbor, dir.opposite()) == Some(*entity);
            // If the draw mode merges symmetrical edges and this is a symmetrical edge,
            // We should only draw the edge once.
            if nav_edge_is_symmetrical
                && let AutoNavVizDrawMode::EnabledForAll(symm_edge_settings) = config.drawing_mode
                && let SymmetricalEdgeSettings::MergeIntoDoubleEnded(_) = symm_edge_settings
                && processed_entities.contains(neighbor)
            {
                continue;
            }
            let to_color = *entity_to_color
                .entry(*neighbor)
                .or_insert(Oklcha::sequential_dispersed(neighbor.index_u32()).into());

            let nav_viz_draw_data = get_nav_viz_draw_data(
                from_pos,
                from_size,
                from_color,
                dir,
                to_pos,
                to_size,
                to_color,
                nav_edge_is_symmetrical,
                config,
            );
            match nav_viz_draw_data {
                NavVizDrawData::Straight(line_data) => {
                    draw_line(&mut gizmos, config, &line_data);
                }
                NavVizDrawData::LoopAround(loop_around_data) => {
                    gizmos.arc_2d(
                        loop_around_data.start_arc.isometry,
                        loop_around_data.start_arc.arc_angle,
                        loop_around_data.start_arc.radius,
                        loop_around_data.start_arc.color,
                    );
                    gizmos.arc_2d(
                        loop_around_data.end_arc.isometry,
                        loop_around_data.end_arc.arc_angle,
                        loop_around_data.end_arc.radius,
                        loop_around_data.end_arc.color,
                    );
                    for line_data in loop_around_data.line_data {
                        draw_line(&mut gizmos, config, &line_data);
                    }
                }
            }
        }
        processed_entities.insert(*entity);
    }
}

// TODO need to reduce the number of arguments supposedly.
// can group the from and to data points into tuples.
fn get_nav_viz_draw_data(
    from_pos: Vec2,
    from_size: Vec2,
    from_color: Color,
    dir: CompassOctant,
    to_pos: Vec2,
    to_size: Vec2,
    to_color: Color,
    is_symmetrical: bool,
    config: &AutoNavVizGizmoConfigGroup,
) -> NavVizDrawData {
    let mut start = get_position_in_direction(from_pos, from_size, dir);
    let (mut end, mut end_dir) = get_closest_point(to_pos, to_size, start);
    let arrow_must_reverse = !dir.is_in_direction(start, end);
    if arrow_must_reverse {
        // The arrow will wrap around the target entity and point to its opposite side.
        // This looks better and conveys the
        // "looping" nature of this navigation path,
        // especially when the arrow has to be double ended.
        end_dir = end_dir.opposite();
        end = get_position_in_direction(to_pos, to_size, end_dir);
    }
    let mut color = config
        .get_color_for_direction(dir)
        .map(|color| {
            if let AutoNavVizColorMode::MixWithEntity(factor) = config.color_mode {
                color.mix(&from_color, factor)
            } else {
                color
            }
        })
        .unwrap_or(from_color);
    let mut line_type = DrawLineType::Arrow;

    if is_symmetrical
        && let AutoNavVizDrawMode::EnabledForAll(symm_edge_settings) = config.drawing_mode
    {
        let start_nudge = match symm_edge_settings {
            SymmetricalEdgeSettings::SpacingBetweenSingleArrows => from_size / 16.,
            _ => Vec2::ZERO,
        };
        let end_nudge = match symm_edge_settings {
            SymmetricalEdgeSettings::SpacingBetweenSingleArrows => to_size / 16.,
            _ => Vec2::ZERO,
        };
        // In general, nudge is applied counter-clockwise for the from* entity.
        // Nudge is applied counter-clockwise for the to* entity if arrow_must_reverse,
        // clockwise otherwise.
        match dir {
            CompassOctant::North => {
                // Nudge West
                start -= Vec2::new(start_nudge.x, 0.);
                end -= Vec2::new(end_nudge.x, 0.);
            }
            CompassOctant::NorthEast => {
                // Nudge West
                start -= Vec2::new(start_nudge.x, 0.);
                // Nudge North
                end += Vec2::new(0., end_nudge.y);
            }
            CompassOctant::East => {
                // Nudge North
                start += Vec2::new(0., start_nudge.y);
                // Nudge North
                end += Vec2::new(0., end_nudge.y);
            }
            CompassOctant::SouthEast => {
                // Nudge North
                start += Vec2::new(0., start_nudge.y);
                // Nudge East
                end += Vec2::new(end_nudge.x, 0.);
            }
            CompassOctant::South => {
                // Nudge East
                start += Vec2::new(start_nudge.x, 0.);
                end += Vec2::new(end_nudge.x, 0.);
            }
            CompassOctant::SouthWest => {
                // Nudge East
                start += Vec2::new(start_nudge.x, 0.);
                // Nudge South
                end -= Vec2::new(0., end_nudge.y);
            }
            CompassOctant::West => {
                // Nudge South
                start -= Vec2::new(0., start_nudge.y);
                end -= Vec2::new(0., end_nudge.y);
            }
            CompassOctant::NorthWest => {
                // Nudge South
                start -= Vec2::new(0., start_nudge.y);
                // Nudge West
                end -= Vec2::new(end_nudge.x, 0.);
            }
        }

        if let SymmetricalEdgeSettings::MergeIntoDoubleEnded(merge_color_factor) =
            symm_edge_settings
        {
            let reverse_color = config
                .get_color_for_direction(dir.opposite())
                .map(|color| {
                    if let AutoNavVizColorMode::MixWithEntity(factor) = config.color_mode {
                        color.mix(&to_color, factor)
                    } else {
                        color
                    }
                })
                .unwrap_or(to_color);
            color = color.mix(&reverse_color, merge_color_factor);
            line_type = DrawLineType::DoubleEndedArrow;
        }
    }

    if arrow_must_reverse {
        // If we must draw a double ended arrow, the line drawn from the source entity to the start arc should
        // have an arrow head facing towards the source entity.
        let start_line_line_type = if line_type == DrawLineType::DoubleEndedArrow {
            DrawLineType::Arrow
        } else {
            DrawLineType::Line
        };

        let (start_line, start_arc, line_start) =
            calculate_arc(start, from_size, dir, false, color, start_line_line_type);
        let (end_line, end_arc, line_end) =
            calculate_arc(end, to_size, end_dir, true, color, DrawLineType::Arrow);
        let line_between_arcs = DrawLineData {
            start: line_start,
            end: line_end,
            color,
            line_type: DrawLineType::Line,
        };
        NavVizDrawData::LoopAround(DrawLoopAroundData {
            start_arc,
            end_arc,
            line_data: [start_line, line_between_arcs, end_line],
        })
    } else {
        NavVizDrawData::Straight(DrawLineData {
            start,
            end,
            color,
            line_type,
        })
    }
}

/// This function returns:
/// - line data between the given `point` to a 180 degree arc. This may or may not be an arrow.
/// - the arc itself.
/// - the endpoint of the arc, where a connecting line may be drawn from/to.
///
/// For ending arcs, the arc should be drawn mirrored.
/// TODO gradient arrows?
fn calculate_arc(
    point: Vec2,
    size: Vec2,
    dir_of_point: CompassOctant,
    mirror: bool,
    color: Color,
    line_type: DrawLineType,
) -> (DrawLineData, DrawArcData, Vec2) {
    // line_start is also the starting point of the arc.
    // This logic also pushes the arc to be drawn further out from the node.
    // It looks a little awkward when drawn too close.
    let line_start = Into::<Dir2>::into(dir_of_point).as_vec2() * 10. + point;
    let draw_line_data = DrawLineData {
        start: line_start,
        end: point,
        color,
        line_type,
    };

    // Ensuring the radius is some fraction of size ensures that
    // multiple consecutive looping edges are spaced out visually when
    // approaching near nodes.
    let radius = size / 12.;
    let translation_nudge = if mirror { -radius } else { radius };
    match dir_of_point {
        CompassOctant::North => (
            draw_line_data,
            DrawArcData {
                isometry: Isometry2d {
                    rotation: Rot2::radians(PI + FRAC_PI_2),
                    translation: Vec2::new(line_start.x - translation_nudge.x, line_start.y),
                },
                arc_angle: PI,
                radius: radius.x,
                color,
            },
            Vec2::new(line_start.x - 2. * translation_nudge.x, line_start.y),
        ),
        CompassOctant::NorthEast => (
            draw_line_data,
            DrawArcData {
                isometry: Isometry2d {
                    rotation: Rot2::radians(PI + FRAC_PI_4),
                    translation: Vec2::new(
                        line_start.x - translation_nudge.x / SQRT_2,
                        line_start.y + translation_nudge.x / SQRT_2,
                    ),
                },
                arc_angle: PI,
                radius: radius.x,
                color,
            },
            Vec2::new(
                line_start.x - 2. * translation_nudge.x / SQRT_2,
                line_start.y + 2. * translation_nudge.x / SQRT_2,
            ),
        ),
        CompassOctant::East => (
            draw_line_data,
            DrawArcData {
                isometry: Isometry2d {
                    rotation: Rot2::PI,
                    translation: Vec2::new(line_start.x, line_start.y + translation_nudge.y),
                },
                arc_angle: PI,
                radius: radius.y,
                color,
            },
            Vec2::new(line_start.x, line_start.y + 2. * translation_nudge.y),
        ),
        CompassOctant::SouthEast => (
            draw_line_data,
            DrawArcData {
                isometry: Isometry2d {
                    rotation: Rot2::radians(FRAC_PI_2 + FRAC_PI_4),
                    translation: Vec2::new(
                        line_start.x + translation_nudge.y / SQRT_2,
                        line_start.y + translation_nudge.y / SQRT_2,
                    ),
                },
                arc_angle: PI,
                radius: radius.y,
                color,
            },
            Vec2::new(
                line_start.x + 2. * translation_nudge.y / SQRT_2,
                line_start.y + 2. * translation_nudge.y / SQRT_2,
            ),
        ),
        CompassOctant::South => (
            draw_line_data,
            DrawArcData {
                isometry: Isometry2d {
                    rotation: Rot2::FRAC_PI_2,
                    translation: Vec2::new(line_start.x + translation_nudge.x, line_start.y),
                },
                arc_angle: PI,
                radius: radius.x,
                color,
            },
            Vec2::new(line_start.x + 2. * translation_nudge.x, line_start.y),
        ),
        CompassOctant::SouthWest => (
            draw_line_data,
            DrawArcData {
                isometry: Isometry2d {
                    rotation: Rot2::FRAC_PI_4,
                    translation: Vec2::new(
                        line_start.x + translation_nudge.x / SQRT_2,
                        line_start.y - translation_nudge.x / SQRT_2,
                    ),
                },
                arc_angle: PI,
                radius: radius.x,
                color,
            },
            Vec2::new(
                line_start.x + 2. * translation_nudge.x / SQRT_2,
                line_start.y - 2. * translation_nudge.x / SQRT_2,
            ),
        ),
        CompassOctant::West => (
            draw_line_data,
            DrawArcData {
                isometry: Isometry2d {
                    rotation: Rot2::IDENTITY,
                    translation: Vec2::new(line_start.x, line_start.y - translation_nudge.y),
                },
                arc_angle: PI,
                radius: radius.y,
                color,
            },
            Vec2::new(line_start.x, line_start.y - 2. * translation_nudge.y),
        ),
        CompassOctant::NorthWest => (
            draw_line_data,
            DrawArcData {
                isometry: Isometry2d {
                    rotation: Rot2::radians(-FRAC_PI_4),
                    translation: Vec2::new(
                        line_start.x - translation_nudge.y / SQRT_2,
                        line_start.y - translation_nudge.y / SQRT_2,
                    ),
                },
                arc_angle: PI,
                radius: radius.y,
                color,
            },
            Vec2::new(
                line_start.x - 2. * translation_nudge.y / SQRT_2,
                line_start.y - 2. * translation_nudge.y / SQRT_2,
            ),
        ),
    }
}

/// Gets the point and direction on the rectangle defined by its center `pos` and `size` that is
/// closest in distance squared to `point`
fn get_closest_point(pos: Vec2, size: Vec2, point: Vec2) -> (Vec2, CompassOctant) {
    let mut closest_dir = CompassOctant::North;
    let mut closest_point = get_position_in_direction(pos, size, closest_dir);
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
            closest_dir = dir;
            closest_point = candidate;
            squared_dist = candidate_dist;
        }
    }
    (closest_point, closest_dir)
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

fn draw_line(
    gizmos: &mut Gizmos<AutoNavVizGizmoConfigGroup>,
    config: &AutoNavVizGizmoConfigGroup,
    line_data: &DrawLineData,
) {
    match line_data.line_type {
        DrawLineType::Line => {
            gizmos.line_2d(line_data.start, line_data.end, line_data.color);
        }
        DrawLineType::Arrow => {
            gizmos
                .arrow_2d(line_data.start, line_data.end, line_data.color)
                .with_tip_length(config.arrow_tip_length);
        }
        DrawLineType::DoubleEndedArrow => {
            gizmos
                .arrow_2d(line_data.start, line_data.end, line_data.color)
                .with_tip_length(config.arrow_tip_length)
                .with_double_end();
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum NavVizDrawData {
    LoopAround(DrawLoopAroundData),
    Straight(DrawLineData),
}

#[derive(Clone, Copy, PartialEq)]
struct DrawLoopAroundData {
    start_arc: DrawArcData,
    end_arc: DrawArcData,
    // contains:
    // - the line from the source node to the start_arc
    // - the mid body line between arcs and the ending arrow
    // - the line from the end_arc to the the destination node
    line_data: [DrawLineData; 3],
}

#[derive(Clone, Copy, PartialEq)]
struct DrawArcData {
    isometry: Isometry2d,
    arc_angle: f32,
    radius: f32,
    color: Color,
}

#[derive(Clone, Copy, PartialEq)]
struct DrawLineData {
    line_type: DrawLineType,
    start: Vec2,
    end: Vec2,
    color: Color,
}

#[derive(Clone, Copy, PartialEq)]
enum DrawLineType {
    Line,
    Arrow,
    DoubleEndedArrow,
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
