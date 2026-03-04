use bevy::ecs::entity::{EntityHashMap, EntityHashSet};
use bevy::input_focus::{InputFocus, directional_navigation::NavNeighbors};
use bevy::math::CompassOctant;
use bevy::prelude::*;

use crate::{
    AutoNavVizDrawMode, AutoNavVizGizmoConfigGroup, NavVizMap, NavVizPosData,
    SymmetricalEdgeSettings,
};

mod asymm_edge_merger;
mod looped;
use asymm_edge_merger::AsymmetricalStraightEdgeMerger;

/// A struct representing a navigation edge's visualization.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum NavVizDrawData {
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
/// visualizes a "looped" navigation edge. Compared to a "straight"
/// navigation edge, a "looped" edge hooks around its start and
/// destination entities to point to/from their farthest points.
#[derive(Clone, Copy, PartialEq)]
pub(crate) struct DrawLoopedLineData {
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
pub(crate) struct DrawArcData {
    isometry: Isometry2d,
    arc_angle: f32,
    radius: f32,
    color: Color,
}

/// A struct containing necessary information to draw a line/arrow
/// via [`Gizmos`].
#[derive(Clone, Copy, PartialEq)]
pub struct DrawLineData {
    pub line_type: DrawLineType,
    pub start: Vec2,
    pub end: Vec2,
    /// Color of the line. If [`DrawLineType`] is [`DrawLineType::Line`] with
    /// an additional line color, this `color` field will be used as the
    /// start_color for `line_gradient_2d`
    pub color: Color,
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

/// A trait that Draw types share so that they can be visualized.
trait Visualizable {
    /// Draws the visualization for `&self` via [`Gizmos`].
    fn visualize(
        &self,
        gizmos: &mut Gizmos<AutoNavVizGizmoConfigGroup>,
        config: &AutoNavVizGizmoConfigGroup,
    );
}

impl Visualizable for DrawLineData {
    fn visualize(
        &self,
        gizmos: &mut Gizmos<AutoNavVizGizmoConfigGroup>,
        config: &AutoNavVizGizmoConfigGroup,
    ) {
        match self.line_type {
            DrawLineType::Line(maybe_color) => {
                if let Some(end_color) = maybe_color {
                    gizmos.line_gradient_2d(self.start, self.end, self.color, end_color);
                } else {
                    gizmos.line_2d(self.start, self.end, self.color);
                }
            }
            DrawLineType::Arrow => {
                gizmos
                    .arrow_2d(self.start, self.end, self.color)
                    .with_tip_length(config.arrow_tip_length);
            }
            DrawLineType::DoubleEndedArrow => {
                gizmos
                    .arrow_2d(self.start, self.end, self.color)
                    .with_tip_length(config.arrow_tip_length)
                    .with_double_end();
            }
        }
    }
}

impl Visualizable for DrawArcData {
    fn visualize(
        &self,
        gizmos: &mut Gizmos<AutoNavVizGizmoConfigGroup>,
        _config: &AutoNavVizGizmoConfigGroup,
    ) {
        gizmos.arc_2d(self.isometry, self.arc_angle, self.radius, self.color);
    }
}

impl Visualizable for DrawLoopedLineData {
    fn visualize(
        &self,
        gizmos: &mut Gizmos<AutoNavVizGizmoConfigGroup>,
        config: &AutoNavVizGizmoConfigGroup,
    ) {
        self.start_arc.visualize(gizmos, config);
        self.end_arc.visualize(gizmos, config);
        for line_data in self.line_data {
            line_data.visualize(gizmos, config);
        }
    }
}

impl Visualizable for NavVizDrawData {
    fn visualize(
        &self,
        gizmos: &mut Gizmos<AutoNavVizGizmoConfigGroup>,
        config: &AutoNavVizGizmoConfigGroup,
    ) {
        match self {
            NavVizDrawData::Looped(loop_around_data) => {
                loop_around_data.visualize(gizmos, config);
            }
            NavVizDrawData::ShortStraight(line_data) => {
                for line_data in line_data {
                    line_data.visualize(gizmos, config);
                }
            }
            NavVizDrawData::Straight(line_data) => {
                for line_data in line_data {
                    line_data.visualize(gizmos, config);
                }
            }
        }
    }
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
    mut asymm_straight_edge_merger: AsymmetricalStraightEdgeMerger,
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
    processed_entities.shrink_to(nav_viz_map.entity_viz_pos_data.len());
    asymm_straight_edge_merger.clear();
    for (entity, neighbors) in entries_to_draw_nav.into_iter() {
        let from_color = *entity_to_color
            .entry(*entity)
            .or_insert(Oklcha::sequential_dispersed(entity.index_u32()).into());
        for (i, maybe_neighbor) in neighbors.neighbors.iter().enumerate() {
            let Some(neighbor) = maybe_neighbor else {
                continue;
            };
            let Some(from_pos_data) = nav_viz_map.entity_viz_pos_data.get(entity) else {
                continue;
            };
            let Some(dir) = CompassOctant::from_index(i) else {
                continue;
            };
            let Some(to_pos_data) = nav_viz_map.entity_viz_pos_data.get(neighbor) else {
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

            let (meta_data, draw_data) = get_nav_viz_draw_data(
                (*entity, from_pos_data, from_color),
                dir,
                (*neighbor, to_pos_data, to_color),
                nav_edge_is_symmetrical,
                config,
            );

            if let NavVizDrawData::Straight(line_data) = draw_data
                && !nav_edge_is_symmetrical
            {
                // Add to asymm_straight_edge_merger for further processing before drawing.
                asymm_straight_edge_merger.add_straight_edge(meta_data, line_data);
            } else {
                draw_data.visualize(&mut gizmos, config);
            }
        }
        processed_entities.insert(*entity);
    }

    // Ensure any asymmetrical straight edges that might "appear"
    // symmetrical when drawn are also merged.
    asymm_straight_edge_merger.do_merge(&nav_viz_map, config);
    for line_data in asymm_straight_edge_merger.get_line_data().iter() {
        line_data.visualize(&mut gizmos, config);
    }
}

/// Creates a tuple of [`NavVizDrawMetaData`] and [`NavVizDrawData`] for the given edge parameters.
///
/// This function decides the most appropriate way to visualize the navigation edge and
/// returns all of the data needed to draw the edge via [`Gizmos`].
fn get_nav_viz_draw_data(
    (from_entity, from_pos_data, from_color): (Entity, &NavVizPosData, Color),
    dir: CompassOctant,
    (to_entity, to_pos_data, to_color): (Entity, &NavVizPosData, Color),
    is_symmetrical: bool,
    config: &AutoNavVizGizmoConfigGroup,
) -> (NavVizDrawMetaData, NavVizDrawData) {
    let mut start = from_pos_data.get_point_in_direction(dir);
    let (mut end, mut end_dir) = get_closest_point_in_dir(to_pos_data, start, dir);
    let arrow_must_reverse = !dir.is_in_direction(start, end);
    if arrow_must_reverse {
        // The arrow will wrap around the target entity and point to its opposite side.
        // This looks better and conveys the "looping" nature of this navigation path.
        end_dir = end_dir.opposite();
        end = to_pos_data.get_point_in_direction(end_dir);
    }

    let mut line_type = DrawLineType::Arrow;
    let start_color = config.get_color_for_direction(from_color, to_color, dir);
    // Assume they should be colored the same
    let mut end_color = start_color;
    let mut override_color = None;

    if is_symmetrical
        && let AutoNavVizDrawMode::EnabledForAll(symm_edge_settings) = config.draw_mode
    {
        let (start_local_nudge, end_local_nudge) = get_local_nudge(
            from_pos_data.obb_size,
            dir,
            to_pos_data.obb_size,
            end_dir,
            symm_edge_settings,
        );
        start = from_pos_data.apply_local_translation(start, start_local_nudge);
        end = to_pos_data.apply_local_translation(end, end_local_nudge);
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
                (start, from_pos_data, dir, start_color),
                (end, to_pos_data, end_dir, end_color),
                start_line_is_arrow,
                override_color,
                config,
            ),
        )
    } else if (end - start).length() <= 2. * config.arrow_tip_length {
        // too short to potentially accommodate a line gradient
        // Todo GH Issue #10: Gizmo arrows should support line gradients on their own.
        (
            meta_data,
            NavVizDrawData::ShortStraight([DrawLineData {
                start,
                end,
                color: override_color.unwrap_or(start_color),
                line_type,
            }]),
        )
    } else {
        // The direction is multiplied by arrow_tip_length because the arrow tip
        // looks most natural when tip length is proportional to the arrow's length itself.
        let source_line_start = Into::<Dir2>::into(dir).as_vec2() * config.arrow_tip_length + start;
        let source_line_type = if line_type == DrawLineType::DoubleEndedArrow {
            DrawLineType::Arrow
        } else {
            DrawLineType::Line(None)
        };
        let destination_arrow_start =
            Into::<Dir2>::into(end_dir).as_vec2() * config.arrow_tip_length + end;
        let gradient_color = if override_color.is_some() {
            None
        } else {
            Some(end_color)
        };
        (
            meta_data,
            NavVizDrawData::Straight([
                DrawLineData {
                    start: source_line_start,
                    end: start,
                    color: override_color.unwrap_or(start_color),
                    line_type: source_line_type,
                },
                DrawLineData {
                    start: source_line_start,
                    end: destination_arrow_start,
                    color: override_color.unwrap_or(start_color),
                    line_type: DrawLineType::Line(gradient_color),
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

/// Nudge by one sixteenth of the local size of each node.
/// One sixteenth is very reasonable and looks good aesthetically, which, spread
/// equally between looped edges, is an additional 1/4 of a side.
const LOCAL_SIZE_NUDGE_PROPORTION: f32 = 1. / 16.;

/// Returns a nudge to be applied to the `start` and `end` of the drawn edge
/// in each entity's local units
/// if `symm_edge_settings` is [`SymmetricalEdgeSettings::SpacingBetweenSingleArrows`].
///
/// The returned nudge is proportional to the sizes of the entities.
/// Nudge is calculated counter-clockwise for the source entity and clockwise
/// for the destination entity.
pub(crate) fn get_local_nudge(
    from_local_size: Vec2,
    start_dir: CompassOctant,
    to_local_size: Vec2,
    end_dir: CompassOctant,
    symm_edge_settings: SymmetricalEdgeSettings,
) -> (Vec2, Vec2) {
    let start_nudge_units = match symm_edge_settings {
        SymmetricalEdgeSettings::SpacingBetweenSingleArrows => {
            from_local_size * LOCAL_SIZE_NUDGE_PROPORTION
        }
        _ => Vec2::ZERO,
    };
    let end_nudge_units = match symm_edge_settings {
        SymmetricalEdgeSettings::SpacingBetweenSingleArrows => {
            to_local_size * LOCAL_SIZE_NUDGE_PROPORTION
        }
        _ => Vec2::ZERO,
    };
    let mut start_nudge = Vec2::ZERO;
    let mut end_nudge = Vec2::ZERO;
    match start_dir {
        CompassOctant::North => {
            // Nudge West
            start_nudge -= Vec2::new(start_nudge_units.x, 0.);
        }
        CompassOctant::NorthEast => {
            // Nudge West
            start_nudge -= Vec2::new(start_nudge_units.x, 0.);
        }
        CompassOctant::East => {
            // Nudge North
            start_nudge += Vec2::new(0., start_nudge_units.y);
        }
        CompassOctant::SouthEast => {
            // Nudge North
            start_nudge += Vec2::new(0., start_nudge_units.y);
        }
        CompassOctant::South => {
            // Nudge East
            start_nudge += Vec2::new(start_nudge_units.x, 0.);
        }
        CompassOctant::SouthWest => {
            // Nudge East
            start_nudge += Vec2::new(start_nudge_units.x, 0.);
        }
        CompassOctant::West => {
            // Nudge South
            start_nudge -= Vec2::new(0., start_nudge_units.y);
        }
        CompassOctant::NorthWest => {
            // Nudge South
            start_nudge -= Vec2::new(0., start_nudge_units.y);
        }
    }

    match end_dir {
        CompassOctant::North => {
            // Nudge East
            end_nudge += Vec2::new(end_nudge_units.x, 0.);
        }
        CompassOctant::NorthEast => {
            // Nudge South
            end_nudge -= Vec2::new(0., end_nudge_units.y);
        }
        CompassOctant::East => {
            // Nudge South
            end_nudge -= Vec2::new(0., end_nudge_units.y);
        }
        CompassOctant::SouthEast => {
            // Nudge West
            end_nudge -= Vec2::new(end_nudge_units.x, 0.);
        }
        CompassOctant::South => {
            // Nudge West
            end_nudge -= Vec2::new(end_nudge_units.x, 0.);
        }
        CompassOctant::SouthWest => {
            // Nudge North
            end_nudge += Vec2::new(0., end_nudge_units.y);
        }
        CompassOctant::West => {
            // Nudge North
            end_nudge += Vec2::new(0., end_nudge_units.y);
        }
        CompassOctant::NorthWest => {
            // Nudge East
            end_nudge += Vec2::new(end_nudge_units.x, 0.);
        }
    }
    (start_nudge, end_nudge)
}

/// Returns a point and direction of the point on the entity's rectangle
/// defined by `pos_data`. This point is closest in distance
/// squared to `point` compared to the other points in the seven other [`CompassOctant`]
/// directions. Ideally, it is also in the desired direction `dir` from `point`.
///
/// If there is no point that is also in the direction of dir, it returns the closest point
/// regardless.
fn get_closest_point_in_dir(
    pos_data: &NavVizPosData,
    point: Vec2,
    in_dir: CompassOctant,
) -> (Vec2, CompassOctant) {
    let mut closest_dir = CompassOctant::North;
    let mut closest_point = pos_data.get_point_in_direction(closest_dir);
    let mut squared_dist = closest_point.distance_squared(point);
    let mut closest_point_is_in_dir = in_dir.is_in_direction(point, closest_point);
    for dir in [
        CompassOctant::NorthEast,
        CompassOctant::East,
        CompassOctant::SouthEast,
        CompassOctant::South,
        CompassOctant::SouthWest,
        CompassOctant::West,
        CompassOctant::NorthWest,
    ] {
        let candidate = pos_data.get_point_in_direction(dir);
        let candidate_dist = candidate.distance_squared(point);
        let candidate_is_in_dir = in_dir.is_in_direction(point, candidate);
        // If all candidates are not in_dir, this will return the closest point that is not in_dir
        // If there is one candidate in_dir, it should be returned even if it's not technically
        // the closest.
        if (candidate_dist < squared_dist && candidate_is_in_dir == closest_point_is_in_dir)
            || (!closest_point_is_in_dir && candidate_is_in_dir)
        {
            closest_dir = dir;
            closest_point = candidate;
            squared_dist = candidate_dist;
            closest_point_is_in_dir = candidate_is_in_dir;
        }
    }
    (closest_point, closest_dir)
}

#[cfg(test)]
mod tests {
    use std::f32::consts::FRAC_PI_2;

    use super::*;

    fn assert_eq_vec2(left: Vec2, right: Vec2) {
        let difference = left - right;
        assert!(
            difference.x.abs() <= 1e-6,
            "left: {}\n right: {}",
            left,
            right
        );
        assert!(
            difference.y.abs() <= 1e-6,
            "left: {}\n right: {}",
            left,
            right
        );
    }

    #[test]
    fn test_local_nudge() {
        let (start_nudge, end_nudge) = get_local_nudge(
            Vec2::new(30., 20.),
            CompassOctant::North,
            Vec2::new(7., 12.),
            CompassOctant::South,
            SymmetricalEdgeSettings::OverlappingSingleArrows,
        );
        // Not `SpacingBetweenSingleArrows`, so no nudge applied
        assert_eq!(start_nudge, Vec2::ZERO);
        assert_eq!(end_nudge, Vec2::ZERO);

        let (start_nudge, end_nudge) = get_local_nudge(
            Vec2::new(30., 20.),
            CompassOctant::North,
            Vec2::new(7., 12.),
            CompassOctant::South,
            SymmetricalEdgeSettings::SpacingBetweenSingleArrows,
        );
        // moves west for both
        assert_eq!(
            start_nudge,
            Vec2::new(-30. * LOCAL_SIZE_NUDGE_PROPORTION, 0.)
        );
        assert_eq!(end_nudge, Vec2::new(-7. * LOCAL_SIZE_NUDGE_PROPORTION, 0.));

        let (start_nudge, end_nudge) = get_local_nudge(
            Vec2::new(30., 20.),
            CompassOctant::South,
            Vec2::new(7., 12.),
            CompassOctant::North,
            SymmetricalEdgeSettings::SpacingBetweenSingleArrows,
        );
        // moves east for both
        assert_eq!(
            start_nudge,
            Vec2::new(30. * LOCAL_SIZE_NUDGE_PROPORTION, 0.)
        );
        assert_eq!(end_nudge, Vec2::new(7. * LOCAL_SIZE_NUDGE_PROPORTION, 0.));

        let (start_nudge, end_nudge) = get_local_nudge(
            Vec2::new(30., 20.),
            CompassOctant::East,
            Vec2::new(7., 12.),
            CompassOctant::West,
            SymmetricalEdgeSettings::SpacingBetweenSingleArrows,
        );
        // moves north for both
        assert_eq!(
            start_nudge,
            Vec2::new(0., 20. * LOCAL_SIZE_NUDGE_PROPORTION)
        );
        assert_eq!(end_nudge, Vec2::new(0., 12. * LOCAL_SIZE_NUDGE_PROPORTION));

        let (start_nudge, end_nudge) = get_local_nudge(
            Vec2::new(30., 20.),
            CompassOctant::West,
            Vec2::new(7., 12.),
            CompassOctant::East,
            SymmetricalEdgeSettings::SpacingBetweenSingleArrows,
        );
        // moves south for both
        assert_eq!(
            start_nudge,
            Vec2::new(0., -20. * LOCAL_SIZE_NUDGE_PROPORTION)
        );
        assert_eq!(end_nudge, Vec2::new(0., -12. * LOCAL_SIZE_NUDGE_PROPORTION));

        let (start_nudge, end_nudge) = get_local_nudge(
            Vec2::new(30., 20.),
            CompassOctant::NorthEast,
            Vec2::new(7., 12.),
            CompassOctant::SouthWest,
            SymmetricalEdgeSettings::SpacingBetweenSingleArrows,
        );
        // moves west
        assert_eq!(
            start_nudge,
            Vec2::new(-30. * LOCAL_SIZE_NUDGE_PROPORTION, 0.)
        );
        // moves north
        assert_eq!(end_nudge, Vec2::new(0., 12. * LOCAL_SIZE_NUDGE_PROPORTION));

        let (start_nudge, end_nudge) = get_local_nudge(
            Vec2::new(30., 20.),
            CompassOctant::SouthWest,
            Vec2::new(7., 12.),
            CompassOctant::NorthEast,
            SymmetricalEdgeSettings::SpacingBetweenSingleArrows,
        );
        // moves east
        assert_eq!(
            start_nudge,
            Vec2::new(30. * LOCAL_SIZE_NUDGE_PROPORTION, 0.)
        );
        // moves south
        assert_eq!(end_nudge, Vec2::new(0., -12. * LOCAL_SIZE_NUDGE_PROPORTION));

        let (start_nudge, end_nudge) = get_local_nudge(
            Vec2::new(30., 20.),
            CompassOctant::NorthWest,
            Vec2::new(7., 12.),
            CompassOctant::SouthEast,
            SymmetricalEdgeSettings::SpacingBetweenSingleArrows,
        );
        // moves south
        assert_eq!(
            start_nudge,
            Vec2::new(0., -20. * LOCAL_SIZE_NUDGE_PROPORTION)
        );
        // moves west
        assert_eq!(end_nudge, Vec2::new(-7. * LOCAL_SIZE_NUDGE_PROPORTION, 0.));

        let (start_nudge, end_nudge) = get_local_nudge(
            Vec2::new(30., 20.),
            CompassOctant::SouthEast,
            Vec2::new(7., 12.),
            CompassOctant::NorthWest,
            SymmetricalEdgeSettings::SpacingBetweenSingleArrows,
        );
        // moves north
        assert_eq!(
            start_nudge,
            Vec2::new(0., 20. * LOCAL_SIZE_NUDGE_PROPORTION)
        );
        // moves east
        assert_eq!(end_nudge, Vec2::new(7. * LOCAL_SIZE_NUDGE_PROPORTION, 0.));
    }

    #[test]
    fn test_closest_point_in_dir() {
        // An entity with corners at NE (8., -2.), NW(-22., -2.), SW(-22., -22.), SE(8., -22.)
        let entity = NavVizPosData {
            aabb_size: Vec2::new(30., 20.),
            transformation: Isometry2d {
                rotation: Rot2::radians(FRAC_PI_2),
                translation: Vec2::new(-7., -12.),
            },
            obb_size: Vec2::new(20., 30.),
        };
        let point = Vec2::new(10., -4.);

        let (closest, dir) = get_closest_point_in_dir(&entity, point, CompassOctant::SouthWest);
        // Even though the NE point is closer, it is not in the SW direction of the given point.
        // The closest point in the SE direction is the eastern point.
        assert_eq_vec2(closest, Vec2::new(8., -12.));
        assert_eq!(dir, CompassOctant::East);

        let (closest, dir) = get_closest_point_in_dir(&entity, point, CompassOctant::West);
        // Returns the NE point since it is in the W direction of the given point.
        assert_eq_vec2(closest, Vec2::new(8., -2.));
        assert_eq!(dir, CompassOctant::NorthEast);

        let (closest, dir) = get_closest_point_in_dir(&entity, point, CompassOctant::East);
        // Returns the NE point since it is the closest point. All of the points
        // on the entity are not in the direction from the starting point.
        assert_eq_vec2(closest, Vec2::new(8., -2.));
        assert_eq!(dir, CompassOctant::NorthEast);
    }
}
