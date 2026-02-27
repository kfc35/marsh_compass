//! # Marsh Compass
//! A Bevy Plugin that draws a visualization of the auto directional navigation
//! system in Bevy's UI Framework.
//!
//! # Usage
//! Simply add the [`AutoNavVizPlugin`] plugin to your app that has auto
//! directional navigation enabled setup.
//!
//! ```no_run
//! # use bevy::prelude::*;
//! # use bevy::input_focus::{
//! #   InputDispatchPlugin,
//! #   directional_navigation::DirectionalNavigationPlugin
//! # };
//! # use marsh_compass::AutoNavVizPlugin;
//! fn main() {
//!     App::new()
//!         .add_plugins((
//!             DefaultPlugins,
//!             InputDispatchPlugin, // Needed for input focus
//!             DirectionalNavigationPlugin, // Needed for auto directional nav
//!             AutoNavVizPlugin // Add this plugin
//!         ))
//!         .run();
//! }
//! ```
//!
//! # Configuration
//! The plugin can be configured via its gizmo config group [`AutoNavVizGizmoConfigGroup`].
//! ```
//! # use bevy::gizmos::config::GizmoConfigGroup;
//! # use bevy::prelude::*;
//! # use marsh_compass::{AutoNavVizGizmoConfigGroup, AutoNavVizDrawMode};
//! fn setup(mut config_store: ResMut<GizmoConfigStore>) {
//!     let mut config = config_store.config_mut::<AutoNavVizGizmoConfigGroup>().1;
//!     // e.g.
//!     config.drawing_mode = AutoNavVizDrawMode::EnabledForCurrentFocus;
//! }
//! ```

use bevy::gizmos::config::GizmoConfigGroup;
use bevy::math::CompassOctant;
use bevy::prelude::*;

mod nav_viz_map;
pub use nav_viz_map::*;
mod visualizer;
pub use visualizer::*;

/// System set for the visualization systems executed in the [`AutoNavVizPlugin`].
///
/// This system set runs in the [`PostUpdate`] schedule after [`TransformSystems::Propagate`].
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AutoNavVizSystems;

/// A [`Plugin`] that adds visualizations for auto navigation systems.
#[derive(Default)]
pub struct AutoNavVizPlugin;

impl Plugin for AutoNavVizPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NavVizMap>()
            .init_gizmo_group::<AutoNavVizGizmoConfigGroup>()
            .add_systems(
                PostUpdate,
                (nav_viz_map::rebuild_nav_viz_map, visualizer::draw_nav_viz)
                    .chain()
                    .after(TransformSystems::Propagate)
                    .in_set(AutoNavVizSystems),
            );
    }
}

/// Setting for whether the navigation visualization should be:
/// - drawn for the current focus only, or
/// - drawn for all [`AutoDirectionalNavigation`](bevy::ui::auto_directional_navigation::AutoDirectionalNavigation)
///   entities rendered to the same target as the current focus.
///
/// The default is to draw for the current focus only.
#[derive(Clone, Copy, Default, Debug, Reflect, PartialEq)]
pub enum AutoNavVizDrawMode {
    #[default]
    EnabledForCurrentFocus,
    EnabledForAll(SymmetricalEdgeSettings),
}

/// Setting for how to render symmetrical navigation edges when the
/// whole auto navigation graph is drawn.
#[derive(Clone, Copy, Debug, Default, Reflect, PartialEq)]
pub enum SymmetricalEdgeSettings {
    /// Merge the two single ended arrows into one drawn double ended arrow.
    /// The color of the merged arrow is a gradient between the colors that
    /// the arrows would have had as single arrows. The color of the single arrow
    /// shows closer to its source entity.
    #[default]
    MergeAndGradient,

    /// Merge the two single ended arrows into one drawn double ended arrow.
    /// The f32 provided is the mixing factor between the colors of the two single ended arrows.
    /// It should be a number between 0.0 and 1.0 as defined by [`Color::mix`], where "this color"
    /// refers to the color of the arrow from the source entity to the destination entity.
    MergeAndMix(f32),

    /// Draw two single ended arrows with spacing inbetween the arrows.
    /// The spacing is automatically calculated by the length and height of the entities.
    SpacingBetweenSingleArrows,

    /// Draw two single ended arrows that are simply drawn over each other.
    /// No merging is done between the two arrows.
    OverlappingSingleArrows,
}

impl SymmetricalEdgeSettings {
    /// Returns [`SymmetricalEdgeSettings::MergeAndMix`] with a mix factor of 0.5
    /// (equal mixing of the colors of the two individual arrows)
    pub fn merge_and_mix_evenly() -> Self {
        Self::MergeAndMix(0.5)
    }

    /// Returns whether this `SymmetricalEdgeSettings` is a Merge* variant.
    pub fn is_merge(&self) -> bool {
        matches!(self, Self::MergeAndGradient | Self::MergeAndMix(_))
    }
}

/// Setting for whether the directional colors provided by the [`AutoNavVizGizmoConfigGroup`]
/// should:
/// - be mixed with a random color generated uniquely for the source entity, or
/// - not be mixed
///
/// The default is not to mix.
#[derive(Clone, Copy, Debug, Default, Reflect, PartialEq)]
pub enum AutoNavVizColorMode {
    /// Mix the color with a random color generated uniquely for the source entity.
    /// The f32 provided is the mixing factor and should be a number between 0.0 and 1.0.
    /// A higher number means that the entity's color is used more.
    MixWithEntity(f32),
    #[default]
    NoMix,
}

impl AutoNavVizColorMode {
    /// Returns [`AutoNavVizColorMode::MixWithEntity`] with a mix factor of 0.5
    /// (equal mixing of the entity's color and the direction)
    pub fn mix_with_entity_evenly() -> Self {
        Self::MixWithEntity(0.5)
    }
}

/// The [`GizmoConfigGroup`] for the auto navigation visualizations
/// that will be drawn.
///
/// As with other custom gizmo config groups, the standard settings to disable drawing,
/// change line width, etc. can be updated via this gizmo's group's
/// [`GizmoConfig`], accessible as the first item in the tuple returned by
/// [`GizmoConfigStore::config<AutoNavVizGizmoConfigGroup>`].
#[derive(Reflect, GizmoConfigGroup)]
pub struct AutoNavVizGizmoConfigGroup {
    /// The drawing mode
    /// See [`AutoNavVizDrawMode`] for more details.
    pub drawing_mode: AutoNavVizDrawMode,

    /// The coloring mode
    /// See [`AutoNavVizColorMode`] for more details.
    pub color_mode: AutoNavVizColorMode,

    // TODO only manual edges, only automated edges, or both.
    /// Determines the arrow tip length for directional arrow in world coordinates.
    pub arrow_tip_length: f32,

    /// A color representing one of the eight [`CompassOctant`] directions
    /// that the auto navigation system uses.
    ///
    /// If set to None, the random color generated uniquely for the source entity will be used only.
    pub north_color: Option<Color>,

    /// Refer to the description of [north_color](AutoNavVizGizmoConfigGroup::north_color).
    pub north_east_color: Option<Color>,

    /// Refer to the description of [north_color](AutoNavVizGizmoConfigGroup::north_color).
    pub east_color: Option<Color>,

    /// Refer to the description of [north_color](AutoNavVizGizmoConfigGroup::north_color).
    pub south_east_color: Option<Color>,

    /// Refer to the description of [north_color](AutoNavVizGizmoConfigGroup::north_color).
    pub south_color: Option<Color>,

    /// Refer to the description of [north_color](AutoNavVizGizmoConfigGroup::north_color).
    pub south_west_color: Option<Color>,

    /// Refer to the description of [north_color](AutoNavVizGizmoConfigGroup::north_color).
    pub west_color: Option<Color>,

    /// Refer to the description of [north_color](AutoNavVizGizmoConfigGroup::north_color).
    pub north_west_color: Option<Color>,
}

impl Default for AutoNavVizGizmoConfigGroup {
    fn default() -> Self {
        Self {
            drawing_mode: Default::default(),
            color_mode: Default::default(),
            arrow_tip_length: 10.,
            // Yellow
            north_color: Some(Color::Srgba(Srgba::new(1.0, 1.0, 0., 0.8))),

            // Orange
            north_east_color: Some(Color::Srgba(Srgba::new(1.0, 0.33, 0., 0.8))),

            // Magenta
            east_color: Some(Color::Srgba(Srgba::new(1.0, 0., 0.5, 0.8))),

            // Purple
            south_east_color: Some(Color::Srgba(Srgba::new(0.67, 0., 1.0, 0.8))),

            // Blue
            south_color: Some(Color::Srgba(Srgba::new(0., 0., 1., 0.8))),

            // Cyan
            south_west_color: Some(Color::Srgba(Srgba::new(0., 0.67, 1.0, 0.8))),

            // Green
            west_color: Some(Color::Srgba(Srgba::new(0., 1.0, 0.5, 0.8))),

            // Light Green
            north_west_color: Some(Color::Srgba(Srgba::new(0.33, 1.0, 0., 0.8))),
        }
    }
}

impl AutoNavVizGizmoConfigGroup {
    /// Toggles the draw mode, using the default values for each draw mode.
    pub fn toggle_draw_mode(&mut self) {
        if self.drawing_mode == AutoNavVizDrawMode::EnabledForCurrentFocus {
            self.drawing_mode =
                AutoNavVizDrawMode::EnabledForAll(SymmetricalEdgeSettings::default());
        } else {
            self.drawing_mode = AutoNavVizDrawMode::EnabledForCurrentFocus;
        }
    }

    /// Returns the color set in this config group for the given direction.
    pub fn get_color_for_direction(&self, dir: CompassOctant) -> Option<Color> {
        match dir {
            CompassOctant::North => self.north_color,
            CompassOctant::NorthEast => self.north_east_color,
            CompassOctant::East => self.east_color,
            CompassOctant::SouthEast => self.south_east_color,
            CompassOctant::South => self.south_color,
            CompassOctant::SouthWest => self.south_west_color,
            CompassOctant::West => self.west_color,
            CompassOctant::NorthWest => self.north_west_color,
        }
    }

    /// Sets all the directional colors to None. This means that the all colors
    /// used for arrows will be unique to the source entity.
    pub fn set_directional_colors_to_none(&mut self) {
        self.north_color = None;
        self.north_east_color = None;
        self.east_color = None;
        self.south_east_color = None;
        self.south_color = None;
        self.south_west_color = None;
        self.west_color = None;
        self.north_west_color = None;
    }

    /// Sets all directional colors back to their defaults.
    pub fn set_directional_colors_to_defaults(&mut self) {
        let default = Self::default();
        self.north_color = default.north_color;
        self.north_east_color = default.north_east_color;
        self.east_color = default.east_color;
        self.south_east_color = default.south_east_color;
        self.south_color = default.south_color;
        self.south_west_color = default.south_west_color;
        self.west_color = default.west_color;
        self.north_west_color = default.north_west_color;
    }

    /// Gets the color of the arrow given the source entity and the direction of navigation.
    /// They will be mixed depending on how the color mode is configured.
    pub fn get_color_for_entity_and_direction(
        &self,
        entity_color: Color,
        dir: CompassOctant,
    ) -> Color {
        self.get_color_for_direction(dir)
            .map(|color| {
                if let AutoNavVizColorMode::MixWithEntity(factor) = self.color_mode {
                    color.mix(&entity_color, factor)
                } else {
                    color
                }
            })
            .unwrap_or(entity_color)
    }
}
