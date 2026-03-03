use core::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI, SQRT_2};

use bevy::math::CompassOctant;
use bevy::prelude::*;

use crate::{
    AutoNavVizGizmoConfigGroup, DrawArcData, DrawLineData, DrawLineType, DrawLoopedLineData,
    NavVizDrawData, NavVizPosData,
};

/// Returns a [`NavVizDrawData`] representing a [`NavVizDrawData::Looped`], a navigation edge
/// that loops around its start and end.
///
/// `start_line_is_arrow` will determine whether the line from the start arc back to the
/// source entity will be an arrow or a plain line.
/// if `override_color` is `Some(Color)`, the draw data will be that color.
/// If `None`, it will be a gradient between `from_color` and `to_color`.
pub(crate) fn new_looped_draw_data(
    (start_point, from_pos_data, start_point_dir, from_color): (
        Vec2,
        &NavVizPosData,
        CompassOctant,
        Color,
    ),
    (end_point, to_pos_data, end_point_dir, to_color): (Vec2, &NavVizPosData, CompassOctant, Color),
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
    let start_to_end_dir = (end_point - start_point).normalize();
    // We work off the default assumption that the arrow will head in the exact
    // opposite direction as the `start_point_dir`. This corresponds with
    // a 180 degree rotation, aka pi-radians rotation.
    // This dot product is how much we deviate from that assumption.
    let mut radians_from_pi_arc = ops::acos(
        Into::<Dir2>::into(start_point_dir.opposite())
            .as_vec2()
            .dot(start_to_end_dir),
    );
    // Since the dot product is always positive, we need to figure out
    // Whether to add or subtract from the pi-radians rotation.
    // For that, we use the cross product.
    let orientation =
        start_to_end_dir.perp_dot(Into::<Dir2>::into(start_point_dir.opposite()).as_vec2());
    if orientation >= 0. {
        radians_from_pi_arc *= -1.;
    }
    let (start_line, start_arc, line_start) = calculate_arc(
        (start_point, from_pos_data, start_point_dir),
        false,
        override_color.unwrap_or(from_color),
        start_line_line_type,
        radians_from_pi_arc,
        config,
    );

    let end_to_start_dir = (start_point - end_point).normalize();
    let mut another_radians_from_pi_arc = ops::acos(
        Into::<Dir2>::into(end_point_dir.opposite())
            .as_vec2()
            .dot(end_to_start_dir),
    );

    // Since the dot product is always positive, we need to figure out
    // Whether to add or subtract from the pi-radians rotation.
    // For that, we use the cross product.
    let other_orientation =
        end_to_start_dir.perp_dot(Into::<Dir2>::into(end_point_dir.opposite()).as_vec2());
    // this is correct with the radians adding logic
    if other_orientation < 0. {
        another_radians_from_pi_arc *= -1.;
    }
    // The ending arc should always end in an arrow
    let (end_line, end_arc, line_end) = calculate_arc_mirror(
        (end_point, to_pos_data, end_point_dir),
        false,
        override_color.unwrap_or(to_color),
        DrawLineType::Arrow,
        another_radians_from_pi_arc,
        config,
    );
    let gradient_color = if override_color.is_some() {
        None
    } else {
        Some(to_color)
    };
    let line_between_arcs = DrawLineData {
        start: line_start,
        end: line_end,
        color: override_color.unwrap_or(from_color),
        line_type: DrawLineType::Line(gradient_color),
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
    (point, pos_data, dir_of_point): (Vec2, &NavVizPosData, CompassOctant),
    mirror: bool,
    color: Color,
    line_type: DrawLineType,
    arc_angle_from_pi_radians: f32,
    config: &AutoNavVizGizmoConfigGroup,
) -> (DrawLineData, DrawArcData, Vec2) {
    // line_start is also the starting point of the arc.
    // This logic also pushes the arc to be drawn further out from the entities.
    // It looks a little awkward when drawn too close.
    // The direction is multiplied by arrow_tip_length because the arrow tip
    // looks most natural when tip length is proportional to the arrow's length itself.
    let line_start = Into::<Dir2>::into(dir_of_point).as_vec2() * config.arrow_tip_length + point;
    let draw_line_data = DrawLineData {
        start: line_start,
        end: point,
        color,
        line_type,
    };

    // Ensuring the radius is some fraction of size ensures that
    // multiple consecutive looping edges are spaced out visually when
    // approaching near entities. Along a side, we must accommodate at most
    // 3 drawn arcs. Since the arcs can be mirrored, we should accommodate
    // 6 arcs per side to account for these permutations.
    // The radius length is 1/2 the arc diameter. So, the radius must be
    // at most 1/12 the length of a side.
    let radius = if mirror {
        -pos_data.obb_size / 12.
    } else {
        pos_data.obb_size / 12.
    };
    let arc_angle = PI + arc_angle_from_pi_radians;

    // TODO continue from here.
    // TODO for mirror'd, we need to adjust the
    // Starting point of the arc because for some reason the arrow gets detached

    // This is correct for settings and what i ideally want
    let isometry_nudge = radius;
    // but looks better in the other example.
    // let isometry_nudge = if mirror { -radius} else radius;
    let endpoint_nudge = isometry_nudge * Vec2::new(ops::cos(arc_angle), ops::sin(arc_angle));
    match dir_of_point {
        CompassOctant::North => {
            let rotation = if mirror {
                Rot2::radians(FRAC_PI_2)
            } else {
                Rot2::radians(PI + FRAC_PI_2)
            };
            (
                draw_line_data,
                DrawArcData {
                    isometry: Isometry2d {
                        rotation,
                        // non-mirrored is nudged West, similar to get_local_nudge
                        // mirrored is nudged east
                        translation: Vec2::new(line_start.x - isometry_nudge.x, line_start.y),
                    },
                    arc_angle: arc_angle,
                    radius: radius.x,
                    color,
                },
                Vec2::new(line_start.x - isometry_nudge.x, line_start.y)
                    + Rot2::IDENTITY * endpoint_nudge,
            )
        }
        CompassOctant::NorthEast => {
            let (isometry_rotation, endpoint_rotation) = if mirror {
                (Rot2::radians(FRAC_PI_4), Rot2::radians(-FRAC_PI_4))
            } else {
                (Rot2::radians(PI + FRAC_PI_4), Rot2::radians(-FRAC_PI_4))
            };
            (
                draw_line_data,
                DrawArcData {
                    isometry: Isometry2d {
                        rotation: isometry_rotation,
                        translation: Vec2::new(
                            // non-mirrored is nudged West
                            // mirrored is nudged East
                            line_start.x - isometry_nudge.x / SQRT_2,
                            // non-mirrored is nudged North, similar to get_local_nudge
                            // mirrored is nudged South
                            line_start.y + isometry_nudge.x / SQRT_2,
                        ),
                    },
                    arc_angle: arc_angle,
                    radius: radius.x,
                    color,
                },
                Vec2::new(
                    line_start.x - isometry_nudge.x / SQRT_2,
                    line_start.y + isometry_nudge.x / SQRT_2,
                ) + endpoint_rotation * endpoint_nudge,
            )
        }
        CompassOctant::East => {
            let (isometry_rotation, endpoint_rotation) = if mirror {
                (Rot2::IDENTITY, Rot2::radians(PI + FRAC_PI_2))
            } else {
                (Rot2::radians(PI), Rot2::radians(PI + FRAC_PI_2))
            };
            (
                draw_line_data,
                DrawArcData {
                    isometry: Isometry2d {
                        rotation: isometry_rotation,
                        translation: Vec2::new(line_start.x, line_start.y + isometry_nudge.y),
                    },
                    arc_angle: arc_angle,
                    radius: radius.y,
                    color,
                },
                Vec2::new(line_start.x, line_start.y + isometry_nudge.y)
                    + endpoint_rotation * endpoint_nudge,
            )
        }
        CompassOctant::SouthEast => {
            let rotation = if mirror {
                Rot2::radians(-FRAC_PI_4)
            } else {
                Rot2::radians(FRAC_PI_2 + FRAC_PI_4)
            };
            (
                draw_line_data,
                DrawArcData {
                    isometry: Isometry2d {
                        rotation,
                        translation: Vec2::new(
                            line_start.x + isometry_nudge.y / SQRT_2,
                            line_start.y + isometry_nudge.y / SQRT_2,
                        ),
                    },
                    arc_angle: arc_angle,
                    radius: radius.y,
                    color,
                },
                Vec2::new(
                    line_start.x + isometry_nudge.y / SQRT_2,
                    line_start.y + isometry_nudge.y / SQRT_2,
                ) + Rot2::radians(PI + FRAC_PI_4) * endpoint_nudge,
            )
        }
        CompassOctant::South => {
            let rotation = if mirror {
                Rot2::radians(PI + FRAC_PI_2)
            } else {
                Rot2::radians(FRAC_PI_2)
            };
            (
                draw_line_data,
                DrawArcData {
                    isometry: Isometry2d {
                        rotation,
                        translation: Vec2::new(line_start.x + isometry_nudge.x, line_start.y),
                    },
                    arc_angle: arc_angle,
                    radius: radius.x,
                    color,
                },
                Vec2::new(line_start.x + isometry_nudge.x, line_start.y)
                    + Rot2::radians(PI) * endpoint_nudge,
            )
        }
        CompassOctant::SouthWest => {
            let rotation = if mirror {
                Rot2::radians(PI + FRAC_PI_4)
            } else {
                Rot2::radians(FRAC_PI_4)
            };
            (
                draw_line_data,
                DrawArcData {
                    isometry: Isometry2d {
                        rotation,
                        translation: Vec2::new(
                            line_start.x + isometry_nudge.x / SQRT_2,
                            line_start.y - isometry_nudge.x / SQRT_2,
                        ),
                    },
                    arc_angle: arc_angle,
                    radius: radius.x,
                    color,
                },
                Vec2::new(
                    line_start.x + isometry_nudge.x / SQRT_2,
                    line_start.y - isometry_nudge.x / SQRT_2,
                ) + Rot2::radians(FRAC_PI_2 + FRAC_PI_4) * endpoint_nudge,
            )
        }
        CompassOctant::West => {
            let rotation = if mirror {
                Rot2::radians(PI)
            } else {
                Rot2::IDENTITY
            };
            (
                draw_line_data,
                DrawArcData {
                    isometry: Isometry2d {
                        rotation,
                        translation: Vec2::new(line_start.x, line_start.y - isometry_nudge.y),
                    },
                    arc_angle: arc_angle,
                    radius: radius.y,
                    color,
                },
                Vec2::new(line_start.x, line_start.y - isometry_nudge.y)
                    + Rot2::radians(FRAC_PI_2) * endpoint_nudge,
            )
        }
        CompassOctant::NorthWest => {
            let rotation = if mirror {
                Rot2::radians(FRAC_PI_2 + FRAC_PI_4)
            } else {
                Rot2::radians(-FRAC_PI_4)
            };
            (
                draw_line_data,
                DrawArcData {
                    isometry: Isometry2d {
                        rotation,
                        translation: Vec2::new(
                            line_start.x - isometry_nudge.y / SQRT_2,
                            line_start.y - isometry_nudge.y / SQRT_2,
                        ),
                    },
                    arc_angle: arc_angle,
                    radius: radius.y,
                    color,
                },
                Vec2::new(
                    line_start.x - isometry_nudge.y / SQRT_2,
                    line_start.y - isometry_nudge.y / SQRT_2,
                ) + Rot2::radians(FRAC_PI_4) * endpoint_nudge,
            )
        }
    }
}

pub(crate) fn calculate_arc_mirror(
    (point, pos_data, dir_of_point): (Vec2, &NavVizPosData, CompassOctant),
    mirror: bool,
    color: Color,
    line_type: DrawLineType,
    arc_angle_from_pi_radians: f32,
    config: &AutoNavVizGizmoConfigGroup,
) -> (DrawLineData, DrawArcData, Vec2) {
    let line_start = Into::<Dir2>::into(dir_of_point).as_vec2() * config.arrow_tip_length + point;
    let draw_line_data = DrawLineData {
        start: line_start,
        end: point,
        color,
        line_type,
    };

    let radius = pos_data.obb_size / 12.;
    // this is correct with the orientation logic
    let arc_angle = PI + arc_angle_from_pi_radians;

    let isometry_nudge = -radius;
    let endpoint_nudge = isometry_nudge * Vec2::new(-ops::cos(arc_angle), ops::sin(arc_angle));
    // TODO the opposite end of the arc is shrinking, so it seems the arc anchor point needs to change?
    // The endpoint nudge seems to work correctly now. it's just that the 
    // arc edges need to be switched somehow.

    let neg_arc_angle = -arc_angle;
    match dir_of_point {
        CompassOctant::North => {
            let (isometry_rotation, endpoint_rotation) = (Rot2::radians(FRAC_PI_2), Rot2::radians(PI));
            (
                draw_line_data,
                DrawArcData {
                    isometry: Isometry2d {
                        rotation: isometry_rotation,
                        translation: Vec2::new(line_start.x - isometry_nudge.x, line_start.y),
                    },
                    arc_angle: neg_arc_angle,
                    radius: radius.x,
                    color,
                },
                Vec2::new(line_start.x - isometry_nudge.x, line_start.y)
                    + endpoint_rotation * endpoint_nudge,
            )
        }
        CompassOctant::NorthEast => {
            let (isometry_rotation, endpoint_rotation) =
                (Rot2::radians(FRAC_PI_4), Rot2::radians(FRAC_PI_2 + FRAC_PI_4));
            (
                draw_line_data,
                DrawArcData {
                    isometry: Isometry2d {
                        rotation: isometry_rotation,
                        translation: Vec2::new(
                            line_start.x - isometry_nudge.x / SQRT_2,
                            line_start.y + isometry_nudge.x / SQRT_2,
                        ),
                    },
                    arc_angle: neg_arc_angle,
                    radius: radius.x,
                    color,
                },
                Vec2::new(
                    line_start.x - isometry_nudge.x / SQRT_2,
                    line_start.y + isometry_nudge.x / SQRT_2,
                ) + endpoint_rotation * endpoint_nudge,
            )
        }
        CompassOctant::East => {
            let (isometry_rotation, endpoint_rotation) =
                (Rot2::IDENTITY, Rot2::radians(FRAC_PI_2));
            (
                draw_line_data,
                DrawArcData {
                    isometry: Isometry2d {
                        rotation: isometry_rotation,
                        translation: Vec2::new(line_start.x, line_start.y + isometry_nudge.y),
                    },
                    arc_angle: neg_arc_angle,
                    radius: radius.y,
                    color,
                },
                Vec2::new(line_start.x, line_start.y + isometry_nudge.y)
                    + endpoint_rotation * endpoint_nudge,
            )
        }
        CompassOctant::SouthEast => {
            let (isometry_rotation, endpoint_rotation) =
                { (Rot2::radians(-FRAC_PI_4), Rot2::radians(FRAC_PI_4)) };
            (
                draw_line_data,
                DrawArcData {
                    isometry: Isometry2d {
                        rotation: isometry_rotation,
                        translation: Vec2::new(
                            line_start.x + isometry_nudge.y / SQRT_2,
                            line_start.y + isometry_nudge.y / SQRT_2,
                        ),
                    },
                    arc_angle: neg_arc_angle,
                    radius: radius.y,
                    color,
                },
                Vec2::new(
                    line_start.x + isometry_nudge.y / SQRT_2,
                    line_start.y + isometry_nudge.y / SQRT_2,
                ) + endpoint_rotation * endpoint_nudge,
            )
        }
        CompassOctant::South => {
            let (isometry_rotation, endpoint_rotation) =
                (Rot2::radians(PI + FRAC_PI_2), Rot2::IDENTITY);
            (
                draw_line_data,
                DrawArcData {
                    isometry: Isometry2d {
                        rotation: isometry_rotation,
                        translation: Vec2::new(line_start.x + isometry_nudge.x, line_start.y),
                    },
                    arc_angle: neg_arc_angle,
                    radius: radius.x,
                    color,
                },
                Vec2::new(line_start.x + isometry_nudge.x, line_start.y)
                    + endpoint_rotation * endpoint_nudge,
            )
        }
        CompassOctant::SouthWest => {
            let (isometry_rotation, endpoint_rotation) = (
                Rot2::radians(PI + FRAC_PI_4),
                Rot2::radians(-FRAC_PI_4),
            );
            (
                draw_line_data,
                DrawArcData {
                    isometry: Isometry2d {
                        rotation: isometry_rotation,
                        translation: Vec2::new(
                            line_start.x + isometry_nudge.x / SQRT_2,
                            line_start.y - isometry_nudge.x / SQRT_2,
                        ),
                    },
                    arc_angle: neg_arc_angle,
                    radius: radius.x,
                    color,
                },
                Vec2::new(
                    line_start.x + isometry_nudge.x / SQRT_2,
                    line_start.y - isometry_nudge.x / SQRT_2,
                ) + endpoint_rotation * endpoint_nudge,
            )
        }
        CompassOctant::West => {
            let (isometry_rotation, endpoint_rotation) =
                (Rot2::radians(PI), Rot2::radians(PI + FRAC_PI_2 ));
            (
                draw_line_data,
                DrawArcData {
                    isometry: Isometry2d {
                        rotation: isometry_rotation,
                        translation: Vec2::new(line_start.x, line_start.y - isometry_nudge.y),
                    },
                    arc_angle: neg_arc_angle,
                    radius: radius.y,
                    color,
                },
                Vec2::new(line_start.x, line_start.y - isometry_nudge.y)
                    + endpoint_rotation * endpoint_nudge,
            )
        }
        CompassOctant::NorthWest => {
            let (isometry_rotation, endpoint_rotation) = (
                Rot2::radians(FRAC_PI_2 + FRAC_PI_4),
                Rot2::radians(PI + FRAC_PI_4),
            );
            (
                draw_line_data,
                DrawArcData {
                    isometry: Isometry2d {
                        rotation: isometry_rotation,
                        translation: Vec2::new(
                            line_start.x - isometry_nudge.y / SQRT_2,
                            line_start.y - isometry_nudge.y / SQRT_2,
                        ),
                    },
                    arc_angle: neg_arc_angle,
                    radius: radius.y,
                    color,
                },
                Vec2::new(
                    line_start.x - isometry_nudge.y / SQRT_2,
                    line_start.y - isometry_nudge.y / SQRT_2,
                ) + endpoint_rotation * endpoint_nudge,
            )
        }
    }
}
