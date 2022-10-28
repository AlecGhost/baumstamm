use std::rc::Rc;

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
    id: &RelationshipId,
    relationships: &[Relationship],
) -> Vec<RelationshipId> {
    let current = relationships
        .iter()
        .find(|rel| rel.id == *id)
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

pub enum NodeTreeType {
    Full,
    TopCut,
}

impl Node {
    fn is_anchestor_of(&self, other: &Rc<Self>) -> Option<(RelationshipId, u32)> {
        // println!("Comparing {}, {}", self.value, other.value);
        if self.value == other.value {
            return Some((self.value, 0));
        }
        if self
            .children
            .borrow()
            .iter()
            .any(|child| child.value == other.value)
        {
            return Some((self.value, 1));
        }
        self.children
            .borrow()
            .iter()
            .filter_map(|child| child.is_anchestor_of(other))
            .reduce(|acc, child| (self.value, acc.1.max(child.1) + 1))
    }
}

pub(super) fn generate_node_tree(
    relationships: &[Relationship],
    tree_type: NodeTreeType,
) -> Rc<Node> {
    fn add_children(relationships: &[Relationship], nodes: &[Rc<Node>], parent: &mut Rc<Node>) {
        if !parent.children.borrow().is_empty() {
            return;
        }
        rel_children(&parent.value, relationships)
            .iter()
            .for_each(|id| {
                let node = nodes
                    .iter()
                    .find(|node| node.value == *id)
                    .expect("Node creation failed");
                parent.children.borrow_mut().push(Rc::clone(node));
                node.parents
                    .borrow_mut()
                    .iter_mut()
                    .for_each(|node_parent| *node_parent = Rc::downgrade(parent));
                parent
                    .children
                    .borrow_mut()
                    .iter_mut()
                    .for_each(|child| add_children(relationships, nodes, child));
            });
    }

    fn cut_cycles(parent: &mut Rc<Node>, nodes: &[Rc<Node>]) {
        let mut children = parent.children.borrow_mut();
        if children.is_empty() {
            return;
        }
        nodes.iter().for_each(|node| {
            let cycle_children: Vec<(RelationshipId, u32)> = children
                .iter()
                .filter_map(|child| child.is_anchestor_of(node))
                .collect();
            if cycle_children.len() > 1 {
                let longest_edge = cycle_children
                    .iter()
                    .reduce(|acc, child| if acc.1 > child.1 { acc } else { child })
                    .expect("Math broken")
                    .0;
                children.retain(|child| {
                    child.value == longest_edge
                        || !cycle_children.iter().any(|tuple| child.value == tuple.0)
                });
            }
        });
        children
            .iter_mut()
            .for_each(|child| cut_cycles(child, nodes));
    }

    let root = Rc::new(Node::new(0));
    let nodes: Vec<Rc<Node>> = relationships
        .iter()
        .map(|rel| Rc::new(Node::new(rel.id)))
        .collect();
    relationships
        .iter()
        // get relationships with no parents
        .filter(|rel| rel.parents().is_empty())
        .map(|rel| rel.id)
        .for_each(|id| {
            // add relationships with no parents to root node
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
    root.children
        .borrow_mut()
        .iter_mut()
        .for_each(|child| add_children(relationships, &nodes, child));
    if let NodeTreeType::TopCut = tree_type {
        root.children
            .borrow_mut()
            .iter_mut()
            .for_each(|child| cut_cycles(child, &nodes))
    }
    root
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::tree::io;

    #[test]
    fn node_tree() {
        let relationships =
            io::read_relationships("test/generation_matrix.json").expect("Cannot read test file");
        println!(
            "{:#?}",
            generate_node_tree(&relationships, NodeTreeType::TopCut)
        );
    }
}
