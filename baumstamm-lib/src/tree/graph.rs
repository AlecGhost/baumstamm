use super::{PersonId, Relationship, RelationshipId};
use itertools::Itertools;
use std::{
    borrow::Borrow,
    cell::RefCell,
    rc::{Rc, Weak},
};

pub fn extract_persons(relationships: &[Relationship]) -> Vec<PersonId> {
    let parents = relationships.iter().flat_map(|rel| rel.parents());
    let children = relationships.iter().flat_map(|rel| rel.children.to_vec());
    parents.chain(children).unique().collect()
}

fn rids_of_children(rid: &RelationshipId, relationships: &[Relationship]) -> Vec<RelationshipId> {
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

fn rids_of_parents(
    rid: &RelationshipId,
    relationships: &[Relationship],
) -> [Option<RelationshipId>; 2] {
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

pub struct Node {
    value: RelationshipId,
    parents: RefCell<[Weak<Node>; 2]>,
    children: RefCell<Vec<Rc<Node>>>,
}

impl Node {
    fn new(value: RelationshipId) -> Self {
        Self {
            value,
            parents: RefCell::new([Weak::new(), Weak::new()]),
            children: RefCell::new(Vec::new()),
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
            .field("children", &self.children.borrow())
            .finish()
    }
}

impl Node {
    fn is_ancestor_of(&self, other: &Rc<Self>) -> Option<usize> {
        if self == other.borrow() {
            return Some(0);
        }
        self.children
            .borrow()
            .iter()
            .filter_map(|child| child.is_ancestor_of(other))
            .reduce(|acc, level| acc.max(level) + 1)
    }

    fn is_descendant_of(&self, other: &Rc<Self>) -> Option<usize> {
        if self == other.borrow() {
            return Some(0);
        }
        self.parents
            .borrow()
            .iter()
            .filter_map(|parent| parent.upgrade())
            .filter_map(|parent| parent.is_descendant_of(other))
            .reduce(|acc, level| acc.max(level) + 1)
    }

    fn walk_descendants(self: &Rc<Self>) -> DescendantWalker {
        DescendantWalker::new(Rc::clone(self))
    }
}

#[derive(Debug)]
struct DescendantWalker {
    node: Rc<Node>,
    index: usize,
    child_walker: Option<Box<DescendantWalker>>,
}

impl DescendantWalker {
    fn new(node: Rc<Node>) -> Self {
        Self {
            node,
            index: 0,
            child_walker: None,
        }
    }
}

impl Iterator for DescendantWalker {
    type Item = Rc<Node>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(child_walker) = &mut self.child_walker {
            if let Some(next) = child_walker.next() {
                Some(next)
            } else {
                self.child_walker = None;
                self.index += 1;
                self.next()
            }
        } else if let Some(child) = self.node.children.borrow().get(self.index) {
            self.child_walker = Some(Box::new(DescendantWalker {
                node: Rc::clone(child),
                index: 0,
                child_walker: None,
            }));
            Some(Rc::clone(child))
        } else {
            None
        }
    }
}

#[derive(Debug, Default)]
pub struct Graph {
    sources: Vec<Rc<Node>>,
}

impl Graph {
    fn get_nodes(&self) -> Vec<Rc<Node>> {
        let mut nodes: Vec<Rc<Node>> = self.sources.iter().map(Rc::clone).collect();
        let mut index = 0;
        while index < nodes.len() {
            #[allow(clippy::needless_collect)]
            let children: Vec<Rc<Node>> = nodes[index]
                .children
                .borrow()
                .iter()
                .map(Rc::clone)
                .collect();
            children.into_iter().for_each(|child| {
                if !nodes.contains(&child) {
                    nodes.push(child);
                }
            });
            index += 1;
        }
        nodes
    }

    pub fn generate(relationships: &[Relationship]) -> Self {
        fn add_children(relationships: &[Relationship], nodes: &[Rc<Node>], parent: &mut Rc<Node>) {
            // return if children are already added
            if !parent.children.borrow().is_empty() {
                return;
            }
            // find child relationships and add them to the graph recursively
            rids_of_children(&parent.value, relationships)
                .iter()
                .for_each(|id| {
                    // extract right node from already existing node list
                    let node = nodes
                        .iter()
                        .find(|node| node.value == *id)
                        .expect("Node creation failed");
                    // add pointer to child node to parent
                    parent.children.borrow_mut().push(Rc::clone(node));
                    // add pointer to parent node to child
                    *node.parents.borrow_mut() = rids_of_parents(id, relationships).map(|rid| {
                        if let Some(parent_id) = rid {
                            let parent_node = nodes
                                .iter()
                                .find(|node| node.value == parent_id)
                                .expect("Parent relationship must exist");
                            Rc::downgrade(parent_node)
                        } else {
                            Weak::new()
                        }
                    });
                    // call function recursively for all children
                    parent
                        .children
                        .borrow_mut()
                        .iter_mut()
                        .for_each(|child| add_children(relationships, nodes, child));
                });
        }

        let mut graph = Self::default();
        let nodes: Vec<Rc<Node>> = relationships
            .iter()
            .map(|rel| Rc::new(Node::new(rel.id)))
            .collect();
        // add relationships with no parents as top level nodes
        relationships
            .iter()
            // get relationships with no parents
            .filter(|rel| rel.parents().is_empty())
            .map(|rel| rel.id)
            .for_each(|id| {
                // add it to root node
                let node = nodes
                    .iter()
                    .find(|node| node.value == id)
                    .expect("Node creation failed");
                graph.sources.push(Rc::clone(node));
            });
        // build graph/add rest of the nodes
        graph
            .sources
            .iter_mut()
            .for_each(|child| add_children(relationships, &nodes, child));
        graph
    }

    pub fn cut(&mut self) {
        struct Descendant {
            rid: RelationshipId,
            level: usize,
        }

        fn update_sources(graph: &mut Graph, nodes: &[Rc<Node>]) {
            let mut new_top_level_nodes: Vec<Rc<Node>> = nodes
                .iter()
                .filter(|node| !graph.sources.contains(node))
                .filter(|node| {
                    let parents = node.parents.borrow();
                    parents[0].upgrade().is_none() && parents[1].upgrade().is_none()
                })
                .map(Rc::clone)
                .collect();
            graph.sources.append(&mut new_top_level_nodes);
        }

        fn cut_parent(node: &Rc<Node>, parent: &Rc<Node>) {
            if let Some(cuttable_parent) = node.parents.borrow_mut().iter_mut().find(
                |cuttable_parent| matches!(cuttable_parent.upgrade(), Some(p) if &p == parent),
            ) {
                *cuttable_parent = Weak::new();
            };
        }

        /**
        If there is a cycle, remove all but one (the longest) edges, to cut the cycle.

        The relationship graph is a directed acyclic graph, therefore there are no cycles it its original sense.
        But if one relationship descends from another in more than one ways, I call it a cycle in this context.
        */
        fn cut_cycles(parent: &mut Rc<Node>, nodes: &[Rc<Node>]) {
            let mut children = parent.children.borrow_mut();
            // no children, no cycle
            if children.is_empty() {
                return;
            }
            // go through each node and cut the cycles
            nodes.iter().for_each(|node| {
                // collect child nodes, who are ancestors of the current node
                let cycle_children: Vec<Descendant> = children
                    .iter()
                    .filter_map(|child| {
                        child.is_ancestor_of(node).map(|level| Descendant {
                            rid: child.value,
                            level,
                        })
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
                    let is_cycle_child = |child: RelationshipId| {
                        cycle_children
                            .iter()
                            .any(|descendant| child == descendant.rid)
                    };
                    children.retain(|child| {
                        if child.value == has_longest_edge || !is_cycle_child(child.value) {
                            true
                        } else {
                            cut_parent(child, parent);
                            false
                        }
                    });
                }
            });
            // continue recursively through the graph
            children
                .iter_mut()
                .for_each(|child| cut_cycles(child, nodes));
        }

        fn cut_double_inheritance(graph: &mut Graph) {
            for (this, next) in graph.sources.iter().tuple_combinations() {
                let mut children = this.children.borrow_mut();
                let cuttable_children = children
                    .borrow()
                    .iter()
                    .filter_map(|child| {
                        child.walk_descendants().find_map(|descendant| {
                            descendant.is_descendant_of(next).map(|level| Descendant {
                                rid: child.value,
                                level,
                            })
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
                    let should_cut = |child: RelationshipId| {
                        cuttable_children
                            .iter()
                            .any(|descendant| child == descendant.rid)
                    };
                    children.retain(|child| {
                        if child.value == has_longest_edge || !should_cut(child.value) {
                            true
                        } else {
                            cut_parent(child, this);
                            false
                        }
                    });
                }
            }
        }

        fn cut_xs(nodes: &[Rc<Node>]) {
            fn get_parents(node: &Rc<Node>) -> Vec<Rc<Node>> {
                node.parents
                    .borrow()
                    .iter()
                    .filter_map(|parent| parent.upgrade())
                    .collect()
            }

            fn is_descendant_of_both(node: &Rc<Node>, anc_a: &Rc<Node>, anc_b: &Rc<Node>) -> bool {
                node.is_descendant_of(anc_a).is_some() && node.is_descendant_of(anc_b).is_some()
            }

            fn is_x_child(
                node_a: &Rc<Node>,
                node_b: &Rc<Node>,
                anc_a: &Rc<Node>,
                anc_b: &Rc<Node>,
            ) -> bool {
                is_descendant_of_both(node_a, anc_a, anc_b)
                    && is_descendant_of_both(node_b, anc_a, anc_b)
                    && !get_parents(node_a)
                        .iter()
                        .any(|parent| is_descendant_of_both(parent, anc_a, anc_b))
                    && !get_parents(node_b)
                        .iter()
                        .any(|parent| is_descendant_of_both(parent, anc_a, anc_b))
            }

            let x_children: Vec<Rc<Node>> = nodes
                .iter()
                .tuple_combinations()
                .filter(|(node_a, node_b)| {
                    // x is only possible, when there are two parents
                    get_parents(node_a).len() == 2 && get_parents(node_b).len() == 2
                })
                .filter(|(child_a, child_b)| {
                    nodes
                        .iter()
                        .tuple_combinations()
                        .any(|(anc_a, anc_b)| is_x_child(child_a, child_b, anc_a, anc_b))
                })
                .map(|(node_a, _)| Rc::clone(node_a))
                .collect();

            for x_child in x_children {
                let first_parent = x_child.parents.borrow()[0]
                    .upgrade()
                    .expect("X child's parent must exist");
                first_parent
                    .children
                    .borrow_mut()
                    .retain(|child| child != &x_child);
                x_child.parents.borrow_mut()[0] = Weak::new();
            }
        }

        let nodes = self.get_nodes();

        // cut cycles
        self
            .sources
            .iter_mut()
            .for_each(|child| cut_cycles(child, &nodes));
        update_sources(self, &nodes);
        // cut double inheritance
        cut_double_inheritance(self);
        update_sources(self, &nodes);
        // cut xs
        cut_xs(&nodes);
    }

    pub fn layers(&self) -> Vec<Vec<RelationshipId>> {
        fn add_layer_rec(
            node: &Node,
            layers: &mut Vec<Vec<RelationshipId>>,
            origin: Option<RelationshipId>,
            level: usize,
        ) {
            if level == layers.len() {
                layers.push(Vec::new());
            }
            layers[level].push(node.value);
            node.parents.borrow().iter().for_each(|parent| {
                if let Some(parent_node) = parent.upgrade() {
                    if Some(parent_node.value) != origin && parent_node.value != 0 {
                        let next_level = if level == 0 {
                            layers.insert(0, Vec::new());
                            0
                        } else {
                            level - 1
                        };
                        add_layer_rec(&parent_node, layers, Some(node.value), next_level);
                    }
                }
            });
            node.children.borrow().iter().for_each(|child| {
                if Some(child.value) != origin {
                    add_layer_rec(child, layers, Some(node.value), level + 1)
                }
            });
        }
        let mut layers = vec![Vec::new()];
        let start = self
            .sources
            .first()
            .expect("Root node must have at least one child");
        add_layer_rec(start, &mut layers, None, 0);
        layers
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::tree::{consistency, io};
    use insta::assert_debug_snapshot;

    #[test]
    fn cycles() {
        let graph_data = io::read("test/graph/cycles.json").expect("Cannot read test file");
        consistency::check(&graph_data).expect("Test data inconsistent");
        let mut graph = Graph::generate(&graph_data.relationships);
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
        let mut graph = Graph::generate(&graph_data.relationships);
        graph.cut();
        assert_debug_snapshot!(graph);
        let layers = graph.layers();
        assert_debug_snapshot!(layers);
    }

    #[test]
    fn xs() {
        let graph_data = io::read("test/graph/xs.json").expect("Cannot read test file");
        consistency::check(&graph_data).expect("Test data inconsistent");
        let mut graph = Graph::generate(&graph_data.relationships);
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
        let mut graph = Graph::generate(&graph_data.relationships);
        graph.cut();
        assert_debug_snapshot!(graph);
        let layers = graph.layers();
        assert_debug_snapshot!(layers);
    }
}
