use std::collections::{BTreeMap, HashMap, HashSet};

use cellar_core::er::{ErGraph, ErNode};

#[derive(Clone)]
pub(super) struct ErViewState {
    pub(super) zoom: f32,
    pub(super) tx: f32,
    pub(super) ty: f32,
    pub(super) compact: bool,
    pub(super) expanded: HashSet<String>,
    pub(super) hidden_schemas: HashSet<String>,
    pub(super) schema_menu: bool,
    pub(super) drag: Option<(f32, f32, f32, f32)>,
    pub(super) node_drag: Option<(String, f32, f32, f32, f32)>,
    pub(super) suppress_open: Option<String>,
    pub(super) overrides: HashMap<String, (f32, f32)>,
}

impl Default for ErViewState {
    fn default() -> Self {
        Self {
            zoom: 1.,
            tx: 48.,
            ty: 48.,
            compact: false,
            expanded: HashSet::new(),
            hidden_schemas: HashSet::new(),
            schema_menu: false,
            drag: None,
            node_drag: None,
            suppress_open: None,
            overrides: HashMap::new(),
        }
    }
}

#[derive(Clone)]
pub(super) struct NodeLayout {
    pub(super) node: ErNode,
    pub(super) x: f32,
    pub(super) y: f32,
    pub(super) width: f32,
    pub(super) height: f32,
    pub(super) visible_columns: usize,
}

pub(super) fn layout_nodes(
    nodes: &[ErNode],
    outgoing: &BTreeMap<String, Vec<String>>,
    view: &ErViewState,
) -> Vec<NodeLayout> {
    let columns = (nodes.len() as f32).sqrt().ceil().max(1.) as usize;
    let mut layouts = Vec::with_capacity(nodes.len());
    let mut y = 0.;
    let mut row_height: f32 = 0.;
    for (index, node) in nodes.iter().enumerate() {
        if index > 0 && index % columns == 0 {
            y += row_height + 36.;
            row_height = 0.;
        }
        let candidates = node
            .columns
            .iter()
            .filter(|column| !view.compact || column.is_primary_key || column.is_foreign_key)
            .count();
        let expanded = view.expanded.contains(&node.id);
        let visible_columns = if expanded {
            candidates
        } else {
            candidates.min(14)
        };
        let footer = usize::from(candidates > visible_columns) * 22;
        let relationships = outgoing.get(&node.id).map_or(0, Vec::len) * 18;
        let height = 28. + visible_columns as f32 * 20. + footer as f32 + relationships as f32;
        row_height = row_height.max(height);
        let default_x = (index % columns) as f32 * 298.;
        let (x, node_y) = view
            .overrides
            .get(&node.id)
            .copied()
            .unwrap_or((default_x, y));
        layouts.push(NodeLayout {
            node: node.clone(),
            x,
            y: node_y,
            width: 250.,
            height,
            visible_columns,
        });
    }
    layouts
}

pub(super) fn fit_parameters(graph: &ErGraph, view: &ErViewState) -> (f32, f32, f32) {
    let nodes = graph
        .nodes
        .iter()
        .filter(|node| !view.hidden_schemas.contains(&node.schema))
        .cloned()
        .collect::<Vec<_>>();
    let mut outgoing = BTreeMap::<String, Vec<String>>::new();
    for edge in &graph.edges {
        outgoing
            .entry(edge.source.clone())
            .or_default()
            .push(String::new());
    }
    let layouts = layout_nodes(&nodes, &outgoing, view);
    let width = layouts
        .iter()
        .map(|layout| layout.x + layout.width)
        .fold(0., f32::max);
    let height = layouts
        .iter()
        .map(|layout| layout.y + layout.height)
        .fold(0., f32::max);
    if width == 0. || height == 0. {
        return (1., 48., 48.);
    }
    let zoom = ((1200. - 96.) / width)
        .min((700. - 96.) / height)
        .clamp(0.1, 1.2);
    (
        zoom,
        (1200. - width * zoom) / 2.,
        (700. - height * zoom) / 2.,
    )
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use cellar_core::er::ErNode;

    use super::{layout_nodes, ErViewState};

    #[test]
    fn diagram_layout_is_deterministic_and_non_overlapping() {
        let nodes = (0..4)
            .map(|index| ErNode {
                id: format!("public.t{index}"),
                schema: "public".into(),
                name: format!("t{index}"),
                columns: Vec::new(),
                primary_key: Vec::new(),
                row_count: None,
            })
            .collect::<Vec<_>>();
        let layout = layout_nodes(&nodes, &BTreeMap::new(), &ErViewState::default());
        assert_eq!(layout.len(), 4);
        assert!(layout[1].x >= layout[0].x + layout[0].width);
        assert!(layout[2].y >= layout[0].y + layout[0].height);
    }
}
