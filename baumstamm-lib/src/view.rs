use crate::{error::InputError, FamilyTree, Person, PersonId, Relationship};
use itertools::Itertools;
use std::collections::HashMap;

type Pid = PersonId;

struct RelationshipCollector {
    relationships: Vec<Relationship>,
    indices: HashMap<crate::RelationshipId, usize>,
}

impl RelationshipCollector {
    fn new() -> Self {
        Self {
            relationships: Vec::new(),
            indices: HashMap::new(),
        }
    }

    fn insert(&mut self, relationship: Relationship) {
        if let Some(index) = self.indices.get(&relationship.id).copied() {
            let existing = &mut self.relationships[index];
            for child in relationship.children {
                if !existing.children.contains(&child) {
                    existing.children.push(child);
                }
            }
            if relationship.parents.iter().flatten().count() > existing.parents().len() {
                existing.parents = relationship.parents;
            }
        } else {
            self.indices
                .insert(relationship.id, self.relationships.len());
            self.relationships.push(relationship);
        }
    }

    fn into_relationships(self) -> Vec<Relationship> {
        self.relationships
    }
}

#[derive(Default)]
struct Traversal {
    ancestors: HashMap<Pid, ViewLimit>,
    descendents: HashMap<Pid, ViewLimit>,
}

impl Traversal {
    fn should_expand(visited: &mut HashMap<Pid, ViewLimit>, pid: Pid, limit: ViewLimit) -> bool {
        let already_covered = visited
            .get(&pid)
            .is_some_and(|previous| match (*previous, limit) {
                (ViewLimit::Unlimited, _) => true,
                (_, ViewLimit::Unlimited) => false,
                (ViewLimit::Limit(previous), ViewLimit::Limit(current)) => previous >= current,
            });
        if already_covered {
            false
        } else {
            visited.insert(pid, limit);
            true
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ViewOptions {
    pub show_partners: bool,
    pub show_siblings: bool,
    pub show_partner_siblings: bool,
    pub show_ancestor_siblings: bool,
    pub descendent_gen_limit: ViewLimit,
    pub ancestor_gen_limit: ViewLimit,
}

impl Default for ViewOptions {
    fn default() -> Self {
        Self {
            show_partners: true,
            show_siblings: false,
            show_partner_siblings: false,
            show_ancestor_siblings: false,
            ancestor_gen_limit: ViewLimit::default(),
            descendent_gen_limit: ViewLimit::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub enum ViewLimit {
    #[default]
    Unlimited,
    Limit(usize),
}

impl std::ops::Sub<usize> for ViewLimit {
    type Output = ViewLimit;
    fn sub(self, rhs: usize) -> Self::Output {
        match self {
            ViewLimit::Limit(0) => ViewLimit::Limit(0),
            ViewLimit::Limit(limit) => ViewLimit::Limit(limit - rhs),
            ViewLimit::Unlimited => ViewLimit::Unlimited,
        }
    }
}

#[derive(Clone, Debug)]
pub struct View<'a> {
    relationships: Vec<Relationship>,
    persons: Vec<&'a Person>,
}

impl View<'_> {
    pub fn new<'a>(
        tree: &'a FamilyTree,
        root: Pid,
        options: &ViewOptions,
    ) -> Result<View<'a>, InputError> {
        if !tree
            .get_persons()
            .iter()
            .map(|person| person.id)
            .any(|pid| pid == root)
        {
            return Err(InputError::InvalidPersonId);
        }
        let mut relationships =
            Self::filter_relationships(&root, tree.get_relationships(), options);
        let parent_pids = relationships
            .iter()
            .flat_map(Relationship::parents)
            .unique()
            .collect_vec();
        let child_pids = relationships
            .iter()
            .flat_map(|rel| &rel.children)
            .unique()
            .collect_vec();
        let missing_parent = parent_pids
            .iter()
            .filter(|parent| !child_pids.contains(parent))
            .collect_vec();
        let missing_rels = tree
            .get_relationships()
            .iter()
            .filter(|rel| {
                missing_parent
                    .iter()
                    .any(|parent| rel.children.contains(parent))
            })
            .map(|rel| Relationship {
                id: rel.id,
                parents: [None, None],
                children: rel
                    .children
                    .iter()
                    .filter(|c| options.show_partner_siblings || missing_parent.contains(c))
                    .cloned()
                    .collect_vec(),
            });
        let mut collector = RelationshipCollector::new();
        relationships
            .into_iter()
            .chain(missing_rels)
            .for_each(|relationship| collector.insert(relationship));
        relationships = collector.into_relationships();
        let persons = Self::filter_persons(tree, &relationships);
        Ok(View {
            relationships,
            persons,
        })
    }

    fn filter_relationships(
        root: &Pid,
        rels: &[Relationship],
        options: &ViewOptions,
    ) -> Vec<Relationship> {
        let mut collector = RelationshipCollector::new();
        let mut traversal = Traversal::default();
        Self::collect_ancestors(
            *root,
            rels,
            options.ancestor_gen_limit,
            options.show_siblings,
            options.show_ancestor_siblings,
            &mut traversal,
            &mut collector,
        );
        Self::collect_descendents(
            *root,
            rels,
            options.descendent_gen_limit,
            options.show_partners,
            &mut traversal,
            &mut collector,
        );
        collector.into_relationships()
    }

    fn collect_ancestors(
        root: Pid,
        rels: &[Relationship],
        limit: ViewLimit,
        show_siblings: bool,
        show_ancestor_siblings: bool,
        traversal: &mut Traversal,
        collector: &mut RelationshipCollector,
    ) {
        let parent_rel = rels
            .iter()
            .find(|rel| rel.children.contains(&root))
            .expect("Pid must be child of rel");
        let parent_rel = match limit {
            ViewLimit::Limit(0) => Relationship {
                id: parent_rel.id,
                parents: [None, None],
                children: parent_rel
                    .children
                    .iter()
                    .filter(|child| show_siblings || **child == root)
                    .cloned()
                    .collect_vec(),
            },
            _ => Relationship {
                children: parent_rel
                    .children
                    .iter()
                    .filter(|child| show_siblings || **child == root)
                    .cloned()
                    .collect_vec(),
                ..parent_rel.clone()
            },
        };

        if !matches!(limit, ViewLimit::Limit(0))
            && Traversal::should_expand(&mut traversal.ancestors, root, limit)
        {
            for parent in parent_rel.parents() {
                Self::collect_ancestors(
                    parent,
                    rels,
                    limit - 1,
                    show_ancestor_siblings,
                    show_ancestor_siblings,
                    traversal,
                    collector,
                );
            }
        }
        collector.insert(parent_rel);
    }

    fn collect_descendents(
        root: Pid,
        rels: &[Relationship],
        limit: ViewLimit,
        show_partners: bool,
        traversal: &mut Traversal,
        collector: &mut RelationshipCollector,
    ) {
        if matches!(limit, ViewLimit::Limit(0))
            || !Traversal::should_expand(&mut traversal.descendents, root, limit)
        {
            return;
        }

        let partnerships = rels
            .iter()
            .filter(|relationship| relationship.parents().contains(&root))
            .map(|relationship| {
                if show_partners {
                    relationship.clone()
                } else {
                    Relationship {
                        id: relationship.id,
                        parents: [Some(root), None],
                        children: relationship.children.clone(),
                    }
                }
            })
            .collect_vec();

        for partnership in partnerships {
            let children = partnership.children.clone();
            collector.insert(partnership);
            for child in children {
                Self::collect_descendents(
                    child,
                    rels,
                    limit - 1,
                    show_partners,
                    traversal,
                    collector,
                );
            }
        }
    }

    fn filter_persons<'a>(tree: &'a FamilyTree, rels: &[Relationship]) -> Vec<&'a Person> {
        let pids = rels
            .iter()
            .flat_map(|rel| rel.persons())
            .unique()
            .collect_vec();
        tree.get_persons()
            .iter()
            .filter(|person| pids.contains(&person.id))
            .collect_vec()
    }

    pub fn get_relationships(&self) -> &[Relationship] {
        &self.relationships
    }

    pub fn get_persons(&self) -> &[&Person] {
        &self.persons
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(file_name: &str) -> crate::TreeData {
        let json_data = std::fs::read_to_string(file_name).expect("Cannot read test file");
        crate::io::read(&json_data).expect("Cannot convert test file")
    }

    macro_rules! test_files {
        ($($name:ident with $options:expr),*) => {
            $(
                #[test]
                fn $name() {
                    let file = format!("test/view/{}.json", stringify!($name));
                    let tree = crate::tree::FamilyTree::try_from(read(&file)).expect("Invalid tree data");
                    let view = crate::view::View::new(&tree, crate::PersonId(0), $options).expect("Invalid root");
                    insta::assert_debug_snapshot!(crate::tree::FamilyTree::from(view));
                }
            )*
        };
    }

    test_files!(
       zero_limits with &ViewOptions {
            ancestor_gen_limit: ViewLimit::Limit(0),
            descendent_gen_limit: ViewLimit::Limit(0),
            show_partners: true,
            show_siblings: false,
            show_ancestor_siblings: false,
            show_partner_siblings: false,
       },
       one_each with &ViewOptions {
            ancestor_gen_limit: ViewLimit::Limit(1),
            descendent_gen_limit: ViewLimit::Limit(1),
            show_partners: true,
            show_siblings: false,
            show_ancestor_siblings: false,
            show_partner_siblings: false,
       }
    );

    #[test]
    fn overlapping_ancestor_and_descendant_paths_have_unique_relationships() {
        let tree = FamilyTree::try_from(include_str!("../../examples/royals/royals.json"))
            .expect("Invalid tree data");
        let root =
            PersonId::try_from("21E21B70321109BB854E2BC594F5A7E8").expect("Invalid person id");

        let view = View::new(&tree, root, &ViewOptions::default()).expect("Invalid root");
        assert_eq!(
            view.relationships.len(),
            view.relationships
                .iter()
                .map(|relationship| relationship.id)
                .unique()
                .count()
        );

        let _ = FamilyTree::from(view);
    }

    #[test]
    fn finite_overlapping_paths_have_unique_relationships() {
        let tree = FamilyTree::try_from(include_str!("../../examples/royals/royals.json"))
            .expect("Invalid tree data");
        let root =
            PersonId::try_from("21E21B70321109BB854E2BC594F5A7E8").expect("Invalid person id");
        let options = ViewOptions {
            show_partners: true,
            show_siblings: true,
            show_partner_siblings: true,
            show_ancestor_siblings: true,
            ancestor_gen_limit: ViewLimit::Limit(8),
            descendent_gen_limit: ViewLimit::Limit(8),
        };

        let view = View::new(&tree, root, &options).expect("Invalid root");
        assert_eq!(
            view.relationships.len(),
            view.relationships
                .iter()
                .map(|relationship| relationship.id)
                .unique()
                .count()
        );

        let _ = FamilyTree::from(view);
    }

    #[test]
    fn got_jaehaerys_default_both_view_is_finite_and_valid() {
        let tree = FamilyTree::try_from(include_str!("../../examples/got/got.json"))
            .expect("Invalid tree data");
        let root =
            PersonId::try_from("9F150863B852978059445D9D56DF75D9").expect("Invalid person id");

        let view = View::new(&tree, root, &ViewOptions::default()).expect("Invalid root");
        assert_eq!(view.persons.len(), 125);
        assert_eq!(view.relationships.len(), 80);
        assert_eq!(
            view.relationships.len(),
            view.relationships
                .iter()
                .map(|relationship| relationship.id)
                .unique()
                .count()
        );

        let _ = FamilyTree::from(view);
    }
}
