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
                    if let Some(line_data) = loop_around_data.start_line_data {
                        draw_line(&mut gizmos, config, &line_data);
                    }
                    for line_data in loop_around_data.line_data {
                        draw_line(&mut gizmos, config, &line_data);
                    }
                }
            }
        }
        processed_entities.insert(*entity);
    }
}

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
        let ((start_isom, start_arc_angle, start_radius), line_start) =
            calculate_start_arc(start, from_size, dir);
        let ((end_isom, end_arc_angle, end_radius), line_end) =
            calculate_end_arc(end, to_size, end_dir);
        let start_line_data = if line_type == DrawLineType::DoubleEndedArrow {
            // The symmetrical double ended arrow will be rendered as
            // two single arrows connected by arcs and a line.
            // This arrow is at the beginning of the start arc
            // The arrow nudge is necessary for the arrow head to render.
            let arrow_nudge = Into::<Dir2>::into(dir).as_vec2();
            Some(DrawLineData {
                start: start + arrow_nudge,
                end: start,
                color,
                line_type: DrawLineType::Arrow,
            })
        } else {
            None
        };
        // The arrow nudge is necessary for the arrow head to render.
        let arrow_nudge = Into::<Dir2>::into(end_dir).as_vec2();
        NavVizDrawData::LoopAround(DrawLoopAroundData {
            start_arc: DrawArcData {
                isometry: start_isom,
                arc_angle: start_arc_angle,
                radius: start_radius,
                color,
            },
            end_arc: DrawArcData {
                isometry: end_isom,
                arc_angle: end_arc_angle,
                radius: end_radius,
                color,
            },
            start_line_data,
            line_data: [
                DrawLineData {
                    start: line_start,
                    end: line_end,
                    color,
                    line_type: DrawLineType::Line,
                },
                // The arrow is at the end of the end arc
                DrawLineData {
                    start: end + arrow_nudge,
                    end,
                    color,
                    line_type: DrawLineType::Arrow,
                },
            ],
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

/// This function returns the start arc's isometry, arc_angle, and radius,
/// alongside the new line start point to compensate for the additional arc.
fn calculate_start_arc(
    point: Vec2,
    from_size: Vec2,
    dir_of_point: CompassOctant,
) -> ((Isometry2d, f32, f32), Vec2) {
    let nudge = from_size / 8.;
    match dir_of_point {
        CompassOctant::North => (
            (
                Isometry2d {
                    rotation: Rot2::radians(PI + FRAC_PI_2),
                    translation: Vec2::new(point.x - nudge.x, point.y),
                },
                PI,
                nudge.x,
            ),
            Vec2::new(point.x - 2. * nudge.x, point.y),
        ),
        CompassOctant::NorthEast => (
            (
                Isometry2d {
                    rotation: Rot2::radians(PI + FRAC_PI_4),
                    translation: Vec2::new(point.x - nudge.x / SQRT_2, point.y + nudge.x / SQRT_2),
                },
                PI,
                nudge.x,
            ),
            Vec2::new(
                point.x - 2. * nudge.x / SQRT_2,
                point.y + 2. * nudge.x / SQRT_2,
            ),
        ),
        CompassOctant::East => (
            (
                Isometry2d {
                    rotation: Rot2::PI,
                    translation: Vec2::new(point.x, point.y + nudge.y),
                },
                PI,
                nudge.y,
            ),
            Vec2::new(point.x, point.y + 2. * nudge.y),
        ),
        CompassOctant::SouthEast => (
            (
                Isometry2d {
                    rotation: Rot2::radians(FRAC_PI_2 + FRAC_PI_4),
                    translation: Vec2::new(point.x + nudge.y / SQRT_2, point.y + nudge.y / SQRT_2),
                },
                PI,
                nudge.y,
            ),
            Vec2::new(
                point.x + 2. * nudge.y / SQRT_2,
                point.y + 2. * nudge.y / SQRT_2,
            ),
        ),
        CompassOctant::South => (
            (
                Isometry2d {
                    rotation: Rot2::FRAC_PI_2,
                    translation: Vec2::new(point.x + nudge.x, point.y),
                },
                PI,
                nudge.x,
            ),
            Vec2::new(point.x + 2. * nudge.x, point.y),
        ),
        CompassOctant::SouthWest => (
            (
                Isometry2d {
                    rotation: Rot2::FRAC_PI_4,
                    translation: Vec2::new(point.x + nudge.x / SQRT_2, point.y - nudge.x / SQRT_2),
                },
                PI,
                nudge.x,
            ),
            Vec2::new(
                point.x + 2. * nudge.x / SQRT_2,
                point.y - 2. * nudge.x / SQRT_2,
            ),
        ),
        CompassOctant::West => (
            (
                Isometry2d {
                    rotation: Rot2::IDENTITY,
                    translation: Vec2::new(point.x, point.y - nudge.y),
                },
                PI,
                nudge.y,
            ),
            Vec2::new(point.x, point.y - 2. * nudge.y),
        ),
        CompassOctant::NorthWest => (
            (
                Isometry2d {
                    rotation: Rot2::radians(-FRAC_PI_4),
                    translation: Vec2::new(point.x - nudge.y / SQRT_2, point.y - nudge.y / SQRT_2),
                },
                PI,
                nudge.y,
            ),
            Vec2::new(
                point.x - 2. * nudge.y / SQRT_2,
                point.y - 2. * nudge.y / SQRT_2,
            ),
        ),
    }
}

/// This function returns the end arc's isometry, arc_angle, and radius,
/// alongside the new line end point to compensate for the additional arc.
fn calculate_end_arc(
    point: Vec2,
    from_size: Vec2,
    dir_of_point: CompassOctant,
) -> ((Isometry2d, f32, f32), Vec2) {
    // Is this worth consolidating with calculate_start_arc?
    let nudge = from_size / 8.;
    match dir_of_point {
        CompassOctant::North => (
            (
                Isometry2d {
                    rotation: Rot2::radians(PI + FRAC_PI_2),
                    translation: Vec2::new(point.x + nudge.x, point.y),
                },
                PI,
                nudge.x,
            ),
            Vec2::new(point.x + 2. * nudge.x, point.y),
        ),
        CompassOctant::NorthEast => (
            (
                Isometry2d {
                    rotation: Rot2::radians(PI + FRAC_PI_4),
                    translation: Vec2::new(point.x + nudge.x / SQRT_2, point.y - nudge.x / SQRT_2),
                },
                PI,
                nudge.x,
            ),
            Vec2::new(
                point.x + 2. * nudge.x / SQRT_2,
                point.y - 2. * nudge.x / SQRT_2,
            ),
        ),
        CompassOctant::East => (
            (
                Isometry2d {
                    rotation: Rot2::radians(PI),
                    translation: Vec2::new(point.x, point.y - nudge.y),
                },
                PI,
                nudge.y,
            ),
            Vec2::new(point.x, point.y - 2. * nudge.y),
        ),
        CompassOctant::SouthEast => (
            (
                Isometry2d {
                    rotation: Rot2::radians(FRAC_PI_2 + FRAC_PI_4),
                    translation: Vec2::new(point.x - nudge.y / SQRT_2, point.y - nudge.y / SQRT_2),
                },
                PI,
                nudge.y,
            ),
            Vec2::new(
                point.x - 2. * nudge.y / SQRT_2,
                point.y - 2. * nudge.y / SQRT_2,
            ),
        ),
        CompassOctant::South => (
            (
                Isometry2d {
                    rotation: Rot2::radians(FRAC_PI_2),
                    translation: Vec2::new(point.x - nudge.x, point.y),
                },
                PI,
                nudge.x,
            ),
            Vec2::new(point.x - 2. * nudge.x, point.y),
        ),
        CompassOctant::SouthWest => (
            (
                Isometry2d {
                    rotation: Rot2::radians(FRAC_PI_4),
                    translation: Vec2::new(point.x - nudge.x / SQRT_2, point.y + nudge.x / SQRT_2),
                },
                PI,
                nudge.x,
            ),
            Vec2::new(
                point.x - 2. * nudge.x / SQRT_2,
                point.y + 2. * nudge.x / SQRT_2,
            ),
        ),
        CompassOctant::West => (
            (
                Isometry2d {
                    rotation: Rot2::IDENTITY,
                    translation: Vec2::new(point.x, point.y + nudge.y),
                },
                PI,
                nudge.y,
            ),
            Vec2::new(point.x, point.y + 2. * nudge.y),
        ),
        CompassOctant::NorthWest => (
            (
                Isometry2d {
                    rotation: Rot2::radians(-FRAC_PI_4),
                    translation: Vec2::new(point.x + nudge.y / SQRT_2, point.y + nudge.y / SQRT_2),
                },
                PI,
                nudge.y,
            ),
            Vec2::new(
                point.x + 2. * nudge.y / SQRT_2,
                point.y + 2. * nudge.y / SQRT_2,
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
    // for symmetrical looping edges, this will be set to draw an arrow.
    start_line_data: Option<DrawLineData>,
    // the mid body line between arcs and the ending arrow
    line_data: [DrawLineData; 2],
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
