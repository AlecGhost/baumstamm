use crate::{Relationship, RelationshipId};
use itertools::Itertools;

type Rid = RelationshipId;

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

#[derive(Debug)]
pub struct Graph {
    sources: Vec<Rid>,
    nodes: Vec<Node>,
}

impl Graph {
    pub fn new(relationships: &[Relationship]) -> Self {
        fn rids_of_children(rid: &Rid, relationships: &[Relationship]) -> Vec<Rid> {
            let current = relationships
                .iter()
                .find(|rel| rel.id == *rid)
                .expect("Inconsistent data");
            relationships
                .iter()
                .filter(|rel| {
                    current
                        .children
                        .iter()
                        .any(|child| rel.parents().contains(child))
                })
                .map(|rel| rel.id)
                .collect()
        }

        fn rids_of_parents(rid: &Rid, relationships: &[Relationship]) -> [Option<Rid>; 2] {
            let current = relationships
                .iter()
                .find(|rel| rel.id == *rid)
                .expect("Inconsistent data");
            current.parents.map(|opt| {
                opt.as_ref()
                    .and_then(|parent_id| {
                        relationships
                            .iter()
                            .find(|rel| rel.children.contains(parent_id))
                    })
                    .map(|rel| rel.id)
            })
        }

        let mut nodes: Vec<Node> = relationships.iter().map(|rel| Node::new(rel.id)).collect();
        // add relationships with no parents as top level nodes
        let sources = relationships
            .iter()
            // get relationships with no parents
            .filter(|rel| rel.parents().is_empty())
            .map(|rel| rel.id)
            .collect();

        nodes.iter_mut().for_each(|node| {
            let rid = node.value;
            node.parents = rids_of_parents(&rid, relationships);
            node.children = rids_of_children(&rid, relationships);
        });
        Self { sources, nodes }
    }

    fn parents_of(&self, rid: &Rid) -> Vec<Rid> {
        let node = self
            .nodes
            .iter()
            .find(|node| node.value == *rid)
            .expect("Invalid RID");
        node.parents.iter().filter_map(|parent| *parent).collect()
    }

    fn children_of(&self, rid: &Rid) -> &[Rid] {
        let node = self
            .nodes
            .iter()
            .find(|node| node.value == *rid)
            .expect("Invalid RID");
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

    pub fn cut(&mut self) {
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
            let node = graph
                .nodes
                .iter_mut()
                .find(|node| node.value == *child)
                .expect("Invalid RID");
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
            let parent_node = graph
                .nodes
                .iter_mut()
                .find(|node| node.value == *parent)
                .expect("Invalid RID");
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
                let this_node = graph
                    .nodes
                    .iter_mut()
                    .find(|node| node.value == *this)
                    .expect("Invalid RID");
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
        sources.iter().for_each(|child| cut_cycles(self, child));
        update_sources(self);
        // cut double inheritance
        cut_double_inheritance(self);
        update_sources(self);
        // cut xs
        cut_xs(self);
    }

    pub fn get_sources(&self) -> &[Rid] {
        self.sources.as_slice()
    }

    pub fn layers(&self) -> Vec<Vec<Rid>> {
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
        let start = self
            .sources
            .first()
            .expect("Root node must have at least one child");
        add_layer_rec(self, start, &mut layers, None, 0);
        layers
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{consistency, io};
    use insta::assert_debug_snapshot;

    #[test]
    fn cycles() {
        let graph_data = io::read("test/graph/cycles.json").expect("Cannot read test file");
        consistency::check(&graph_data).expect("Test data inconsistent");
        let mut graph = Graph::new(&graph_data.relationships);
        graph.cut();
        assert_debug_snapshot!(graph);
        let layers = graph.layers();
        assert_debug_snapshot!(layers);
    }

    #[test]
    fn double_inheritance() {
        let graph_data =
            io::read("test/graph/double_inheritance.json").expect("Cannot read test file");
        consistency::check(&graph_data).expect("Test data inconsistent");
        let mut graph = Graph::new(&graph_data.relationships);
        graph.cut();
        assert_debug_snapshot!(graph);
        let layers = graph.layers();
        assert_debug_snapshot!(layers);
    }

    #[test]
    fn xs() {
        let graph_data = io::read("test/graph/xs.json").expect("Cannot read test file");
        consistency::check(&graph_data).expect("Test data inconsistent");
        let mut graph = Graph::new(&graph_data.relationships);
        graph.cut();
        assert_debug_snapshot!(graph);
        let layers = graph.layers();
        assert_debug_snapshot!(layers);
    }

    #[test]
    fn double_inheritance_and_xs() {
        let graph_data =
            io::read("test/graph/double_inheritance_and_xs.json").expect("Cannot read test file");
        consistency::check(&graph_data).expect("Test data inconsistent");
        let mut graph = Graph::new(&graph_data.relationships);
        graph.cut();
        assert_debug_snapshot!(graph);
        let layers = graph.layers();
        assert_debug_snapshot!(layers);
    }
}
