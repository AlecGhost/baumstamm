use crate::{extract_persons, Relationship};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use specta::Type;

type Rid = crate::RelationshipId;
type Pid = crate::PersonId;

#[derive(Clone, Serialize, Deserialize, Type)]
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

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
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

    fn is_descendant_of(&self, rid: &Rid, other: &Rid) -> Option<usize> {
        if rid == other {
            return Some(0);
        }
        self.parents_of(rid)
            .iter()
            .filter_map(|parent| self.is_descendant_of(parent, other))
            .reduce(|acc, level| acc.max(level))
            .map(|level| level + 1)
    }

    const fn walk_descendants(&self, rid: &Rid) -> DescendantWalker {
        DescendantWalker::new(self, *rid)
    }

    pub fn cut(mut self) -> CutGraph {
        #[derive(Debug)]
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
                            .is_descendant_of(rid, child)
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
                    // remove the pointers to the child relationships,
                    // where there is a cycle, except for the longest edge to the node
                    let is_cycle_child =
                        |child: &Rid| cycle_children.iter().any(|desc| *child == desc.rid);
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

        /// Double inheritance is when two ancestor nodes
        /// have more than one common descendant node.
        /// Those occurrences are cut by removing the connection
        /// between the common descendant node and one of its parents
        fn cut_double_inheritance(graph: &mut Graph) {
            #[derive(Debug)]
            struct DoubleInheritance {
                anc_a: Rid,
                anc_b: Rid,
                common_descs: Vec<Rid>,
            }

            let mut double_inheritances = graph
                .nodes
                .iter()
                .map(|node| node.value)
                .tuple_combinations()
                .filter_map(|(anc_a, anc_b)| {
                    if graph.is_descendant_of(&anc_a, &anc_b).is_some()
                        || graph.is_descendant_of(&anc_b, &anc_a).is_some()
                    {
                        // anc_a and anc_b are related
                        return None;
                    }
                    let common_descs = graph
                        .walk_descendants(&anc_a)
                        .filter(|desc| graph.is_descendant_of(desc, &anc_b).is_some())
                        .filter(|desc| {
                            !graph.parents_of(desc).iter().any(|parent| {
                                graph.is_descendant_of(parent, &anc_a).is_some()
                                    && graph.is_descendant_of(parent, &anc_b).is_some()
                            })
                        })
                        .collect_vec();
                    if common_descs.len() >= 2 {
                        Some(DoubleInheritance {
                            anc_a,
                            anc_b,
                            common_descs,
                        })
                    } else {
                        None
                    }
                })
                .collect_vec();

            while let Some(double_inheritance) = double_inheritances.pop() {
                // cut
                double_inheritance
                    .common_descs
                    .iter()
                    .skip(1)
                    .for_each(|desc| {
                        let desc_node = graph.get_node_mut(desc);
                        let parent = desc_node.parents[0].expect("Parent must be present");
                        desc_node.parents[0] = None;
                        let parent_node = graph.get_node_mut(&parent);
                        parent_node.children.retain(|child| child != desc);
                    });

                fn is_resolved(graph: &Graph, double_inheritance: &mut DoubleInheritance) -> bool {
                    double_inheritance.common_descs.retain(|desc| {
                        graph
                            .is_descendant_of(desc, &double_inheritance.anc_a)
                            .is_some()
                            && graph
                                .is_descendant_of(desc, &double_inheritance.anc_b)
                                .is_some()
                    });
                    double_inheritance.common_descs.len() < 2
                }

                double_inheritances
                    .retain_mut(|double_inheritance| !is_resolved(graph, double_inheritance));
            }
        }
        // cut cycles
        let sources = self.sources.clone();
        sources
            .iter()
            .for_each(|child| cut_cycles(&mut self, child));
        update_sources(&mut self);
        // cut double inheritance
        cut_double_inheritance(&mut self);
        CutGraph(self)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct CutGraph(Graph);

impl CutGraph {
    pub fn layers(&self) -> Vec<Vec<Rid>> {
        let graph = &self.0;
        let start = match graph.sources.first() {
            Some(rid) => *rid,
            None => return Vec::new(), // no nodes, no layers
        };
        let mut layers = vec![vec![start]];
        let mut added = vec![start];
        while added.len() < graph.nodes.len() {
            // sweep down
            let mut new_layers = Vec::new();
            for layer in layers.iter() {
                let mut children: Vec<Rid> = Vec::new();
                for rid in layer.iter() {
                    let mut current_children = graph.children_of(rid).to_vec();
                    current_children.retain(|child| !added.contains(child));
                    added.extend(current_children.clone());
                    children.extend(current_children)
                }
                new_layers.push(children);
            }
            // add to layers
            layers.push(Vec::new());
            layers
                .iter_mut()
                .skip(1)
                .enumerate()
                .for_each(|(i, layer)| layer.append(&mut new_layers[i]));
            // cleanup
            if layers
                .last()
                .expect("Must have at least one layer")
                .is_empty()
            {
                layers.pop();
            }

            // sweep up
            let mut new_layers = Vec::new();
            for layer in layers.iter() {
                let mut parents: Vec<Rid> = Vec::new();
                for rid in layer.iter() {
                    let mut current_parents =
                        graph.parents_of(rid).into_iter().unique().collect_vec();
                    current_parents.retain(|parent| !added.contains(parent));
                    added.extend(current_parents.clone());
                    parents.extend(current_parents)
                }
                new_layers.push(parents);
            }
            // add to layers
            layers.insert(0, Vec::new());
            layers.iter_mut().enumerate().for_each(|(i, layer)| {
                if i != new_layers.len() {
                    layer.append(&mut new_layers[i])
                }
            });
            // cleanup
            if layers
                .first()
                .expect("Must have at least one layer")
                .is_empty()
            {
                layers.remove(0);
            }
        }
        assert_eq!(
            added.len(),
            graph.nodes.len(),
            "Number of nodes in layers and graph must be equal"
        );
        let parents_in_top_row: usize = layers[0]
            .iter()
            .map(|rid| graph.parents_of(rid).len())
            .sum();
        assert_eq!(parents_in_top_row, 0, "No parents in top row");
        layers
    }
}

pub fn person_layers(layers: &Vec<Vec<Rid>>, relationships: &[Relationship]) -> Vec<Vec<Pid>> {
    if layers.is_empty() {
        return Vec::new();
    }
    let mut person_layers = vec![Vec::new(); layers.len()];

    // add all children
    for (i, layer) in layers.iter().enumerate() {
        for rid in layer {
            let rel = relationships
                .iter()
                .find(|rel| rel.id == *rid)
                .expect("Relationship must exist");
            person_layers[i].extend(rel.children.clone());
        }
    }

    // add missing parents
    for (i, layer) in layers.iter().skip(1).enumerate() {
        let parents = layer
            .iter()
            .map(|rid| {
                relationships
                    .iter()
                    .find(|rel| rel.id == *rid)
                    .expect("Relationship must exist")
                    .parents
            })
            .collect_vec();

        let mut missing_partners = parents
            .iter()
            .filter_map(|parents| match parents {
                [Some(parent_a), Some(parent_b)] => Some((parent_a, parent_b)),
                _ => None,
            })
            .filter_map(|(parent_a, parent_b)| {
                match (
                    person_layers[i].iter().position(|pid| pid == parent_a),
                    person_layers[i].iter().position(|pid| pid == parent_b),
                ) {
                    (Some(_), Some(_)) => None,
                    (Some(pos_a), None) => Some((pos_a, parent_b)),
                    (None, Some(pos_b)) => Some((pos_b, parent_a)),
                    (None, None) => None,
                }
            })
            .sorted_by(|(pos_a, _), (pos_b, _)| Ord::cmp(pos_a, pos_b))
            .collect_vec();

        fn add_to_layer(layer: &mut Vec<Pid>, index: usize, acc: Vec<Pid>) {
            let len = acc.len();
            let middle = if len % 2 == 0 {
                // even
                len / 2
            } else {
                // uneven
                (len - 1) / 2
            };
            let mut offset = 0;
            acc.into_iter().enumerate().for_each(|(i, pid)| {
                let insertion_index = offset + index;
                assert!(insertion_index < layer.len());
                if i < middle {
                    layer.insert(insertion_index, pid);
                    offset += 1;
                } else {
                    layer.insert(insertion_index + 1, pid)
                }
            });
        }

        let mut offset = 0;
        let mut last_index = 0;
        let mut acc = Vec::new();
        while let Some((index, missing_partner)) = missing_partners.pop() {
            if index == last_index {
                acc.push(*missing_partner);
                continue;
            }
            let acc_len = acc.len();
            add_to_layer(&mut person_layers[i], last_index + offset, acc);
            offset += acc_len;
            last_index = index;
            acc = vec![*missing_partner];
        }
        add_to_layer(&mut person_layers[i], last_index, acc);

        let missing_singles = parents
            .iter()
            .filter_map(|parents| match parents {
                [Some(parent), None] => Some(parent),
                [None, Some(parent)] => Some(parent),
                _ => None,
            })
            .filter(|pid| !person_layers[i].contains(pid))
            .collect_vec();
        person_layers[i].extend(missing_singles);
    }

    // remove last layer if empty
    if person_layers
        .last()
        .expect("Must have at least one layer")
        .is_empty()
    {
        person_layers.pop();
    }

    // move partners together, if they do not both have parents
    for layer in person_layers.iter_mut().skip(1) {
        let leaves = layer
            .iter()
            .cloned()
            .enumerate()
            .filter(|(_, pid)| {
                let child_rel = relationships
                    .iter()
                    .find(|rel| rel.children.contains(pid))
                    .expect("Inconsistent relationships");
                child_rel.persons().len() == 1
            })
            .collect_vec();
        for (index, pid) in leaves {
            if let Some(partner_index) = layer.iter().position(|partner| {
                relationships.iter().any(|rel| {
                    let parents = rel.parents();
                    *partner != pid && parents.contains(partner) && parents.contains(&pid)
                })
            }) {
                let leaf = layer.remove(index);
                let insertion_index = if index <= partner_index {
                    partner_index
                } else {
                    partner_index + 1
                };
                assert!(insertion_index <= layer.len());
                layer.insert(insertion_index, leaf);
            }
        }
    }

    let nr_persons: usize = person_layers.iter().flatten().unique().count();
    assert_eq!(
        nr_persons,
        extract_persons(relationships).len(),
        "Number of persons in layer inconsistent"
    );
    person_layers
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
        let tree_data = read("test/graph/cycles.json");
        consistency::check(&tree_data).expect("Test data inconsistent");
        let graph = Graph::new(&tree_data.relationships);
        let cut_graph = graph.cut();
        assert_debug_snapshot!(cut_graph);
        let layers = cut_graph.layers();
        assert_debug_snapshot!(layers);
        let person_layers = person_layers(&layers, &tree_data.relationships);
        assert_debug_snapshot!(person_layers);
    }

    #[test]
    fn double_inheritance() {
        let tree_data = read("test/graph/double_inheritance.json");
        consistency::check(&tree_data).expect("Test data inconsistent");
        let graph = Graph::new(&tree_data.relationships);
        let cut_graph = graph.cut();
        assert_debug_snapshot!(cut_graph);
        let layers = cut_graph.layers();
        assert_debug_snapshot!(layers);
        let person_layers = person_layers(&layers, &tree_data.relationships);
        assert_debug_snapshot!(person_layers);
    }

    #[test]
    fn xs() {
        let tree_data = read("test/graph/xs.json");
        consistency::check(&tree_data).expect("Test data inconsistent");
        let graph = Graph::new(&tree_data.relationships);
        let cut_graph = graph.cut();
        assert_debug_snapshot!(cut_graph);
        let layers = cut_graph.layers();
        assert_debug_snapshot!(layers);
        let person_layers = person_layers(&layers, &tree_data.relationships);
        assert_debug_snapshot!(person_layers);
    }

    #[test]
    fn double_inheritance_and_xs() {
        let tree_data = read("test/graph/double_inheritance_and_xs.json");
        consistency::check(&tree_data).expect("Test data inconsistent");
        let graph = Graph::new(&tree_data.relationships);
        let cut_graph = graph.cut();
        assert_debug_snapshot!(cut_graph);
        let layers = cut_graph.layers();
        assert_debug_snapshot!(layers);
        let person_layers = person_layers(&layers, &tree_data.relationships);
        assert_debug_snapshot!(person_layers);
    }

    #[test]
    fn sort_long_chain() {
        let tree_data = read("test/graph/sort_long_chain.json");
        consistency::check(&tree_data).expect("Test data inconsistent");
        let graph = Graph::new(&tree_data.relationships);
        let cut_graph = graph.cut();
        assert_debug_snapshot!(cut_graph);
        let layers = cut_graph.layers();
        assert_debug_snapshot!(layers);
        let person_layers = person_layers(&layers, &tree_data.relationships);
        assert_debug_snapshot!(person_layers);
    }

    #[test]
    fn siblings() {
        let tree_data = read("test/graph/siblings.json");
        consistency::check(&tree_data).expect("Test data inconsistent");
        let graph = Graph::new(&tree_data.relationships);
        let cut_graph = graph.cut();
        assert_debug_snapshot!(cut_graph);
        let layers = cut_graph.layers();
        assert_debug_snapshot!(layers);
        let person_layers = person_layers(&layers, &tree_data.relationships);
        assert_debug_snapshot!(person_layers);
    }
}
