//! # Bevy Auto Nav Viz
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
//! # use bevy_auto_nav_viz::AutoNavVizPlugin;
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
//! Once you have set the [`InputFocus`](bevy::input_focus::InputFocus) in the app
//! to a UI entity with the [`AutoDirectionalNavigation`] component, the [`AutoNavVizPlugin`]
//! will draw the navigation edges that exist with all other UI entities in the
//! same render target that have also opted in to [`AutoDirectionalNavigation`].
//!
//! # Configuration
//! The plugin can be configured via its gizmo config group [`AutoNavVizGizmoConfigGroup`].
//!
//! [`AutoDirectionalNavigation`]: bevy::ui::auto_directional_navigation::AutoDirectionalNavigation
//! ```
//! # use bevy::gizmos::config::GizmoConfigGroup;
//! # use bevy::prelude::*;
//! # use bevy_auto_nav_viz::{AutoNavVizGizmoConfigGroup, AutoNavVizDrawMode};
//! fn setup(mut config_store: ResMut<GizmoConfigStore>) {
//!     let mut config = config_store.config_mut::<AutoNavVizGizmoConfigGroup>().1;
//!     // e.g.
//!     config.draw_mode = AutoNavVizDrawMode::EnabledForCurrentFocus;
//! }
//! ```
//! Refer to the [`AutoNavVizGizmoConfigGroup`] for more information on all of the settings that can
//! be changed, or check out the `settings` example available in the repository.

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
pub enum AutoNavVizSystems {
    /// The [`rebuild_nav_viz_map`] system.
    BuildMap,
    /// The [`draw_nav_viz`] system.
    Draw,
}

/// A [`Plugin`] that adds visualizations for auto navigation systems.
#[derive(Default)]
pub struct AutoNavVizPlugin;

impl Plugin for AutoNavVizPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NavVizMap>()
            .init_gizmo_group::<AutoNavVizGizmoConfigGroup>()
            .add_systems(
                PostUpdate,
                (
                    nav_viz_map::rebuild_nav_viz_map.in_set(AutoNavVizSystems::BuildMap),
                    visualizer::draw_nav_viz.in_set(AutoNavVizSystems::Draw),
                )
                    .chain()
                    .after(TransformSystems::Propagate),
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
/// whole auto navigation graph is drawn. This will include edges that
/// may not be considered "symmetrical" by the navigation system itself,
/// but would be overlapping visually (i.e. a drawn edge between the
/// NE corner and NW corner of two nodes).
#[derive(Clone, Copy, Debug, Default, Reflect, PartialEq)]
pub enum SymmetricalEdgeSettings {
    /// Merge the two single ended arrows into one drawn double ended arrow.
    /// The color of the merged arrow is a gradient between the colors that
    /// the arrows would have had as single arrows. The color of each single arrow
    /// is more prominent closer to their respective source entity in the combined arrow.
    #[default]
    MergeAndGradient,

    /// Merge the two single ended arrows into one drawn double ended arrow.
    /// The f32 provided is the mixing factor between the colors of the two single ended arrows.
    /// It should be a number between 0.0 and 1.0 as defined by [`Color::mix`], where "this color"
    /// refers to the color of the arrow from the source entity to the destination entity.
    MergeAndMix(f32),

    /// Draw two single ended arrows with spacing inbetween the arrows.
    /// The spacing is automatically calculated using the [`AutoNavVizGizmoConfigGroup`]'s
    /// arrow tip length ([`AutoNavVizGizmoConfigGroup::get_nudge_units`]).
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

    /// Returns whether this [`SymmetricalEdgeSettings`] is a Merge* variant.
    pub fn is_merge(&self) -> bool {
        matches!(self, Self::MergeAndGradient | Self::MergeAndMix(_))
    }

    /// Returns whether this [`SymmetricalEdgeSettings`] is
    /// [`SymmetricalEdgeSettings::OverlappingSingleArrows`]
    pub fn is_overlap(&self) -> bool {
        *self == Self::OverlappingSingleArrows
    }
}

/// Setting for whether the directional colors provided by the [`AutoNavVizGizmoConfigGroup`]
/// should:
/// - be mixed with a color generated uniquely for the source entity,
/// - be mixed with a color generated uniquely for the destination entity,
/// - not be mixed
///
/// The default is not to mix.
#[derive(Clone, Copy, Debug, Default, Reflect, PartialEq)]
pub enum AutoNavVizColorMode {
    /// Mix the directional color with a color generated uniquely for the source entity.
    /// The f32 provided is the mixing factor and should be a number between 0.0 and 1.0.
    /// A higher number means that the entity's color is used more.
    MixedWithSourceEntity(f32),
    /// Mix the directional color with a color generated uniquely for the destination entity.
    /// The f32 provided is the mixing factor and should be a number between 0.0 and 1.0.
    /// A higher number means that the entity's color is used more.
    MixedWithDestinationEntity(f32),
    #[default]
    DirectionalOnly,
}

impl AutoNavVizColorMode {
    /// Returns [`AutoNavVizColorMode::MixedWithSourceEntity`] with a mix factor of 0.5
    /// (equal mixing of the entity's color and the direction)
    pub fn mix_with_source_entity_evenly() -> Self {
        Self::MixedWithSourceEntity(0.5)
    }

    /// Returns [`AutoNavVizColorMode::MixedWithSourceEntity`] with a mix factor of 1.0
    /// (only the source entity's color is used)
    pub fn source_entity_color_only() -> Self {
        Self::MixedWithSourceEntity(1.)
    }

    /// Returns [`AutoNavVizColorMode::MixedWithDestinationEntity`] with a mix factor of 0.5
    /// (equal mixing of the entity's color and the direction)
    pub fn mix_with_destination_entity_evenly() -> Self {
        Self::MixedWithDestinationEntity(0.5)
    }

    /// Returns [`AutoNavVizColorMode::MixedWithDestinationEntity`] with a mix factor of 1.0
    /// (only the destination entity's color is used)
    pub fn destination_entity_color_only() -> Self {
        Self::MixedWithDestinationEntity(1.)
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
    pub draw_mode: AutoNavVizDrawMode,

    /// The coloring mode
    /// See [`AutoNavVizColorMode`] for more details.
    pub color_mode: AutoNavVizColorMode,

    /// Determines the arrow tip length for directional arrow in world coordinates.
    /// This also determines the amount of nudge used with
    /// [`SymmetricalEdgeSettings::SpacingBetweenSingleArrows`] and the radius
    /// of the pair of arcs drawn for looped edges.
    pub arrow_tip_length: f32,

    /// A color representing one of the eight [`CompassOctant`] directions
    /// that the auto navigation system uses.
    ///
    /// If set to None, the default directional color `default_color` will be used.
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

    /// The default color to be used if a direction's color is set to None.
    pub default_color: Color,
}

impl Default for AutoNavVizGizmoConfigGroup {
    fn default() -> Self {
        Self {
            draw_mode: Default::default(),
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

            // White
            default_color: Color::Srgba(Srgba::WHITE),
        }
    }
}

impl AutoNavVizGizmoConfigGroup {
    /// Toggles the draw mode, using the default values for each draw mode.
    pub fn toggle_draw_mode(&mut self) {
        if self.draw_mode == AutoNavVizDrawMode::EnabledForCurrentFocus {
            self.draw_mode = AutoNavVizDrawMode::EnabledForAll(SymmetricalEdgeSettings::default());
        } else {
            self.draw_mode = AutoNavVizDrawMode::EnabledForCurrentFocus;
        }
    }

    /// Returns the color set in this config group for the given direction.
    pub fn get_setting_color_for_direction(&self, dir: CompassOctant) -> Option<Color> {
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

    /// Gets the color given the source entity's color, destination entity's color,
    /// and the direction of navigation. They will be mixed depending on how the
    /// color mode is configured.
    pub fn get_color_for_direction(
        &self,
        source_color: Color,
        destination_color: Color,
        dir: CompassOctant,
    ) -> Color {
        let color = self
            .get_setting_color_for_direction(dir)
            .unwrap_or(self.default_color);
        match self.color_mode {
            AutoNavVizColorMode::MixedWithSourceEntity(factor) => color.mix(&source_color, factor),
            AutoNavVizColorMode::MixedWithDestinationEntity(factor) => {
                color.mix(&destination_color, factor)
            }
            AutoNavVizColorMode::DirectionalOnly => color,
        }
    }

    /// Returns the nudge units used when the Draw Mode is set to
    /// [`AutoNavVizDrawMode::EnabledForAll`] with symmetrical edge settings
    /// [`SymmetricalEdgeSettings::SpacingBetweenSingleArrows`]. The value returned
    /// is half of the `arrow_tip_length`.
    pub fn get_nudge_units(&self) -> Vec2 {
        Vec2::splat(self.arrow_tip_length / 2.)
    }

    /// Returns the arc radius for looped edges: 0.75 the value of `arrow_tip_length`.
    pub fn get_arc_radius(&self) -> Vec2 {
        Vec2::splat(self.arrow_tip_length * 0.75)
    }
}
