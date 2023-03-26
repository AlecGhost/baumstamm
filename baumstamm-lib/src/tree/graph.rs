use super::{PersonId, Relationship, RelationshipId};
use crate::util::UniqueIterator;
use std::{
    borrow::Borrow,
    cell::RefCell,
    rc::{Rc, Weak},
};

pub(super) fn extract_persons(relationships: &[Relationship]) -> Vec<PersonId> {
    let parents = relationships.iter().flat_map(|rel| rel.parents());
    let children = relationships.iter().flat_map(|rel| rel.children.to_vec());
    parents.chain(children).unique().collect()
}

pub(super) fn parent_relationships<'a>(
    id: &PersonId,
    relationships: &'a [Relationship],
) -> Vec<&'a Relationship> {
    relationships
        .iter()
        .filter(|rel| rel.parents().contains(id))
        .collect()
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

#[derive(Debug, Default)]
pub(super) struct Tree {
    top_level_nodes: Vec<Rc<Node>>,
}

impl Tree {
    fn get_nodes(&self) -> Vec<Rc<Node>> {
        let mut nodes: Vec<Rc<Node>> = self.top_level_nodes.iter().map(Rc::clone).collect();
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
}

struct Node {
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
        if self
            .children
            .borrow()
            .iter()
            .any(|child| child == other.borrow())
        {
            return Some(1);
        }
        self.children
            .borrow()
            .iter()
            .filter_map(|child| child.is_ancestor_of(other))
            .reduce(|acc, level| acc.max(level) + 1)
    }
}

pub(super) fn generate_tree(relationships: &[Relationship]) -> Tree {
    fn add_children(relationships: &[Relationship], nodes: &[Rc<Node>], parent: &mut Rc<Node>) {
        // return if children are already added
        if !parent.children.borrow().is_empty() {
            return;
        }
        // find child relationships and add them to the tree recursively
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

    let mut tree = Tree::default();
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
            tree.top_level_nodes.push(Rc::clone(node));
        });
    // build tree/add rest of the nodes
    tree.top_level_nodes
        .iter_mut()
        .for_each(|child| add_children(relationships, &nodes, child));
    tree
}

fn cut_tree(tree: &mut Tree) {
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
            let cycle_children: Vec<(RelationshipId, usize)> = children
                .iter()
                .filter_map(|child| child.is_ancestor_of(node).map(|level| (child.value, level)))
                .collect();
            // if the node descends from more than one child, there is a cycle
            if cycle_children.len() > 1 {
                // find out, which of the child relationships has the longer edge to the node, and should therefore remain in the tree
                let has_longest_edge = cycle_children
                    .iter()
                    .reduce(|acc, child| if acc.1 > child.1 { acc } else { child })
                    .expect("Math broken")
                    .0;
                // remove the pointers to the child relationships, where there is a cycle, except for the longest edge to the node
                let is_cycle_child =
                    |child: RelationshipId| cycle_children.iter().any(|tuple| child == tuple.0);
                children.retain(|child| {
                    if child.value == has_longest_edge || !is_cycle_child(child.value) {
                        true
                    } else {
                        child
                            .parents
                            .borrow_mut()
                            .iter_mut()
                            .for_each(|cuttable_parent| {
                                if let Some(p) = cuttable_parent.upgrade() {
                                    if &p == parent {
                                        *cuttable_parent = Weak::new();
                                    }
                                }
                            });
                        false
                    }
                });
            }
        });
        // continue recursively through the tree
        children
            .iter_mut()
            .for_each(|child| cut_cycles(child, nodes));
    }

    let nodes = tree.get_nodes();
    tree.top_level_nodes
        .iter_mut()
        .for_each(|child| cut_cycles(child, &nodes));
    let mut new_top_level_nodes: Vec<Rc<Node>> = nodes
        .iter()
        .filter(|node| !tree.top_level_nodes.contains(node))
        .filter(|node| {
            let parents = node.parents.borrow();
            let no_parents = parents[0].upgrade().is_none() && parents[1].upgrade().is_none();
            if no_parents {}
            no_parents
        })
        .map(Rc::clone)
        .collect();
    tree.top_level_nodes.append(&mut new_top_level_nodes);
}

fn generations(tree: &Tree) -> Vec<Vec<RelationshipId>> {
    fn add_gen_rec(
        node: &Node,
        generations: &mut Vec<Vec<RelationshipId>>,
        origin: Option<RelationshipId>,
        level: usize,
    ) {
        if level == generations.len() {
            generations.push(Vec::new());
        }
        generations[level].push(node.value);
        node.parents.borrow().iter().for_each(|parent| {
            if let Some(parent_node) = parent.upgrade() {
                if Some(parent_node.value) != origin && parent_node.value != 0 {
                    let next_level;
                    if level == 0 {
                        generations.insert(0, Vec::new());
                        next_level = 0;
                    } else {
                        next_level = level - 1;
                    }
                    add_gen_rec(&parent_node, generations, Some(node.value), next_level);
                }
            }
        });
        node.children.borrow().iter().for_each(|child| {
            if Some(child.value) != origin {
                add_gen_rec(child, generations, Some(node.value), level + 1)
            }
        });
    }
    let mut generations = vec![Vec::new()];
    let start = tree
        .top_level_nodes
        .first()
        .expect("Root node must have at least one child");
    add_gen_rec(start, &mut generations, None, 0);
    generations
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::tree::{consistency, io};
    use insta::assert_debug_snapshot;

    #[test]
    fn complete_tree_1() {
        let tree_data = io::read("test/graph/node_tree.json").expect("Cannot read test file");
        consistency::check(&tree_data).expect("Test data inconsistent");
        let mut tree = generate_tree(&tree_data.relationships);
        cut_tree(&mut tree);
        assert_debug_snapshot!(tree);
        let gens = generations(&tree);
        assert_debug_snapshot!(gens);
    }
}
