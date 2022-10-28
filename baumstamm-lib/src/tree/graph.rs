use std::rc::Rc;

use super::{Node, PersonId, Relationship, RelationshipId};
use crate::util::UniqueIterator;

pub(super) fn extract_persons(relationships: &[Relationship]) -> Vec<PersonId> {
    let parents = relationships.iter().flat_map(|rel| rel.parents());
    let children = relationships.iter().flat_map(|rel| rel.children.to_vec());
    parents.chain(children).unique().collect()
}

pub(super) fn child_relationship<'a>(
    id: &PersonId,
    relationships: &'a [Relationship],
) -> &'a Relationship {
    relationships
        .iter()
        .filter(|rel| rel.children.contains(id))
        .collect::<Vec<&Relationship>>()
        .first()
        // TODO: change signature to result
        .expect("Inconsistent data")
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

pub(super) fn generate_node_tree(relationships: &[Relationship]) -> Rc<Node> {
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

    let root = Rc::new(Node::new(0));
    let nodes: Vec<Rc<Node>> = relationships
        .iter()
        .map(|rel| Rc::new(Node::new(rel.id)))
        .collect();
    relationships
        .iter()
        // get relationships with not parents
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
        println!("{:#?}", generate_node_tree(&relationships));
    }
}
