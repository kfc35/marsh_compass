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
//!     config.drawing_mode = AutoNavVizDrawMode::EnabledForAll;
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
#[derive(Clone, Default, Debug, Reflect, PartialEq, Eq, Hash)]
pub enum AutoNavVizDrawMode {
    #[default]
    EnabledForCurrentFocus,
    EnabledForAll,
}

/// Setting for whether the directional colors provided by the [`AutoNavVizGizmoConfigGroup`]
/// should:
/// - be mixed with a random color generated uniquely for the source entity, or
/// - not be mixed
///
/// The default is not to mix.
#[derive(Clone, Debug, Default, Reflect, PartialEq)]
pub enum AutoNavVizColorMode {
    /// Mix the color with a random color generated uniquely for the source entity.
    /// The f32 provided is the mixing factor and should be a number between 0.0 and 1.0.
    /// A higher number means that the entity's color is used more.
    MixWithEntity(f32),
    #[default]
    NoMix,
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

    /// The amount of units (in world space) to render between arrows that would otherwise
    /// overlap. This is mostly used when navigation edges are symmetrical, but can also be used
    /// depending on how the visualization system calculates how best to place arrows to avoid
    /// awkward or confusing placement.
    pub symmetrical_edge_spacing: f32,

    /// For visualizations that need a 180 degree arc to rotate direction, this denotes
    /// how wide the radius is in world coordinates.
    pub arc_radius: f32,

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
            symmetrical_edge_spacing: 10.,
            arc_radius: 10.,
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
}
