use baumstamm_lib::{
    FamilyTree, Person, PersonId, Relationship, RelationshipId, TreeData,
    view::{View, ViewLimit, ViewOptions},
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

fn representative_tree() -> FamilyTree {
    // The deliberately non-topological relationship and person order makes these
    // snapshots sensitive to all three externally observable orderings.
    let tree_data = TreeData {
        relationships: vec![
            relationship(0x105, [Some(0x6), Some(0x10)], &[0x11, 0x12]),
            relationship(0x102, [Some(0x1), Some(0x2)], &[0x3, 0x4]),
            relationship(0x107, [Some(0x20), Some(0x21)], &[0x10, 0x16]),
            relationship(0x101, [None, None], &[0x1, 0x2, 0x5, 0x8, 0x13, 0x20, 0x21]),
            relationship(0x106, [Some(0x11), Some(0x13)], &[0x14, 0x15]),
            relationship(0x103, [Some(0x3), Some(0x5)], &[0x6, 0x7]),
            relationship(0x108, [Some(0x10), Some(0x16)], &[0x17]),
            relationship(0x104, [Some(0x4), Some(0x8)], &[0x9]),
        ],
        persons: [
            0x17, 0x6, 0x10, 0x1, 0x14, 0x3, 0x21, 0x12, 0x5, 0x16, 0x2, 0x15, 0x8, 0x11, 0x20,
            0x4, 0x13, 0x7, 0x9,
        ]
        .into_iter()
        .map(person)
        .collect(),
    };

    FamilyTree::try_from(tree_data).expect("representative fixture must be consistent")
}

fn output(tree: &FamilyTree, root: u128, options: &ViewOptions) -> TreeData {
    View::new(tree, person_id(root), options)
        .expect("fixture root must be valid")
        .into()
}

fn serialized_output(tree: &FamilyTree, root: u128, options: &ViewOptions) -> String {
    serde_json::to_string(&output(tree, root, options)).expect("view output must serialize")
}

#[derive(serde::Serialize)]
struct OrderedViewShape {
    relationships: Vec<Relationship>,
    persons: Vec<PersonId>,
}

fn serialized_shape(tree: &FamilyTree, root: PersonId, options: &ViewOptions) -> String {
    let view = View::new(tree, root, options).expect("fixture root must be valid");
    let shape = OrderedViewShape {
        relationships: view.get_relationships().to_vec(),
        persons: view.get_persons().iter().map(|person| person.id).collect(),
    };
    serde_json::to_string(&shape).expect("view shape must serialize")
}

#[test]
fn default_unlimited_view_output() {
    let tree = representative_tree();
    insta::assert_snapshot!(serialized_output(&tree, 0x6, &ViewOptions::default()));
}

#[test]
fn finite_ancestor_and_descendant_limit_output() {
    let tree = representative_tree();
    let cases = [(0, 0), (0, 1), (1, 0), (1, 1), (2, 3), (3, 2)];
    let outputs = cases
        .into_iter()
        .map(|(ancestor_limit, descendant_limit)| {
            let options = ViewOptions {
                ancestor_gen_limit: ViewLimit::Limit(ancestor_limit),
                descendent_gen_limit: ViewLimit::Limit(descendant_limit),
                ..ViewOptions::default()
            };
            format!(
                "ancestors={ancestor_limit}, descendants={descendant_limit}\n{}",
                serialized_output(&tree, 0x6, &options)
            )
        })
        .collect::<Vec<_>>();

    insta::assert_snapshot!(outputs.join("\n"));
}

#[test]
fn view_option_feature_matrix_output() {
    let tree = representative_tree();
    let mut outputs = Vec::new();

    for show_partners in [false, true] {
        for show_siblings in [false, true] {
            for show_ancestor_siblings in [false, true] {
                for show_partner_siblings in [false, true] {
                    let options = ViewOptions {
                        show_partners,
                        show_siblings,
                        show_partner_siblings,
                        show_ancestor_siblings,
                        ancestor_gen_limit: ViewLimit::Limit(2),
                        descendent_gen_limit: ViewLimit::Limit(2),
                    };
                    outputs.push(format!(
                        "partners={show_partners}, siblings={show_siblings}, \
                         ancestor_siblings={show_ancestor_siblings}, \
                         partner_siblings={show_partner_siblings}\n{}",
                        serialized_output(&tree, 0x6, &options)
                    ));
                }
            }
        }
    }

    insta::assert_snapshot!(outputs.join("\n"));
}

#[test]
fn missing_parent_synthesis_and_output_ordering() {
    let tree = representative_tree();
    let options = ViewOptions {
        show_partners: true,
        show_siblings: true,
        show_partner_siblings: true,
        show_ancestor_siblings: false,
        ancestor_gen_limit: ViewLimit::Limit(0),
        descendent_gen_limit: ViewLimit::Limit(2),
    };

    insta::assert_snapshot!(serialized_output(&tree, 0x6, &options));
}

#[test]
fn overlapping_royal_ancestry_and_descendancy_output() {
    let tree = FamilyTree::try_from(include_str!("../../examples/royals/royals.json"))
        .expect("royals fixture must be consistent");
    let root =
        PersonId::try_from("21E21B70321109BB854E2BC594F5A7E8").expect("root id must be valid");

    insta::assert_snapshot!(
        "overlapping_royals_default",
        serialized_shape(&tree, root, &ViewOptions::default())
    );

    let finite_options = ViewOptions {
        show_partners: true,
        show_siblings: true,
        show_partner_siblings: true,
        show_ancestor_siblings: true,
        ancestor_gen_limit: ViewLimit::Limit(8),
        descendent_gen_limit: ViewLimit::Limit(8),
    };
    insta::assert_snapshot!(
        "overlapping_royals_finite",
        serialized_shape(&tree, root, &finite_options)
    );
}
