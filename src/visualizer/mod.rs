use bevy::ecs::entity::{EntityHashMap, EntityHashSet};
use bevy::input_focus::{InputFocus, directional_navigation::NavNeighbors};
use bevy::math::CompassOctant;
use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::*;

use crate::{AutoNavVizDrawMode, AutoNavVizGizmoConfigGroup, NavVizMap, SymmetricalEdgeSettings};

mod looped;

/// A struct representing a navigation edge's visualization.
#[derive(Clone, Copy, PartialEq)]
pub enum NavVizDrawData {
    /// A navigation edge that connects the two closest points of
    /// two navigation entities. It is broken up into 3 [`DrawLineData`]
    /// segments to allow for a possible color gradient along the arrow.
    /// The first entry is an arrow or line towards the source entity.
    /// The second entry is a line between the source entity's line/arrow and the destination entity's arrow
    /// The third entry is an arrow towards the destination entity.
    Straight([DrawLineData; 3]),

    /// A navigation edge that connects the two closest points of
    /// two navigation entities. It is too short to be broken up
    /// into 3 [`DrawLineData`], so it is just one [`DrawLineData`]
    /// representing an arrow or a double ended arrow
    ShortStraight([DrawLineData; 1]),

    /// A navigation edge that must loop around its entities to point to
    /// their farthest points. See [`DrawLoopedLineData`] for details.
    Looped(DrawLoopedLineData),
}

/// Metadata describing what a [`NavVizDrawData`] connects at a high level.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct NavVizDrawMetaData {
    pub source_entity: Entity,
    /// The direction from which the visualization starts at the source entity
    pub source_direction: CompassOctant,
    pub destination_entity: Entity,
    /// The direction to which the visualization ends on the destination entity.
    /// i.e. SouthEast means that the arrow points towards the SE corner of the
    /// destination entity.
    pub destination_direction: CompassOctant,
}

impl NavVizDrawMetaData {
    /// Returns the meta data for the hypothetical opposite edge compared
    /// to this one (i.e. the source and destination fields switch values).
    pub fn opposite(&self) -> Self {
        NavVizDrawMetaData {
            source_entity: self.destination_entity,
            source_direction: self.destination_direction,
            destination_entity: self.source_entity,
            destination_direction: self.source_direction,
        }
    }
}

/// A struct containing multiple draw elements that, when composed,
/// visualize a "looped" navigation edge. Compared to a "straight"
/// navigation edge, a "looped" edge hooks around its start and
/// destination entities to point to/from their farthest points.
#[derive(Clone, Copy, PartialEq)]
pub struct DrawLoopedLineData {
    /// The arc (semi-circle) drawn near the source entity.
    start_arc: DrawArcData,

    /// The arc (semi-circle) drawn near the destination entity.
    end_arc: DrawArcData,

    /// line_data contains:
    /// - the line from the source entity to the start_arc
    /// - the line between start_arc and end_arc
    /// - the line from the end_arc to the destination entity
    line_data: [DrawLineData; 3],
}

/// A struct containing necessary information to draw an arc
/// via [`Gizmos`].
#[derive(Clone, Copy, PartialEq)]
pub struct DrawArcData {
    isometry: Isometry2d,
    arc_angle: f32,
    radius: f32,
    color: Color,
}

/// A struct containing necessary information to draw a line/arrow
/// via [`Gizmos`].
#[derive(Clone, Copy, PartialEq)]
pub struct DrawLineData {
    line_type: DrawLineType,
    start: Vec2,
    end: Vec2,
    /// Color of the line. If [`DrawLineType`] is [`DrawLineType::Line`] with
    /// an additional line color, this `color` field will be used as the
    /// start_color for `line_gradient_2d`
    color: Color,
}

/// An enum used by [`DrawLineData`] to denote whether the line should
/// be drawn as a Line, Arrow, or a Double Ended Arrow
#[derive(Clone, Copy, PartialEq)]
pub enum DrawLineType {
    /// Used to represent a line. If a color is provided, this line
    /// will be drawn with a gradient via `line_gradient_2d`, using
    /// the given Some(color) as the end_color.
    Line(Option<Color>),
    Arrow,
    DoubleEndedArrow,
}

/// The system that draws the visualizations of the auto navigation
/// system. It uses gizmos to draw arrows between entities.
pub fn draw_nav_viz(
    config_store: Res<GizmoConfigStore>,
    input_focus: Res<InputFocus>,
    nav_viz_map: Res<NavVizMap>,
    mut gizmos: Gizmos<AutoNavVizGizmoConfigGroup>,
    mut processed_entities: Local<EntityHashSet>,
    mut entity_to_color: Local<EntityHashMap<Color>>,
    mut processed_asym_straight_edges: Local<HashSet<NavVizDrawMetaData>>,
    mut asym_straight_edge_map: Local<HashMap<NavVizDrawMetaData, [DrawLineData; 3]>>,
) {
    let config = config_store.config::<AutoNavVizGizmoConfigGroup>().1;
    let entries_to_draw_nav = match config.draw_mode {
        AutoNavVizDrawMode::EnabledForCurrentFocus => {
            if let Some(entity) = &input_focus.0
                && let Some(neighbors) = nav_viz_map.map.get_neighbors(*entity)
            {
                vec![(entity, neighbors)]
            } else {
                return;
            }
        }
        AutoNavVizDrawMode::EnabledForAll(_) => {
            // Ensure entries are processed deterministically to avoid
            // flickering of edges (particularly looping edges)
            let mut entries = nav_viz_map
                .map
                .neighbors
                .iter()
                .collect::<Vec<(&Entity, &NavNeighbors)>>();
            entries.sort_by(|&(&a, _), &(b, _)| a.cmp(b));
            entries
        }
    };

    processed_entities.clear();
    asym_straight_edge_map.clear();
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
                && let AutoNavVizDrawMode::EnabledForAll(symm_edge_settings) = config.draw_mode
                && symm_edge_settings.is_merge()
                && processed_entities.contains(neighbor)
            {
                continue;
            }
            let to_color = *entity_to_color
                .entry(*neighbor)
                .or_insert(Oklcha::sequential_dispersed(neighbor.index_u32()).into());

            let (nav_viz_meta_data, nav_viz_draw_data) = get_nav_viz_draw_data(
                (*entity, from_pos, from_size, from_color),
                dir,
                (*neighbor, to_pos, to_size, to_color),
                nav_edge_is_symmetrical,
                config,
            );
            match nav_viz_draw_data {
                NavVizDrawData::Looped(loop_around_data) => {
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
                NavVizDrawData::ShortStraight(line_data) => {
                    for line_data in line_data {
                        draw_line(&mut gizmos, config, &line_data);
                    }
                }
                NavVizDrawData::Straight(line_data) => {
                    if !nav_edge_is_symmetrical {
                        // Add to the asym_straight_edge_map for further checking before drawing.
                        asym_straight_edge_map.insert(nav_viz_meta_data, line_data);
                    } else {
                        for line_data in line_data {
                            draw_line(&mut gizmos, config, &line_data);
                        }
                    }
                }
            }
        }
        processed_entities.insert(*entity);
    }

    // Merge any asymmetrical straight edges that would be drawn overlapping and
    // therefore are "visualized" symmetric. This process is only necessary
    // if the symmetric edge setting is a "merge" setting
    // TODO what about applying nudge if they should be spaced
    let edges = if let AutoNavVizDrawMode::EnabledForAll(symm_settings) = config.draw_mode
        && symm_settings.is_merge()
    {
        processed_asym_straight_edges.clear();
        let mut edges: Vec<DrawLineData> = Vec::with_capacity(asym_straight_edge_map.len());
        for (meta_data, &edge) in asym_straight_edge_map.iter() {
            let opposite_meta_data = meta_data.opposite();
            if !processed_asym_straight_edges.contains(meta_data)
                && let Some(&other_edge) = asym_straight_edge_map.get(&opposite_meta_data)
            {
                // edge and other_edge are not symmetric in the *navigation* system, but would be
                // draw overlapping, and so "appear" symmetric to the eyes due to mirror symmetry.
                // For example, if between entities A <-> B, there is a NE Nav Edge from A -> B
                // and a NW Nav Edge from B -> A, there will be an edge *drawn* symmetrically
                // between A's NE point and B's NW point, but this is *not* considered a
                // "symmetric navigation edge".
                // In this case, we want to merge the appearance of such "appearing symmetric"
                // edges in accordance with the Symmetrical Edge Settings
                if let SymmetricalEdgeSettings::MergeAndMix(factor) = symm_settings {
                    let mixed_color = edge[0].color.mix(&other_edge[2].color, factor);
                    // override the color of the whole edge with the mixed color.
                    edges.push(DrawLineData {
                        start: edge[0].start,
                        end: edge[0].end,
                        // the segment pointing to the start should be an arrow now that this
                        // edge will appear symmetrical.
                        line_type: DrawLineType::Arrow,
                        color: mixed_color,
                    });
                    edges.push(DrawLineData {
                        start: edge[1].start,
                        end: edge[1].end,
                        line_type: DrawLineType::Line(None),
                        color: mixed_color,
                    });
                    edges.push(DrawLineData {
                        start: edge[2].start,
                        end: edge[2].end,
                        line_type: DrawLineType::Arrow,
                        color: mixed_color,
                    });
                } else {
                    // MergeAndGradient
                    edges.push(DrawLineData {
                        start: edge[0].start,
                        end: edge[0].end,
                        // the segment pointing to the start should be an arrow now that this
                        // edge will appear symmetrical.
                        line_type: DrawLineType::Arrow,
                        color: edge[0].color,
                    });
                    edges.push(DrawLineData {
                        start: edge[1].start,
                        end: edge[1].end,
                        color: edge[0].color,
                        // Add a gradient to the other color for middle segment.
                        line_type: DrawLineType::Line(Some(other_edge[0].color)),
                    });
                    edges.push(DrawLineData {
                        start: edge[2].start,
                        end: edge[2].end,
                        // Use the coloring from the other_edge
                        color: other_edge[0].color,
                        line_type: DrawLineType::Arrow,
                    });
                }
                // TODO what about nudging them when  `SpacingBetweenSingleArrows`
                processed_asym_straight_edges.insert(opposite_meta_data);
            } else if !processed_asym_straight_edges.contains(meta_data) {
                // Draw the asymmetrical edge as normal.
                for line_data in edge {
                    edges.push(line_data);
                }
            }
            processed_asym_straight_edges.insert(*meta_data);
        }
        edges
    } else {
        asym_straight_edge_map
            .iter()
            .flat_map(|(_, &edge)| edge)
            .collect::<Vec<DrawLineData>>()
    };

    for line_data in edges.iter() {
        draw_line(&mut gizmos, config, line_data);
    }
}

/// Creates a tuple of [`NavVizDrawMetaData`] and [`NavVizDrawData`] for the given edge parameters.
///
/// This function decides the most appropriate way to visualize the navigation edge and
/// returns all of the data needed to draw the edge via [`Gizmos`].
fn get_nav_viz_draw_data(
    (from_entity, from_pos, from_size, from_color): (Entity, Vec2, Vec2, Color),
    dir: CompassOctant,
    (to_entity, to_pos, to_size, to_color): (Entity, Vec2, Vec2, Color),
    is_symmetrical: bool,
    config: &AutoNavVizGizmoConfigGroup,
) -> (NavVizDrawMetaData, NavVizDrawData) {
    let mut start = get_position_in_direction(from_pos, from_size, dir);
    let (mut end, mut end_dir) = get_closest_point(to_pos, to_size, start);
    let arrow_must_reverse = !dir.is_in_direction(start, end);
    if arrow_must_reverse {
        // The arrow will wrap around the target entity and point to its opposite side.
        // This looks better and conveys the "looping" nature of this navigation path.
        end_dir = end_dir.opposite();
        end = get_position_in_direction(to_pos, to_size, end_dir);
    }

    let mut line_type = DrawLineType::Arrow;
    let start_color = config.get_color_for_direction(from_color, to_color, dir);
    // Assume they should be colored the same
    let mut end_color = start_color;
    let mut override_color = None;

    if is_symmetrical
        && let AutoNavVizDrawMode::EnabledForAll(symm_edge_settings) = config.draw_mode
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

        if symm_edge_settings.is_merge() {
            // Update the `end_color` to what the opposite arrow would have been colored.
            end_color = config.get_color_for_direction(to_color, from_color, end_dir);
            line_type = DrawLineType::DoubleEndedArrow;
            if let SymmetricalEdgeSettings::MergeAndMix(merge_color_factor) = symm_edge_settings {
                // The whole edge should be colored this override, a mix of both colors.
                override_color = Some(start_color.mix(&end_color, merge_color_factor));
            }
        }
    }

    let meta_data = NavVizDrawMetaData {
        source_entity: from_entity,
        source_direction: dir,
        destination_entity: to_entity,
        destination_direction: end_dir,
    };
    if arrow_must_reverse {
        // If we must draw a double ended arrow, the line drawn from the start arc to the source entity should
        // have an arrow head facing towards the source entity.
        let start_line_is_arrow = line_type == DrawLineType::DoubleEndedArrow;

        (
            meta_data,
            looped::new_looped_draw_data(
                (start, from_size, dir, start_color),
                (end, to_size, end_dir, end_color),
                start_line_is_arrow,
                override_color,
                config,
            ),
        )
    } else if (end - start).length() <= 2. * config.arrow_tip_length {
        // too short to accommodate a line gradient
        (
            meta_data,
            NavVizDrawData::ShortStraight([DrawLineData {
                start,
                end,
                color: start_color,
                line_type,
            }]),
        )
    } else {
        // The direction is multiplied by arrow_tip_length because the arrow tip
        // looks most natural when tip length is proportional to the arrow's length itself.
        let source_arrow_start =
            Into::<Dir2>::into(dir).as_vec2() * config.arrow_tip_length + start;
        let source_arrow_type = if line_type == DrawLineType::DoubleEndedArrow {
            DrawLineType::Arrow
        } else {
            DrawLineType::Line(None)
        };
        let destination_arrow_start =
            Into::<Dir2>::into(end_dir).as_vec2() * config.arrow_tip_length + end;
        (
            meta_data,
            NavVizDrawData::Straight([
                DrawLineData {
                    start: source_arrow_start,
                    end: start,
                    color: override_color.unwrap_or(start_color),
                    line_type: source_arrow_type,
                },
                DrawLineData {
                    start: source_arrow_start,
                    end: destination_arrow_start,
                    color: override_color.unwrap_or(start_color),
                    // If an override was provided, set to None, otherwise to_color
                    line_type: DrawLineType::Line(
                        override_color.map_or_else(|| Some(end_color), |_| None),
                    ),
                },
                DrawLineData {
                    start: destination_arrow_start,
                    end,
                    color: override_color.unwrap_or(end_color),
                    line_type: DrawLineType::Arrow,
                },
            ]),
        )
    }
}

/// Returns the point and direction of the point on the rectangle
/// defined by its center `pos` and `size`. This point is closest in distance
/// squared to `point` compared to the other points in the seven other [`CompassOctant`]
/// directions.
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

/// Returns the point on the rectangle defined by its center `pos` and `size` that is in the direction of `dir`.
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

/// Given a [`DrawLineData`], draws a line or arrow via gizmos.
fn draw_line(
    gizmos: &mut Gizmos<AutoNavVizGizmoConfigGroup>,
    config: &AutoNavVizGizmoConfigGroup,
    line_data: &DrawLineData,
) {
    match line_data.line_type {
        DrawLineType::Line(maybe_color) => {
            if let Some(end_color) = maybe_color {
                gizmos.line_gradient_2d(line_data.start, line_data.end, line_data.color, end_color);
            } else {
                gizmos.line_2d(line_data.start, line_data.end, line_data.color);
            }
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
    //         draw_mode: AutoNavVizDrawMode::EnabledForAll,
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
