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
    let (start_line, start_arc, line_start) = calculate_arc(
        (start_point, from_pos_data.obb_size, start_point_dir),
        get_angle_from_pi_rotation(start_point, start_point_dir, end_point),
        false,
        override_color.unwrap_or(from_color),
        start_line_line_type,
        config,
    );

    // The ending arc should always end in an arrow
    let (end_line, end_arc, line_end) = calculate_arc(
        (end_point, to_pos_data.obb_size, end_point_dir),
        get_angle_from_pi_rotation(end_point, end_point_dir, start_point),
        true,
        override_color.unwrap_or(to_color),
        DrawLineType::Arrow,
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

/// Ensuring the arc radius is some fraction of size ensures that
/// multiple consecutive looping edges are spaced out visually when
/// approaching near entities. Along a side, we must accommodate at most
/// 3 drawn arcs. Since the arcs can be mirrored, we should accommodate
/// 6 arcs per side to account for these permutations.
/// The radius length is 1/2 the arc diameter. So, the radius must be
/// at most 1/12 the length of a side.
const RADIUS_LOCAL_SIZE_PROPORTION: f32 = 1. / 12.;

/// Helper function for creating [`DrawLoopedLineData`](crate::DrawLoopedLineData).
/// This returns data for drawing the line/arrow that loops out from/
/// in to `point`, the semi-circle arc connected to this line/arrow, and the
/// world coordinate of an endpoint for the connecting line between this arc
/// and its opposite.
///
/// `point` lies in the direction of `dir_of_point` on the rectangle.
/// The arc and the line/arrow are drawn with the provided `color`.
/// `angle_from_pi_radians` is the deviation from a semi-circle arc
/// (arc with arc_length PI), for when the destination point is not directly
/// opposite the given `point`.
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
fn calculate_arc(
    (point, local_size, dir_of_point): (Vec2, Vec2, CompassOctant),
    angle_from_pi_radians: f32,
    mirror: bool,
    color: Color,
    line_type: DrawLineType,
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
    let radius = local_size * RADIUS_LOCAL_SIZE_PROPORTION;

    // The mirror side's arc_angle should increase as the normal side's decreases,
    // and vice versa (as the endpoints become heavily misaligned, one arc angle
    // heavily curve towards the other, while the other arc angle becomes shallower
    // to compensate). When the angle is 0., this means that the endpoints
    // are perfectly in the opposite direction of each other.
    //
    // The arc's center will always be translated from line_start
    // by a radius length. However, the mirror's center
    // should be translated in the opposite way per compass direction.
    let (angle, arc_translation) = if mirror {
        (PI - angle_from_pi_radians, -radius)
    } else {
        (PI + angle_from_pi_radians, radius)
    };

    // Finds the placement of a point along the circle
    // It needs to be translated to where the arc is and then rotated to be
    // oriented to the created arc, hence "relative"
    let find_endpoint_of_arc = |radius: f32| -> Vec2 {
        if mirror {
            // This traverses the mirror side's arc clockwise starting from `-Vec2::X`
            radius * Vec2::new(-ops::cos(angle), ops::sin(angle))
        } else {
            // This traverses the regular side's arc counterclockwise starting from `Vec2::X`
            radius * Vec2::new(ops::cos(angle), ops::sin(angle))
        }
    };

    let arc_angle = if mirror {
        // The mirror side's arc is drawn clockwise from its starting point
        // so it must be negated
        -angle
    } else {
        angle
    };

    // For reasons why these are set at the given values, it helps to look at the spec for
    // Gizmos arc_2d:
    //    - `isometry` defines the translation and rotation of the arc.
    //    - the translation specifies the center of the arc
    //    - the rotation is counter-clockwise starting from `Vec2::Y`
    // The isometry_rotation tells you where the arc will start from.
    // The isometry_translation gives you the center of the arc.
    //
    // The arc_endpoint_rotation will rotate `relative_endpoint_of_arc` so that
    // it aligns with the start of the arc, allowing the endpoint to be
    // correctly calculated.
    //
    // In general, the isometry_translation is shifted CCW for regular, and
    // CW for mirrored.
    let (isometry_rotation, isometry_translation, arc_radius, arc_endpoint_rotation) =
        match dir_of_point {
            CompassOctant::North => {
                // To explain these values:
                //
                // The regular arc is rotated so that it starts from `Vec2::X` (a counter-clockwise
                // rotation of 3 PI / 2, or 270 degrees from `Vec2::Y`)
                // The regular arc's center is shifted one radius west from line_start.
                // The regular arc's arc_endpoint_rotation is IDENTITY since `relative_endpoint_of_arc`
                // already starts at `Vec2::X`.
                //
                // For the mirror side:
                // The mirrored arc is rotated so that it starts from `-Vec2::X` (a CCR of PI / 2,
                // or 90 degrees from `Vec2::Y`). This is equal to taking the regular arc's rotation
                // and rotating it clockwise by PI (or 180 degrees).
                // The mirrored arc's center is shifted one radius east from the line_start
                // (arc_translation has already been negated).
                // The mirrored arc's arc_endpoint_rotation is IDENTITY since `relative_endpoint_of_arc`
                // already starts at `-Vec2::X`.
                //
                // This pattern is used for all the compass directions, shifting PI/4 (or 45 degrees)
                // each successive time.
                //
                // The radius is just the abs value of the arc_translation coordinate used.
                let mut isometry_rotation = Rot2::radians(PI + FRAC_PI_2);
                if mirror {
                    isometry_rotation *= Rot2::radians(-PI)
                }
                let isometry_translation =
                    Vec2::new(line_start.x - arc_translation.x, line_start.y);
                let arc_endpoint_rotation = Rot2::IDENTITY;
                (
                    isometry_rotation,
                    isometry_translation,
                    radius.x,
                    arc_endpoint_rotation,
                )
            }
            CompassOctant::NorthEast => {
                // Compared to north, all rotations are decreased by FRAC_PI_4, which
                // correspond to a clockwise rotation.
                // This makes sense since north-east is FRAC_PI_4 radians CW from north.
                let mut isometry_rotation = Rot2::radians(PI + FRAC_PI_4);
                if mirror {
                    isometry_rotation *= Rot2::radians(-PI)
                }
                // The arc's center is shifted west and north (mirrored: east and south).
                // We use arc_translation.x here for both to ensure the math works out.
                // This corresponds to a radius of length ops::abs(arc_translation.x), aka radius.x
                let isometry_translation = Vec2::new(
                    line_start.x - arc_translation.x / SQRT_2,
                    line_start.y + arc_translation.x / SQRT_2,
                );
                let arc_endpoint_rotation = Rot2::radians(-FRAC_PI_4);
                (
                    isometry_rotation,
                    isometry_translation,
                    radius.x,
                    arc_endpoint_rotation,
                )
            }
            CompassOctant::East => {
                // Once again, rotations decreased by PI/4. And so on for the rest...
                let mut isometry_rotation = Rot2::radians(PI);
                if mirror {
                    isometry_rotation *= Rot2::radians(-PI)
                }
                // The arc's center is only shifted north (mirrored: south). It uses arc_translation.y this time,
                // so the arc_radius is radius.y. And so on for the rest where applicable...
                let isometry_translation =
                    Vec2::new(line_start.x, line_start.y + arc_translation.y);
                // Note that a rotation by -FRAC_PI_2 is equivalent to
                // a rotation by PI + FRAC_PI_2
                let arc_endpoint_rotation = Rot2::radians(PI + FRAC_PI_2);

                (
                    isometry_rotation,
                    isometry_translation,
                    radius.y,
                    arc_endpoint_rotation,
                )
            }
            CompassOctant::SouthEast => {
                let mut isometry_rotation = Rot2::radians(FRAC_PI_2 + FRAC_PI_4);
                if mirror {
                    isometry_rotation *= Rot2::radians(-PI)
                }
                let isometry_translation = Vec2::new(
                    line_start.x + arc_translation.y / SQRT_2,
                    line_start.y + arc_translation.y / SQRT_2,
                );
                let arc_endpoint_rotation = Rot2::radians(PI + FRAC_PI_4);

                (
                    isometry_rotation,
                    isometry_translation,
                    radius.y,
                    arc_endpoint_rotation,
                )
            }
            CompassOctant::South => {
                let mut isometry_rotation = Rot2::radians(FRAC_PI_2);
                if mirror {
                    isometry_rotation *= Rot2::radians(-PI)
                }
                let isometry_translation =
                    Vec2::new(line_start.x + arc_translation.x, line_start.y);
                let arc_endpoint_rotation = Rot2::radians(PI);
                (
                    isometry_rotation,
                    isometry_translation,
                    radius.x,
                    arc_endpoint_rotation,
                )
            }
            CompassOctant::SouthWest => {
                let mut isometry_rotation = Rot2::radians(FRAC_PI_4);
                if mirror {
                    isometry_rotation *= Rot2::radians(-PI)
                }
                let isometry_translation = Vec2::new(
                    line_start.x + arc_translation.x / SQRT_2,
                    line_start.y - arc_translation.x / SQRT_2,
                );
                let arc_endpoint_rotation = Rot2::radians(FRAC_PI_2 + FRAC_PI_4);
                (
                    isometry_rotation,
                    isometry_translation,
                    radius.x,
                    arc_endpoint_rotation,
                )
            }
            CompassOctant::West => {
                let mut isometry_rotation = Rot2::IDENTITY;
                if mirror {
                    isometry_rotation *= Rot2::radians(-PI)
                }
                let isometry_translation =
                    Vec2::new(line_start.x, line_start.y - arc_translation.y);
                let arc_endpoint_rotation = Rot2::radians(FRAC_PI_2);
                (
                    isometry_rotation,
                    isometry_translation,
                    radius.y,
                    arc_endpoint_rotation,
                )
            }
            CompassOctant::NorthWest => {
                let mut isometry_rotation = Rot2::radians(-FRAC_PI_4);
                if mirror {
                    isometry_rotation *= Rot2::radians(-PI)
                }
                let isometry_translation = Vec2::new(
                    line_start.x - arc_translation.y / SQRT_2,
                    line_start.y - arc_translation.y / SQRT_2,
                );
                let arc_endpoint_rotation = Rot2::radians(FRAC_PI_4);
                (
                    isometry_rotation,
                    isometry_translation,
                    radius.y,
                    arc_endpoint_rotation,
                )
            }
        };
    (
        draw_line_data,
        DrawArcData {
            isometry: Isometry2d {
                rotation: isometry_rotation,
                translation: isometry_translation,
            },
            arc_angle,
            radius: arc_radius,
            color,
        },
        // To calculate the final endpoint:
        // - go to where the center of the arc is (`isometry_translation`)
        // - orient correctly (`arc_endpoint_rotation`)
        // - then follow the arc for the correct angle length via `relative_endpoint_of_arc`
        isometry_translation + arc_endpoint_rotation * (find_endpoint_of_arc(arc_radius)),
    )
}

/// This function returns the angle between:
/// - The vector that represents `dir_from_start.opposite()`
/// - The vector from `from_point` to `to_point`
///
/// Since `dir_from_start.opposite()` is equivalent to a rotation by `PI`, this
/// angle is returned relative to that rotation.
///
/// This is eventually used to figure out the curvature of the arc that is created near `start`.
/// The signed dot product will be used to adjust how much of the semi-circle arc (arc_angle = PI)
/// is drawn.
fn get_angle_from_pi_rotation(
    start: Vec2,
    dir_from_start_to_rotate: CompassOctant,
    end: Vec2,
) -> f32 {
    let pi_rotation = dir_from_start_to_rotate.opposite();
    let start_to_end_dir = (end - start).normalize();
    let mut radians_from_pi_arc = ops::acos(
        Into::<Dir2>::into(pi_rotation)
            .as_vec2()
            .dot(start_to_end_dir),
    );

    // Since the dot product is always positive, we need to figure out
    // the orientation of this difference from `pi_rotation`
    // For that, we use the cross product.
    // A positive dot product will be associated with a negative angle.
    let orientation = start_to_end_dir.perp_dot(Into::<Dir2>::into(pi_rotation).as_vec2());
    if orientation > 0. {
        radians_from_pi_arc *= -1.;
    }

    radians_from_pi_arc
}

#[cfg(test)]
mod tests {
    use std::f32::consts::FRAC_PI_2;

    use super::*;

    fn assert_eq_f32(left: f32, right: f32) {
        let difference = left - right;
        assert!(
            ops::abs(difference) <= 1e-6,
            "left: {}\n right: {}",
            left,
            right
        );
    }

    #[test]
    fn test_get_angle_from_pi_rotation() {
        let angle = get_angle_from_pi_rotation(
            Vec2::ZERO,
            CompassOctant::North,
            Into::<Dir2>::into(CompassOctant::East).as_vec2(),
        );
        assert_eq_f32(angle, FRAC_PI_2);

        let angle = get_angle_from_pi_rotation(
            Vec2::ZERO,
            CompassOctant::North,
            Into::<Dir2>::into(CompassOctant::SouthEast).as_vec2(),
        );
        assert_eq_f32(angle, FRAC_PI_4);

        let angle = get_angle_from_pi_rotation(
            Vec2::ZERO,
            CompassOctant::North,
            Into::<Dir2>::into(CompassOctant::South).as_vec2(),
        );
        assert_eq_f32(angle, 0.);

        let angle = get_angle_from_pi_rotation(
            Vec2::ZERO,
            CompassOctant::North,
            Into::<Dir2>::into(CompassOctant::SouthWest).as_vec2(),
        );
        assert_eq_f32(angle, -FRAC_PI_4);

        let angle = get_angle_from_pi_rotation(
            Vec2::ZERO,
            CompassOctant::North,
            Into::<Dir2>::into(CompassOctant::West).as_vec2(),
        );
        assert_eq_f32(angle, -FRAC_PI_2);
    }

    #[test]
    fn test_calculate_arc() {
        let mut config = AutoNavVizGizmoConfigGroup::default();
        config.arrow_tip_length = 5.;

        // Straightforward Test
        let (line, arc, endpoint) = calculate_arc(
            (Vec2::ZERO, Vec2::new(12., 18.), CompassOctant::North),
            0.,
            false,
            Color::Srgba(Srgba::RED),
            DrawLineType::Arrow,
            &config,
        );
        // arrow tip length shifts the start north
        let expected_arc_start = Vec2::new(0., 5.);
        assert_eq!(line.start, expected_arc_start);
        assert_eq!(line.end, Vec2::ZERO);
        assert_eq!(line.color, Color::Srgba(Srgba::RED));
        assert_eq!(line.line_type, DrawLineType::Arrow);

        let expected_radius = 1.; // 12. * RADIUS_LOCAL_SIZE_PROPORTION
        assert_eq!(
            arc.isometry.translation,
            expected_arc_start + Vec2::new(-expected_radius, 0.)
        );
        assert_eq!(arc.isometry.rotation, Rot2::radians(PI + FRAC_PI_2));
        assert_eq!(arc.radius, expected_radius);
        assert_eq!(arc.arc_angle, PI);
        assert_eq!(arc.color, Color::Srgba(Srgba::RED));

        assert_eq!(
            endpoint,
            expected_arc_start + 2. * Vec2::new(-expected_radius, 0.)
        );

        // Mirrored test with angle
        let (line, arc, endpoint) = calculate_arc(
            (Vec2::ZERO, Vec2::new(12., 18.), CompassOctant::East),
            FRAC_PI_4,
            true,
            Color::Srgba(Srgba::RED),
            DrawLineType::Arrow,
            &config,
        );
        // arrow tip length shifts the start north
        let expected_arc_start = Vec2::new(5., 0.);
        assert_eq!(line.start, expected_arc_start);
        assert_eq!(line.end, Vec2::ZERO);
        assert_eq!(line.color, Color::Srgba(Srgba::RED));
        assert_eq!(line.line_type, DrawLineType::Arrow);

        let expected_radius = 1.5; // 18. * RADIUS_LOCAL_SIZE_PROPORTION
        assert_eq!(
            arc.isometry.translation,
            expected_arc_start + Vec2::new(0., -expected_radius)
        );
        assert_eq!(arc.isometry.rotation, Rot2::IDENTITY);
        assert_eq!(arc.radius, expected_radius);
        // it's a negative number because it is mirrored.
        // the angle is subtracted from PI because it is mirrored.
        assert_eq!(arc.arc_angle, -(PI - FRAC_PI_4));
        assert_eq!(arc.color, Color::Srgba(Srgba::RED));

        // translate to the center of the arc and then add the relative point along the arc
        assert_eq!(
            endpoint,
            expected_arc_start
                + Vec2::new(0., -expected_radius)
                + Rot2::radians(PI + FRAC_PI_2)
                    * (expected_radius * Vec2::new(1. / SQRT_2, 1. / SQRT_2))
        );
    }
}
