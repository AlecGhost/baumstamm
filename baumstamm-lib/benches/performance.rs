use baumstamm_lib::{
    view::{View, ViewOptions},
    FamilyTree, Person, PersonId, Relationship, RelationshipId, TreeData,
};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::time::Duration;

const FAMILY_COUNTS: [usize; 3] = [31, 127, 511];

struct Fixture {
    json: String,
    tree: FamilyTree,
    root: PersonId,
    person_count: usize,
    relationship_count: usize,
}

fn person_id(family_index: usize, is_partner: bool, family_count: usize) -> PersonId {
    let offset = if is_partner { family_count } else { 0 };
    PersonId::from((family_index + offset + 1) as u128)
}

fn build_fixture(family_count: usize) -> Fixture {
    assert!((family_count + 1).is_power_of_two());

    let persons = (0..family_count)
        .map(|family_index| Person {
            id: person_id(family_index, false, family_count),
            info: None,
        })
        .chain((0..family_count / 2).map(|family_index| Person {
            id: person_id(family_index, true, family_count),
            info: None,
        }))
        .collect::<Vec<_>>();

    let mut next_relationship_id = 1_u128;
    let mut relationships = Vec::with_capacity(family_count + 1 + family_count / 2);

    let mut push_relationship = |parents: [Option<PersonId>; 2], children: Vec<PersonId>| {
        relationships.push(Relationship {
            id: RelationshipId::from(next_relationship_id),
            parents,
            children,
        });
        next_relationship_id += 1;
    };

    // The root person and every internal node's partner need an origin relationship
    // so every person is exactly one relationship's child. Leaf nodes have no
    // partner because they do not form a relationship of their own in this fixture.
    push_relationship([None, None], vec![person_id(0, false, family_count)]);
    for family_index in 0..(family_count / 2) {
        push_relationship(
            [None, None],
            vec![person_id(family_index, true, family_count)],
        );
    }

    // Complete binary family hierarchy: every non-leaf couple has two children.
    // Those children are the primary person in the two child family units.
    for family_index in 0..(family_count / 2) {
        let first_child = 2 * family_index + 1;
        let second_child = first_child + 1;
        push_relationship(
            [
                Some(person_id(family_index, false, family_count)),
                Some(person_id(family_index, true, family_count)),
            ],
            vec![
                person_id(first_child, false, family_count),
                person_id(second_child, false, family_count),
            ],
        );
    }

    let person_count = persons.len();
    let relationship_count = relationships.len();
    let tree_data = TreeData {
        relationships,
        persons,
    };
    let json = serde_json::to_string(&tree_data).expect("fixture must serialize");
    let tree = FamilyTree::try_from(json.as_str()).expect("fixture must be consistent");

    Fixture {
        json,
        tree,
        root: person_id(0, false, family_count),
        person_count,
        relationship_count,
    }
}

fn benchmark_family_tree(c: &mut Criterion) {
    let fixtures = FAMILY_COUNTS
        .into_iter()
        .map(build_fixture)
        .collect::<Vec<_>>();

    let mut parsing = c.benchmark_group("json_to_family_tree");
    for fixture in &fixtures {
        parsing.throughput(Throughput::Bytes(fixture.json.len() as u64));
        parsing.bench_with_input(
            BenchmarkId::new(
                "parse_and_validate",
                format!(
                    "{}_people_{}_relationships",
                    fixture.person_count, fixture.relationship_count
                ),
            ),
            fixture,
            |b, fixture| {
                b.iter(|| {
                    let tree = FamilyTree::try_from(black_box(fixture.json.as_str()))
                        .expect("fixture must remain consistent");
                    black_box(tree)
                });
            },
        );
    }
    parsing.finish();

    let options = ViewOptions::default();
    let mut view = c.benchmark_group("view_new_preparsed");
    for fixture in &fixtures {
        view.throughput(Throughput::Elements(fixture.person_count as u64));
        view.bench_with_input(
            BenchmarkId::new(
                "full_descendant_view",
                format!(
                    "{}_people_{}_relationships",
                    fixture.person_count, fixture.relationship_count
                ),
            ),
            fixture,
            |b, fixture| {
                b.iter(|| {
                    let result = View::new(
                        black_box(&fixture.tree),
                        black_box(fixture.root),
                        black_box(&options),
                    )
                    .expect("fixture root must be valid");
                    black_box(result)
                });
            },
        );
    }
    view.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .sample_size(10)
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(1));
    targets = benchmark_family_tree
}
criterion_main!(benches);
