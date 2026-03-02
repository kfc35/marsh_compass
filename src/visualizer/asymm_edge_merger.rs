use bevy::ecs::system::SystemParam;
use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::*;

use crate::{
    AutoNavVizDrawMode, AutoNavVizGizmoConfigGroup, DrawLineData, DrawLineType, NavVizDrawMetaData,
    NavVizMap, SymmetricalEdgeSettings,
};

/// A System Param that merges asymmetrical straight edges within the [`draw_nav_viz`](crate::draw_nav_viz) system.
///
/// This system param takes advantage of private [`Local`] variables, so it must be cleared before every use.
///
/// For more information on why this process is necessary, refer to
/// [`AsymmetricalStraightEdgeMerger::do_merge`].
#[derive(SystemParam)]
pub struct AsymmetricalStraightEdgeMerger<'s> {
    processed_asym_straight_edges: Local<'s, HashSet<NavVizDrawMetaData>>,
    /// The map used to detect any "asymmetric but symmetric visually" line data.
    asym_straight_edge_map: Local<'s, HashMap<NavVizDrawMetaData, [DrawLineData; 3]>>,
    /// The output of this system param
    line_data_to_draw: Local<'s, Vec<DrawLineData>>,
}

impl<'s> AsymmetricalStraightEdgeMerger<'s> {
    /// Clears [`Local`] fields so that this param may be re-used.
    ///
    /// Invoke this function before starting to prep visualization data for drawing.
    /// Afterwards, asymmetrical straight edge can be queued for processing via
    /// [`add_straight_edge`](Self::add_straight_edge)
    pub fn clear(&mut self) {
        self.processed_asym_straight_edges.clear();
        self.asym_straight_edge_map.clear();
        self.line_data_to_draw.clear();
    }

    /// Adds an asymmetrical straight edge for processing via [`do_merge`](Self::do_merge)
    pub fn add_straight_edge(
        &mut self,
        meta_data: NavVizDrawMetaData,
        straight_edge_line_data: [DrawLineData; 3],
    ) {
        self.asym_straight_edge_map
            .insert(meta_data, straight_edge_line_data);
    }

    /// Merges any asymmetrical straight edges that would be drawn overlapping and
    /// therefore would appear symmetric to the user, even if it may not be considered
    /// "symmetric" navigation wise.
    ///
    /// This should only be called after first running [clear](Self::clear)'ed and
    /// then adding all the asymmetrical straight edges that need to be processed via
    /// [`add_straight_edge`](Self::add_straight_edge).
    ///
    /// An example: If, between entities A <-> B, there is a NE Nav Edge from A -> B
    /// and a NW Nav Edge from B -> A, then there will be two asymmetrical edges
    /// queued to be drawn between A's NE point and B's NW point.
    /// This is *not* defined as a "symmetric navigation edge", which is a navigation
    /// edge between two opposite directions i.e. NE <-> SW or SE <-> NW.
    /// When the NE and NW nav edges are drawn, however, they would overlap and "appear"
    /// to be a symmetric edge visually. This function merges such edges.
    pub fn do_merge(&mut self, nav_viz_map: &NavVizMap, config: &AutoNavVizGizmoConfigGroup) {
        self.line_data_to_draw
            .shrink_to(self.asym_straight_edge_map.len() * 3);
        // The merging process is only necessary if the symmetric edge setting is not the overlap setting.
        if let AutoNavVizDrawMode::EnabledForAll(symm_edge_settings) = config.draw_mode
            && !symm_edge_settings.is_overlap()
        {
            self.processed_asym_straight_edges.clear();
            self.processed_asym_straight_edges
                .shrink_to(self.asym_straight_edge_map.len());
            for (meta_data, &(mut edge)) in self.asym_straight_edge_map.into_iter() {
                let opposite_meta_data = meta_data.opposite();
                if !self.processed_asym_straight_edges.contains(meta_data)
                    && let Some(other_edge) = self.asym_straight_edge_map.get(&opposite_meta_data)
                {
                    if let SymmetricalEdgeSettings::MergeAndMix(factor) = symm_edge_settings {
                        edge[0].line_type = DrawLineType::Arrow;
                        edge[1].line_type = DrawLineType::Line(None);
                        edge[2].line_type = DrawLineType::Arrow;
                        // override the color of the whole edge with the mixed color.
                        let mixed_color = edge[0].color.mix(&other_edge[0].color, factor);

                        for mut line_data in edge {
                            line_data.color = mixed_color;
                            self.line_data_to_draw.push(line_data);
                        }
                        self.processed_asym_straight_edges
                            .insert(opposite_meta_data);
                    } else if let SymmetricalEdgeSettings::MergeAndGradient = symm_edge_settings {
                        edge[0].line_type = DrawLineType::Arrow;
                        edge[2].line_type = DrawLineType::Arrow;
                        // ensure there is a gradient in the middle line to the destination color.
                        edge[1].color = edge[0].color;
                        edge[1].line_type = DrawLineType::Line(Some(other_edge[0].color));
                        // the arrow to the destination should get its color from the destination entity.
                        edge[2].color = other_edge[0].color;

                        for line_data in edge {
                            self.line_data_to_draw.push(line_data);
                        }
                        self.processed_asym_straight_edges
                            .insert(opposite_meta_data);
                    } else {
                        // symm_edge_settings = SpacingBetweenTwoArrows
                        // Must apply nudging to visibly see two arrows.
                        let from_size = nav_viz_map
                            .entity_viz_pos_data
                            .get(&meta_data.source_entity)
                            .expect("This succeeded when first making these edges")
                            .size;
                        let to_size = nav_viz_map
                            .entity_viz_pos_data
                            .get(&meta_data.destination_entity)
                            .expect("This succeeded when first making these edges")
                            .size;

                        let (start_nudge, end_nudge) = crate::get_nudge(
                            from_size,
                            meta_data.source_direction,
                            to_size,
                            meta_data.destination_direction,
                            symm_edge_settings,
                        );

                        edge[0].start += start_nudge;
                        edge[0].end += start_nudge;
                        edge[1].start += start_nudge;
                        edge[1].end += end_nudge;
                        edge[2].start += end_nudge;
                        edge[2].end += end_nudge;

                        for line_data in edge {
                            self.line_data_to_draw.push(line_data);
                        }
                        // The opposite edge will be processed similarly later
                    }
                } else if !self.processed_asym_straight_edges.contains(meta_data) {
                    // Draw the asymmetrical edge as normal.
                    for line_data in edge {
                        self.line_data_to_draw.push(line_data);
                    }
                }
                self.processed_asym_straight_edges.insert(*meta_data);
            }
        } else {
            // If we don't have to merge, just return all the edge data.
            for line_data in self
                .asym_straight_edge_map
                .into_iter()
                .flat_map(|(_, &edge)| edge)
            {
                self.line_data_to_draw.push(line_data);
            }
        }
    }

    /// Returns drawable line data from merged asymmetrical straight edges
    /// to be drawn by [`Gizmos`].
    ///
    /// This should only be called after [`do_merge`](Self::do_merge) have been called.
    pub fn get_line_data(&self) -> &Vec<DrawLineData> {
        &self.line_data_to_draw
    }
}
