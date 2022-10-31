use std::{borrow::Borrow, rc::Rc};

use super::{Node, PersonId, Relationship, RelationshipId};
use crate::util::UniqueIterator;

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

pub(super) fn rel_children(
    rid: &RelationshipId,
    relationships: &[Relationship],
) -> Vec<RelationshipId> {
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

struct AncestorLevel {
    rid: RelationshipId,
    level: u32,
}

impl AncestorLevel {
    fn new(rid: RelationshipId, level: u32) -> Self {
        Self { rid, level }
    }
}

impl Node {
    fn is_ancestor_of(&self, other: &Rc<Self>) -> Option<AncestorLevel> {
        if self == other.borrow() {
            return Some(AncestorLevel::new(self.value, 0));
        }
        if self
            .children
            .borrow()
            .iter()
            .any(|child| child == other.borrow())
        {
            return Some(AncestorLevel::new(self.value, 1));
        }
        self.children
            .borrow()
            .iter()
            .filter_map(|child| child.is_ancestor_of(other))
            .reduce(|acc, child| AncestorLevel::new(self.value, acc.level.max(child.level) + 1))
    }

    fn get_nodes(&self) -> Vec<Rc<Self>> {
        let mut nodes: Vec<Rc<Node>> = self.children.borrow().iter().map(Rc::clone).collect();
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

pub(super) fn generate_node_graph(relationships: &[Relationship]) -> Rc<Node> {
    fn add_children(relationships: &[Relationship], nodes: &[Rc<Node>], parent: &mut Rc<Node>) {
        // return if children are already added
        if !parent.children.borrow().is_empty() {
            return;
        }
        // find child relationships and add them to the tree recursively
        rel_children(&parent.value, relationships)
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
                node.parents
                    .borrow_mut()
                    .iter_mut()
                    .for_each(|node_parent| *node_parent = Rc::downgrade(parent));
                // call function recursively for all children
                parent
                    .children
                    .borrow_mut()
                    .iter_mut()
                    .for_each(|child| add_children(relationships, nodes, child));
            });
    }

    let root = Rc::new(Node::new(0));
    let nodes: Vec<Rc<Node>> = relationships
        .iter()
        .map(|rel| Rc::new(Node::new(rel.id)))
        .collect();
    // add relationships with no parents to root node
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
            root.children.borrow_mut().push(Rc::clone(node));
            node.parents
                .borrow_mut()
                .iter_mut()
                .for_each(|parent| *parent = Rc::downgrade(&root));
        });
    // build tree/add rest of the nodes
    root.children
        .borrow_mut()
        .iter_mut()
        .for_each(|child| add_children(relationships, &nodes, child));

    root
}

fn cut_node_graph(root: Rc<Node>) -> Rc<Node> {
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
            let cycle_children: Vec<AncestorLevel> = children
                .iter()
                .filter_map(|child| child.is_ancestor_of(node))
                .collect();
            // if the node descends from more than one child, there is a cycle
            if cycle_children.len() > 1 {
                // find out, which of the child relationships has the longer edge to the node, and should therefore remain in the tree
                let longest_edge = cycle_children
                    .iter()
                    .reduce(|acc, child| if acc.level > child.level { acc } else { child })
                    .expect("Math broken")
                    .rid;
                // remove the pointers to the child relationships, where there is a cycle, except for the longest edge to the node
                children.retain(|child| {
                    child.value == longest_edge
                        || !cycle_children.iter().any(|tuple| child.value == tuple.rid)
                });
            }
        });
        // continue recursively through the tree
        children
            .iter_mut()
            .for_each(|child| cut_cycles(child, nodes));
    }

    let nodes = root.get_nodes();
    root.children
        .borrow_mut()
        .iter_mut()
        .for_each(|child| cut_cycles(child, &nodes));
    root
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::tree::{consistency, io};

    #[test]
    fn node_tree() {
        let tree_data = io::read("test/graph/node_tree.json").expect("Cannot read test file");
        consistency::check(&tree_data).expect("Test data inconsistent");
        let expected_result = std::fs::read_to_string("test/graph/node_tree_result.txt")
            .expect("Cannot read test file");
        let root = generate_node_graph(&tree_data.relationships);
        let root = cut_node_graph(root);
        assert_eq!(expected_result, format!("{:#?}", root));
    }
}
