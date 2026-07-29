use baumstamm_lib::{
    FamilyTree, Person, PersonId, Relationship, RelationshipId, TreeData,
    error::{ConsistencyError, Error},
};

fn person_id(value: u128) -> PersonId {
    value.into()
}

fn relationship_id(value: u128) -> RelationshipId {
    value.into()
}

fn person(value: u128) -> Person {
    Person {
        id: person_id(value),
        info: None,
    }
}

fn relationship(id: u128, parents: [Option<u128>; 2], children: &[u128]) -> Relationship {
    Relationship {
        id: relationship_id(id),
        parents: parents.map(|parent| parent.map(person_id)),
        children: children.iter().copied().map(person_id).collect(),
    }
}

fn tree(relationships: Vec<Relationship>, persons: &[u128]) -> TreeData {
    TreeData {
        relationships,
        persons: persons.iter().copied().map(person).collect(),
    }
}

fn consistency_error(tree_data: TreeData) -> ConsistencyError {
    match FamilyTree::try_from(tree_data) {
        Err(Error::Consistency(error)) => error,
        Err(error) => panic!("expected consistency error, got {error:?}"),
        Ok(_) => panic!("expected consistency error, got a valid tree"),
    }
}

fn error_output(tree_data: TreeData) -> String {
    let error = consistency_error(tree_data);
    format!("Debug: {error:?}\nDisplay: {error}")
}

fn valid_output(tree_data: TreeData) -> String {
    match FamilyTree::try_from(tree_data) {
        Ok(tree) => format!(
            "Ok\nRelationships: {}\nPersons: {}",
            tree.get_relationships().len(),
            tree.get_persons().len(),
        ),
        Err(error) => format!("Err\nDebug: {error:?}\nDisplay: {error}"),
    }
}

fn balanced_tree(depth: usize) -> TreeData {
    let mut relationships = vec![relationship(1, [None, None], &[1])];
    let mut persons = vec![person(1)];
    let mut frontier = vec![1_u128];
    let mut next_person_id = 2_u128;
    let mut next_relationship_id = 2_u128;

    for _ in 0..depth {
        let mut next_frontier = Vec::with_capacity(frontier.len() * 2);
        for parent in frontier {
            let children = [next_person_id, next_person_id + 1];
            next_person_id += 2;
            persons.extend(children.map(person));
            next_frontier.extend(children);
            relationships.push(relationship(
                next_relationship_id,
                [Some(parent), None],
                &children,
            ));
            next_relationship_id += 1;
        }
        frontier = next_frontier;
    }

    TreeData {
        relationships,
        persons,
    }
}

#[test]
fn accepted_tree_data_round_trips_through_family_tree_save() {
    let tree_data = TreeData {
        relationships: vec![
            relationship(0x12, [Some(0xA), None], &[0xB, 0xC]),
            relationship(0x11, [None, None], &[0xA]),
        ],
        persons: vec![person(0xC), person(0xA), person(0xB)],
    };

    let family_tree = FamilyTree::try_from(tree_data).expect("fixture must be consistent");
    insta::assert_snapshot!(family_tree.save().expect("tree must serialize"));
}

#[test]
fn balanced_valid_trees_at_multiple_depths() {
    insta::assert_snapshot!("balanced_valid_depth_0", valid_output(balanced_tree(0)));
    insta::assert_snapshot!("balanced_valid_depth_2", valid_output(balanced_tree(2)));
    insta::assert_snapshot!("balanced_valid_depth_4", valid_output(balanced_tree(4)));
}

#[test]
fn relationship_id_exists_output() {
    insta::assert_snapshot!(error_output(tree(
        vec![
            relationship(1, [None, None], &[1]),
            relationship(1, [Some(1), None], &[2]),
        ],
        &[1, 2],
    )));
}

#[test]
fn relationship_exists_output() {
    insta::assert_snapshot!(error_output(tree(
        vec![
            relationship(1, [None, None], &[1, 2]),
            relationship(2, [Some(1), Some(2)], &[3]),
            relationship(3, [Some(2), Some(1)], &[4]),
        ],
        &[1, 2, 3, 4],
    )));
}

#[test]
fn self_reference_output() {
    insta::assert_snapshot!(error_output(tree(
        vec![
            relationship(1, [None, None], &[1]),
            relationship(2, [Some(1), Some(1)], &[2]),
        ],
        &[1, 2],
    )));
}

#[test]
fn direct_cycle_output() {
    insta::assert_snapshot!(error_output(tree(
        vec![relationship(1, [Some(1), None], &[1])],
        &[1],
    )));
}

#[test]
fn must_be_child_output() {
    insta::assert_snapshot!(error_output(tree(
        vec![relationship(1, [Some(1), None], &[2])],
        &[1, 2],
    )));
}

#[test]
fn more_than_once_child_output() {
    insta::assert_snapshot!(error_output(tree(
        vec![
            relationship(1, [Some(1), None], &[2]),
            relationship(2, [Some(3), None], &[2]),
            relationship(3, [None, None], &[3]),
        ],
        &[1, 2, 3],
    )));
}

#[test]
fn unconnected_output() {
    insta::assert_snapshot!(error_output(tree(
        vec![
            relationship(1, [None, None], &[1]),
            relationship(2, [Some(1), None], &[2]),
            relationship(3, [None, None], &[3]),
            relationship(4, [Some(3), None], &[4]),
        ],
        &[1, 2, 3, 4],
    )));
}

#[test]
fn indirect_cycle_output() {
    insta::assert_snapshot!(error_output(tree(
        vec![
            relationship(1, [Some(1), None], &[2]),
            relationship(2, [Some(2), None], &[3]),
            relationship(3, [Some(3), None], &[1]),
        ],
        &[1, 2, 3],
    )));
}

#[test]
fn person_id_exists_output() {
    insta::assert_snapshot!(error_output(tree(
        vec![relationship(1, [None, None], &[1, 2])],
        &[1, 1],
    )));
}

#[test]
fn different_number_of_persons_output() {
    insta::assert_snapshot!(error_output(tree(
        vec![relationship(1, [None, None], &[1, 2])],
        &[1],
    )));
}

#[test]
fn unmatched_quantity_output() {
    insta::assert_snapshot!(error_output(tree(
        vec![relationship(1, [None, None], &[1, 2])],
        &[1, 3],
    )));
}

#[test]
fn consistency_error_precedence_with_multiple_faults() {
    let relationship_id_before_person_id = error_output(tree(
        vec![
            relationship(1, [None, None], &[1]),
            relationship(1, [Some(1), None], &[2]),
        ],
        &[1, 1],
    ));
    let duplicate_parents_before_self_reference = error_output(tree(
        vec![
            relationship(1, [None, None], &[1]),
            relationship(2, [Some(1), Some(1)], &[2]),
            relationship(3, [Some(1), Some(1)], &[3]),
        ],
        &[1, 2, 3],
    ));
    let self_reference_before_direct_cycle =
        error_output(tree(vec![relationship(1, [Some(1), Some(1)], &[1])], &[1]));
    let direct_cycle_before_missing_child = error_output(tree(
        vec![relationship(1, [Some(1), Some(2)], &[1])],
        &[1, 2],
    ));
    let missing_child_before_duplicate_child = error_output(tree(
        vec![relationship(1, [Some(1), Some(3)], &[2, 2])],
        &[1, 2, 3],
    ));
    let duplicate_child_before_unconnected = error_output(tree(
        vec![
            relationship(1, [Some(1), None], &[2]),
            relationship(2, [Some(3), None], &[2]),
            relationship(3, [None, None], &[3]),
            relationship(4, [None, None], &[4]),
        ],
        &[1, 2, 3, 4],
    ));
    let unconnected_before_indirect_cycle = error_output(tree(
        vec![
            relationship(1, [Some(1), None], &[2]),
            relationship(2, [Some(2), None], &[1]),
            relationship(3, [None, None], &[3]),
        ],
        &[1, 2, 3],
    ));
    let person_id_before_quantity = error_output(tree(
        vec![relationship(1, [None, None], &[1, 2, 3])],
        &[1, 1],
    ));

    insta::assert_snapshot!(format!(
        "\
relationship id before person id:
{relationship_id_before_person_id}

duplicate parents before self reference:
{duplicate_parents_before_self_reference}

self reference before direct cycle:
{self_reference_before_direct_cycle}

direct cycle before missing child:
{direct_cycle_before_missing_child}

missing child before duplicate child:
{missing_child_before_duplicate_child}

duplicate child before unconnected:
{duplicate_child_before_unconnected}

unconnected before indirect cycle:
{unconnected_before_indirect_cycle}

person id before person quantity:
{person_id_before_quantity}"
    ));
}
