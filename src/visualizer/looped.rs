use core::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI, SQRT_2};

use bevy::math::CompassOctant;
use bevy::prelude::*;

use crate::{
    AutoNavVizGizmoConfigGroup, DrawArcData, DrawLineData, DrawLineType, DrawLoopedLineData,
    NavVizDrawData,
};

/// Returns a [`NavVizDrawData`] representing a [`NavVizDrawData::Looped`], a navigation edge
/// that loops around its start and end.
///
/// `start_line_is_arrow` will determine whether the line from the start arc back to the
/// source entity will be an arrow or a plain line.
/// if `override_color` is `Some(Color)`, the draw data will be that color.
/// If `None`, it will be a gradient between `from_color` and `to_color`.
pub(crate) fn new_looped_draw_data(
    (start_point, from_size, start_point_dir, from_color): (Vec2, Vec2, CompassOctant, Color),
    (end_point, to_size, end_point_dir, to_color): (Vec2, Vec2, CompassOctant, Color),
    start_line_is_arrow: bool,
    override_color: Option<Color>,
    config: &AutoNavVizGizmoConfigGroup,
) -> NavVizDrawData {
    let start_line_line_type = if start_line_is_arrow {
        DrawLineType::Arrow
    } else {
        // The line should not be drawn with a gradient.
        DrawLineType::Line(None)
    };
    let (start_line, start_arc, line_start) = calculate_arc(
        start_point,
        from_size,
        start_point_dir,
        false,
        override_color.unwrap_or(from_color),
        start_line_line_type,
        config,
    );
    // The ending arc should always end in an arrow
    let (end_line, end_arc, line_end) = calculate_arc(
        end_point,
        to_size,
        end_point_dir,
        true,
        override_color.unwrap_or(to_color),
        DrawLineType::Arrow,
        config,
    );
    let line_between_arcs = DrawLineData {
        start: line_start,
        end: line_end,
        color: override_color.unwrap_or(from_color),
        // If an override was provided, set to None, otherwise to_color
        line_type: DrawLineType::Line(override_color.map_or_else(|| Some(to_color), |_| None)),
    };
    NavVizDrawData::Looped(DrawLoopedLineData {
        start_arc,
        end_arc,
        line_data: [start_line, line_between_arcs, end_line],
    })
}

/// Helper function for creating [`DrawLoopedLineData`](crate::DrawLoopedLineData).
/// This returns data for drawing the line/arrow that loops out from/
/// in to `point`, the semi-circle arc connected to this line/arrow, and the
/// world coordinate of an endpoint for the connecting line between this arc
/// and its opposite.
///
/// `point` lies in the direction of `dir_of_point` on the rectangle.
/// `size` is the size of said rectangle.
/// The arc and the line/arrow are drawn with the provided `color`.
///
/// Concretely, this function returns:
/// - [`DrawLineData`] for the line/arrow (determined by [`DrawLineType`]) between the given
///   `point` and the semi-circle arc.
/// - the [`DrawArcData`] arc itself.
/// - the endpoint of the arc as a [`Vec2`], where a connecting line may be drawn from/to.
///
/// For starting arcs, `mirror` should be set to false. line_type should be set to
/// [`DrawLineType::Arrow`] or [`DrawLineType::Line`] depending on if a symmetrical
/// edge is being drawn.
///
/// For ending arcs, the arc should be drawn mirrored (`mirror` set to true) for aesthetics.
/// line_type should also be set to [`DrawLineType::Arrow`].
pub(crate) fn calculate_arc(
    point: Vec2,
    size: Vec2,
    dir_of_point: CompassOctant,
    mirror: bool,
    color: Color,
    line_type: DrawLineType,
    config: &AutoNavVizGizmoConfigGroup,
) -> (DrawLineData, DrawArcData, Vec2) {
    // line_start is also the starting point of the arc.
    // This logic also pushes the arc to be drawn further out from the node.
    // It looks a little awkward when drawn too close.
    let line_start = Into::<Dir2>::into(dir_of_point).as_vec2() * config.arrow_tip_length + point;
    let draw_line_data = DrawLineData {
        start: line_start,
        end: point,
        color,
        line_type,
    };

    // Ensuring the radius is some fraction of size ensures that
    // multiple consecutive looping edges are spaced out visually when
    // approaching near nodes. Along a side, we must accommodate at most
    // 3 drawn arcs. Since the arcs can be mirrored, we should accommodate
    // 6 arcs per side to account for these permutations.
    // The radius length is 1/2 the arc diameter. So, the radius must be
    // at most 1/12 the length of a side.
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
