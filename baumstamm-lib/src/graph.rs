use crate::{error::DisplayError, Relationship};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashMap;

type Rid = u128;

#[derive(Serialize, Deserialize, Type)]
struct Node {
    value: Rid,
    parents: [Option<Rid>; 2],
    children: Vec<Rid>,
}

impl Node {
    const fn new(value: Rid) -> Self {
        Self {
            value,
            parents: [None, None],
            children: Vec::new(),
        }
    }
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl std::fmt::Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node")
            .field("value", &self.value)
            .field("children", &self.children)
            .finish()
    }
}

#[derive(Debug)]
struct DescendantWalker<'a> {
    graph: &'a Graph,
    rid: Rid,
    index: usize,
    child_walker: Option<Box<Self>>,
}

impl<'a> DescendantWalker<'a> {
    const fn new(graph: &'a Graph, rid: Rid) -> Self {
        Self {
            graph,
            rid,
            index: 0,
            child_walker: None,
        }
    }
}

impl<'a> Iterator for DescendantWalker<'a> {
    type Item = Rid;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(child_walker) = &mut self.child_walker {
            if let Some(next) = child_walker.next() {
                Some(next)
            } else {
                self.child_walker = None;
                self.index += 1;
                self.next()
            }
        } else if let Some(child) = self.graph.children_of(&self.rid).get(self.index) {
            self.child_walker = Some(Box::new(Self {
                graph: self.graph,
                rid: *child,
                index: 0,
                child_walker: None,
            }));
            Some(*child)
        } else {
            None
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct Graph {
    sources: Vec<Rid>,
    nodes: Vec<Node>,
}

impl Graph {
    pub fn new(relationships: &[Relationship]) -> Self {
        fn rids_of_children(rid: &Rid, relationships: &[Relationship]) -> Vec<Rid> {
            let current = relationships
                .iter()
                .find(|rel| rel.id.0 == *rid)
                .expect("Inconsistent data");
            relationships
                .iter()
                .filter(|rel| {
                    current
                        .children
                        .iter()
                        .any(|child| rel.parents().contains(child))
                })
                .map(|rel| rel.id.0)
                .collect()
        }

        fn rids_of_parents(rid: &Rid, relationships: &[Relationship]) -> [Option<Rid>; 2] {
            let current = relationships
                .iter()
                .find(|rel| rel.id.0 == *rid)
                .expect("Inconsistent data");
            current.parents.map(|opt| {
                opt.as_ref()
                    .and_then(|parent_id| {
                        relationships
                            .iter()
                            .find(|rel| rel.children.contains(parent_id))
                    })
                    .map(|rel| rel.id.0)
            })
        }

        let mut nodes: Vec<Node> = relationships
            .iter()
            .map(|rel| Node::new(rel.id.0))
            .collect();
        // add relationships with no parents as top level nodes
        let sources = relationships
            .iter()
            // get relationships with no parents
            .filter(|rel| rel.parents().is_empty())
            .map(|rel| rel.id.0)
            .collect();

        nodes.iter_mut().for_each(|node| {
            let rid = node.value;
            node.parents = rids_of_parents(&rid, relationships);
            node.children = rids_of_children(&rid, relationships);
        });
        Self { sources, nodes }
    }

    fn get_node(&self, rid: &Rid) -> &Node {
        self.nodes
            .iter()
            .find(|node| node.value == *rid)
            .expect("Invalid RID")
    }

    fn get_node_mut(&mut self, rid: &Rid) -> &mut Node {
        self.nodes
            .iter_mut()
            .find(|node| node.value == *rid)
            .expect("Invalid RID")
    }

    fn parents_of(&self, rid: &Rid) -> Vec<Rid> {
        let node = self.get_node(rid);
        node.parents.iter().filter_map(|parent| *parent).collect()
    }

    fn children_of(&self, rid: &Rid) -> &[Rid] {
        let node = self.get_node(rid);
        node.children.as_slice()
    }

    fn is_ancestor_of(&self, rid: &Rid, other: &Rid) -> Option<usize> {
        if rid == other {
            return Some(0);
        }
        self.children_of(rid)
            .iter()
            .filter_map(|child| self.is_ancestor_of(child, other))
            .reduce(|acc, level| acc.max(level) + 1)
    }

    fn is_descendant_of(&self, rid: &Rid, other: &Rid) -> Option<usize> {
        if rid == other {
            return Some(0);
        }
        self.parents_of(rid)
            .iter()
            .filter_map(|parent| self.is_descendant_of(parent, other))
            .reduce(|acc, level| acc.max(level) + 1)
    }

    const fn walk_descendants(&self, rid: &Rid) -> DescendantWalker {
        DescendantWalker::new(self, *rid)
    }

    pub fn cut(mut self) -> CutGraph {
        struct Descendant {
            rid: Rid,
            level: usize,
        }

        fn update_sources(graph: &mut Graph) {
            let new_top_level_rids: Vec<Rid> = graph
                .nodes
                .iter()
                .map(|node| node.value)
                .filter(|rid| !graph.sources.contains(rid))
                .filter(|rid| graph.parents_of(rid).is_empty())
                .collect();
            graph.sources.extend(new_top_level_rids);
        }

        fn cut_parent(graph: &mut Graph, child: &Rid, parent: &Rid) {
            let node = graph.get_node_mut(child);
            if let Some(cuttable_parent) = node
                .parents
                .iter_mut()
                .find(|cuttable_parent| matches!(cuttable_parent, Some(p) if p == parent))
            {
                *cuttable_parent = None;
            };
        }

        /**
        If there is a cycle, remove all but one (the longest) edges, to cut the cycle.

        The relationship graph is a directed acyclic graph, therefore there are no cycles it its original sense.
        But if one relationship descends from another in more than one ways, I call it a cycle in this context.
        */
        fn cut_cycles(graph: &mut Graph, parent: &Rid) {
            let mut children = graph.children_of(parent).to_vec();
            // no children, no cycle
            if children.is_empty() {
                return;
            }
            // go through each node and cut the cycles
            let rids: Vec<Rid> = graph.nodes.iter().map(|node| node.value).collect();
            rids.iter().for_each(|rid| {
                // collect child nodes, who are ancestors of the current node
                let cycle_children: Vec<Descendant> = children
                    .iter()
                    .filter_map(|child| {
                        graph
                            .is_ancestor_of(child, rid)
                            .map(|level| Descendant { rid: *child, level })
                    })
                    .collect();
                // if the node descends from more than one child, there is a cycle
                if cycle_children.len() > 1 {
                    // find out, which of the child relationships has the longer edge to the node, and should therefore remain in the graph
                    let has_longest_edge = cycle_children
                        .iter()
                        .reduce(|acc, child| if acc.level > child.level { acc } else { child })
                        .expect("There must be at least two cycle children")
                        .rid;
                    // remove the pointers to the child relationships, where there is a cycle, except for the longest edge to the node
                    let is_cycle_child = |child: &Rid| {
                        cycle_children
                            .iter()
                            .any(|descendant| *child == descendant.rid)
                    };
                    children.retain(|child| {
                        if *child == has_longest_edge || !is_cycle_child(child) {
                            true
                        } else {
                            cut_parent(graph, child, parent);
                            false
                        }
                    });
                }
            });
            // continue recursively through the graph
            children
                .iter_mut()
                .for_each(|child| cut_cycles(graph, child));
            // replace children
            let parent_node = graph.get_node_mut(parent);
            parent_node.children = children;
        }

        fn cut_double_inheritance(graph: &mut Graph) {
            let sources = graph.sources.clone();
            for (this, next) in sources.iter().tuple_combinations() {
                let mut children = graph.children_of(this).to_vec();
                let cuttable_children = children
                    .iter()
                    .filter_map(|child| {
                        graph.walk_descendants(child).find_map(|descendant| {
                            graph
                                .is_descendant_of(&descendant, next)
                                .map(|level| Descendant { rid: *child, level })
                        })
                    })
                    .collect_vec();
                if cuttable_children.len() > 1 {
                    // find out, which of the child relationships has the longer edge to the node, and should therefore remain in the graph
                    let has_longest_edge = cuttable_children
                        .iter()
                        .reduce(|acc, child| if acc.level > child.level { acc } else { child })
                        .expect("There must be at least two cuttable children")
                        .rid;
                    // remove the pointers to the child relationships, where there is a cycle, except for the longest edge to the node
                    let should_cut = |child: &Rid| {
                        cuttable_children
                            .iter()
                            .any(|descendant| child == &descendant.rid)
                    };
                    children.retain(|child| {
                        if *child == has_longest_edge || !should_cut(child) {
                            true
                        } else {
                            cut_parent(graph, child, this);
                            false
                        }
                    });
                }
                // replace children
                let this_node = graph.get_node_mut(this);
                this_node.children = children;
            }
        }

        fn cut_xs(graph: &mut Graph) {
            fn is_descendant_of_both(graph: &Graph, child: &Rid, anc_a: &Rid, anc_b: &Rid) -> bool {
                graph.is_descendant_of(child, anc_a).is_some()
                    && graph.is_descendant_of(child, anc_b).is_some()
            }

            fn is_x_child(
                graph: &Graph,
                child_a: &Rid,
                child_b: &Rid,
                anc_a: &Rid,
                anc_b: &Rid,
            ) -> bool {
                is_descendant_of_both(graph, child_a, anc_a, anc_b)
                    && is_descendant_of_both(graph, child_b, anc_a, anc_b)
                    && !graph
                        .parents_of(child_a)
                        .iter()
                        .any(|parent| is_descendant_of_both(graph, parent, anc_a, anc_b))
                    && !graph
                        .parents_of(child_b)
                        .iter()
                        .any(|parent| is_descendant_of_both(graph, parent, anc_a, anc_b))
            }

            let x_children: Vec<Rid> = graph
                .nodes
                .iter()
                .map(|node| node.value)
                .tuple_combinations()
                .filter(|(child_a, child_b)| {
                    // x is only possible, when there are two parents
                    graph.parents_of(child_a).len() == 2 && graph.parents_of(child_b).len() == 2
                })
                .filter(|(child_a, child_b)| {
                    graph
                        .nodes
                        .iter()
                        .map(|node| &node.value)
                        .tuple_combinations()
                        .any(|(anc_a, anc_b)| is_x_child(graph, child_a, child_b, anc_a, anc_b))
                })
                .map(|(child_a, _)| child_a)
                .collect();
            let x_parents: Vec<Rid> = x_children
                .iter()
                .map(|x_child| {
                    graph
                        .parents_of(x_child)
                        .first()
                        .cloned()
                        .expect("X child's parent must exist")
                })
                .collect();
            graph
                .nodes
                .iter_mut()
                .filter(|node| x_children.contains(&node.value))
                .for_each(|x_child_node| x_child_node.parents[0] = None);
            graph
                .nodes
                .iter_mut()
                .filter(|node| x_parents.contains(&node.value))
                .enumerate()
                .for_each(|(i, x_parent_node)| {
                    x_parent_node
                        .children
                        .retain(|child| child != &x_children[i])
                });
        }

        // cut cycles
        let sources = self.sources.clone();
        sources
            .iter()
            .for_each(|child| cut_cycles(&mut self, child));
        update_sources(&mut self);
        // cut double inheritance
        cut_double_inheritance(&mut self);
        update_sources(&mut self);
        // cut xs
        cut_xs(&mut self);
        CutGraph(self)
    }
}

fn layers(graph: &Graph) -> Vec<Vec<Rid>> {
    fn add_layer_rec(
        graph: &Graph,
        rid: &Rid,
        layers: &mut Vec<Vec<Rid>>,
        origin: Option<Rid>,
        level: usize,
    ) {
        if level == layers.len() {
            layers.push(Vec::new());
        }
        layers[level].push(*rid);
        graph.parents_of(rid).into_iter().for_each(|parent| {
            if Some(parent) != origin {
                let next_level = if level == 0 {
                    layers.insert(0, Vec::new());
                    0
                } else {
                    level - 1
                };
                add_layer_rec(graph, &parent, layers, Some(*rid), next_level);
            }
        });
        graph.children_of(rid).iter().for_each(|child| {
            if Some(*child) != origin {
                add_layer_rec(graph, child, layers, Some(*rid), level + 1)
            }
        });
    }
    let mut layers = vec![Vec::new()];
    let start = graph
        .sources
        .first()
        .expect("Root node must have at least one child");
    add_layer_rec(graph, start, &mut layers, None, 0);
    layers
}

#[derive(Debug)]
pub struct CutGraph(Graph);

impl CutGraph {
    pub fn layers(&self) -> Vec<Vec<Rid>> {
        layers(&self.0)
    }

    /// Modify graph, so that it can be displayed in 2D without overlapping edges.
    /// For this, no more than two child relationships
    /// can have their other-parent relationship displayed.
    ///
    /// The result is dependant on the starting relationship,
    /// so it must be specified in the options.
    /// Furthermore, edges that should be retained can be specified.
    /// However, there must not be more than two other-parent relationships
    /// per parent relationship.
    ///
    /// other-parent parent other-parent
    ///      |  +------|------+  |
    ///      \  |      |      |  /
    ///      child   child   child
    pub fn display(self, options: DisplayOptions) -> Result<DisplayGraph, DisplayError> {
        fn cut_children(
            rid: &Rid,
            graph: &mut Graph,
            options: &DisplayOptions,
            previous: Option<&Rid>,
        ) -> Result<(), DisplayError> {
            let children = graph.children_of(rid).to_vec();

            if children.len() > 2 {
                let other_edges = children
                    .iter()
                    .flat_map(|child| {
                        graph
                            .parents_of(child)
                            .into_iter()
                            .filter(|parent| parent != rid)
                            .map(|parent| (parent, *child))
                            .collect_vec()
                    })
                    .collect_vec();
                if other_edges.len() > 2 {
                    type Edge = (Rid, Rid);
                    let (mut retain, mut dismiss): (Vec<Edge>, Vec<Edge>) =
                        other_edges.into_iter().partition(|(parent, child)| {
                            if let Some(options) = options.retain_edges.get(parent) {
                                options.iter().flatten().contains(child)
                            } else {
                                false
                            }
                        });
                    if retain.len() > 2 {
                        return Err(DisplayError::ConflictingRetain);
                    }
                    while retain.len() < 2 {
                        retain.push(dismiss.pop().expect("There must be enough parents"))
                    }
                    dismiss.iter().for_each(|(parent, child)| {
                        graph
                            .get_node_mut(child)
                            .parents
                            .iter_mut()
                            .for_each(|opt| {
                                if matches!(opt, Some(p) if p == parent) {
                                    *opt = None;
                                }
                            });
                        graph.get_node_mut(parent).children.retain(|c| c != child);

                        // cleanup
                        let relatives = graph
                            .nodes
                            .iter()
                            .map(|node| node.value)
                            .filter(|rid| {
                                graph.is_ancestor_of(parent, rid).is_some()
                                    || graph.is_descendant_of(parent, rid).is_some()
                            })
                            .collect_vec();
                        graph.nodes.retain(|node| !relatives.contains(&node.value));
                        graph.sources.retain(|source| !relatives.contains(source));
                    });
                }
            }

            for child in children
                .iter()
                .filter(|child| !matches!(previous, Some(rid) if rid == *child))
            {
                cut_children(child, graph, options, Some(rid))?;
                cut_parents(child, graph, options, Some(rid))?;
            }
            Ok(())
        }

        fn cut_parents(
            rid: &Rid,
            graph: &mut Graph,
            options: &DisplayOptions,
            previous: Option<&Rid>,
        ) -> Result<(), DisplayError> {
            for parent in graph
                .parents_of(rid)
                .into_iter()
                .filter(|parent| !matches!(previous, Some(rid) if rid == parent))
            {
                cut_children(&parent, graph, options, Some(rid))?;
            }
            Ok(())
        }

        fn is_valid(graph: &Graph, rid: &Rid) -> bool {
            graph.nodes.iter().map(|node| node.value).contains(rid)
        }

        let mut graph = self.0;
        let start = graph
            .nodes
            .iter()
            .map(|node| node.value)
            .find(|rid| *rid == options.start)
            .ok_or(DisplayError::InvalidStartId)?;

        // check retain edges
        for (key, values) in options.retain_edges.iter() {
            if !is_valid(&graph, key) {
                return Err(DisplayError::InvalidRetainId);
            }
            for value in values.iter().flatten() {
                if !is_valid(&graph, value) {
                    return Err(DisplayError::InvalidRetainId);
                }
                let node = graph.get_node(key);
                if !node.children.contains(value) {
                    return Err(DisplayError::InvalidRetainEdge);
                }
            }
        }

        // start cutting
        cut_children(&start, &mut graph, &options, None)?;
        cut_parents(&start, &mut graph, &options, None)?;

        Ok(DisplayGraph(graph))
    }
}

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct DisplayGraph(Graph);

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct DisplayOptions {
    #[serde(with = "crate::id")]
    start: Rid,
    retain_edges: HashMap<Rid, [Option<Rid>; 2]>,
}

impl DisplayGraph {
    pub fn layers(&self) -> Vec<Vec<Rid>> {
        layers(&self.0)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{consistency, TreeData};
    use insta::assert_debug_snapshot;

    fn read(file_name: &str) -> TreeData {
        let json_data = std::fs::read_to_string(file_name).expect("Cannot read test file");
        crate::io::read(&json_data).expect("Cannot convert test file")
    }

    #[test]
    fn cycles() {
        let tree_date = read("test/graph/cycles.json");
        consistency::check(&tree_date).expect("Test data inconsistent");
        let graph = Graph::new(&tree_date.relationships);
        let cut_graph = graph.cut();
        assert_debug_snapshot!(cut_graph);
        let layers = cut_graph.layers();
        assert_debug_snapshot!(layers);
    }

    #[test]
    fn double_inheritance() {
        let tree_date = read("test/graph/double_inheritance.json");
        consistency::check(&tree_date).expect("Test data inconsistent");
        let graph = Graph::new(&tree_date.relationships);
        let cut_graph = graph.cut();
        assert_debug_snapshot!(cut_graph);
        let layers = cut_graph.layers();
        assert_debug_snapshot!(layers);
    }

    #[test]
    fn xs() {
        let tree_date = read("test/graph/xs.json");
        consistency::check(&tree_date).expect("Test data inconsistent");
        let graph = Graph::new(&tree_date.relationships);
        let cut_graph = graph.cut();
        assert_debug_snapshot!(cut_graph);
        let layers = cut_graph.layers();
        assert_debug_snapshot!(layers);
    }

    #[test]
    fn double_inheritance_and_xs() {
        let tree_date = read("test/graph/double_inheritance_and_xs.json");
        consistency::check(&tree_date).expect("Test data inconsistent");
        let graph = Graph::new(&tree_date.relationships);
        let cut_graph = graph.cut();
        assert_debug_snapshot!(cut_graph);
        let layers = cut_graph.layers();
        assert_debug_snapshot!(layers);
    }

    #[test]
    fn display_no_retain() {
        let tree_date = read("test/graph/display.json");
        consistency::check(&tree_date).expect("Test data inconsistent");
        let graph = Graph::new(&tree_date.relationships);
        let cut_graph = graph.cut();
        assert_debug_snapshot!(cut_graph);
        let display = cut_graph.display(DisplayOptions {
            start: 0,
            retain_edges: HashMap::new(),
        });
        assert_debug_snapshot!(display);
    }

    #[test]
    fn display_retain_one() {
        let tree_date = read("test/graph/display.json");
        consistency::check(&tree_date).expect("Test data inconsistent");
        let graph = Graph::new(&tree_date.relationships);
        let cut_graph = graph.cut();
        let display = cut_graph.display(DisplayOptions {
            start: 0,
            retain_edges: HashMap::from([(6, [Some(3), None])]),
        });
        assert_debug_snapshot!(display);
    }

    #[test]
    fn display_retain_two() {
        let tree_date = read("test/graph/display.json");
        consistency::check(&tree_date).expect("Test data inconsistent");
        let graph = Graph::new(&tree_date.relationships);
        let cut_graph = graph.cut();
        let display = cut_graph.display(DisplayOptions {
            start: 0,
            retain_edges: HashMap::from([(6, [Some(3), None]), (7, [Some(4), None])]),
        });
        assert_debug_snapshot!(display);
    }
}
