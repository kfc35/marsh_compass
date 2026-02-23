use bevy::ecs::entity::EntityHashMap;
use bevy::input_focus::InputFocus;
use bevy::input_focus::directional_navigation::{AutoNavigationConfig, DirectionalNavigationMap};
use bevy::prelude::*;
use bevy::ui::auto_directional_navigation::AutoDirectionalNavigation;

mod viz_data;
pub use crate::nav_viz_map::viz_data::NavVizData;

mod nav_map;
mod navigable_nodes;

/// Resource used to cache the complete navigation map: auto navigation edges combined
/// with overridden manual edges. It also includes world-coordinate position and size
/// data of each entity in a separate map for visualization purposes.
#[derive(Resource, Default)]
pub struct NavVizMap {
    /// This map contains the navigation edges that would be taken by the
    /// [`AutoDirectionalNavigator`](bevy::ui::auto_directional_navigation::AutoDirectionalNavigator)
    /// with overridden manual edges. This map is scoped to only include entities that are either:
    ///
    /// - under the same camera as the current [`InputFocus`](bevy::input_focus::InputFocus) or
    ///
    /// - manually defined in the existing [`DirectionalNavigationMap`] resource.
    ///
    /// Despite it having the same type, this is not to be confused with the
    /// [`DirectionalNavigationMap`] resource, which is a resource that contains only manually
    /// defined navigation edges.
    pub map: DirectionalNavigationMap,

    /// A cache map that stores an entity's FocusableArea (position and size).
    /// The information is used when drawing navigation edges.
    /// This map only contains entities rendered to the same target as the current [`InputFocus`].
    pub entity_viz_data: EntityHashMap<NavVizData>,
}

/// A System that rebuilds the [`NavVizMap`] resource with the
/// [`AutoDirectionalNavigation`] entities that share the camera with the current [`InputFocus`]
/// and any manual edges defined in the [`DirectionalNavigationMap`] resource.
pub fn rebuild_nav_viz_map(
    mut nav_viz_map: ResMut<NavVizMap>,
    current_focus: Res<InputFocus>,
    manual_edge_map: Res<DirectionalNavigationMap>,
    config: Res<AutoNavigationConfig>,
    navigable_entities_query: Query<
        (
            Entity,
            &'static ComputedUiTargetCamera,
            &'static ComputedNode,
            &'static UiGlobalTransform,
            &'static InheritedVisibility,
        ),
        With<AutoDirectionalNavigation>,
    >,
    camera_and_focusable_area_query: Query<
        (
            Entity,
            &'static ComputedUiTargetCamera,
            &'static ComputedNode,
            &'static UiGlobalTransform,
        ),
        With<AutoDirectionalNavigation>,
    >,
    viewport_logical_size_query: Query<
        &'static ComputedUiRenderTargetInfo,
        With<AutoDirectionalNavigation>,
    >,
) {
    // Get the focusable areas related to the current focus and the entities
    // it shares the camera with. This is what the `AutoDirectionalNavigator`
    // also does under the hood
    let Some(focus) = current_focus.get() else {
        return;
    };
    // The viewport logical size is needed to figure out where to draw the viz.
    let Ok(logical_size) = viz_data::viewport_logical_size(focus, viewport_logical_size_query)
    else {
        return;
    };
    let Some((camera, current_focusable_area)) =
        navigable_nodes::entity_to_camera_and_focusable_area(
            focus,
            camera_and_focusable_area_query,
        )
    else {
        return;
    };
    let mut focusable_areas =
        navigable_nodes::get_navigable_nodes(camera, navigable_entities_query);
    focusable_areas.push(current_focusable_area);

    nav_map::rebuild_nav_map(
        &mut nav_viz_map.map,
        &manual_edge_map,
        &focusable_areas,
        &config,
    );
    viz_data::rebuild_entity_viz_data(
        &mut nav_viz_map.entity_viz_data,
        &focusable_areas,
        logical_size,
    );
}
