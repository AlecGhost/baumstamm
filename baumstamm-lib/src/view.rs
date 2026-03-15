use crate::{error::InputError, FamilyTree, Person, PersonId, Relationship};
use itertools::Itertools;

type Pid = PersonId;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ViewOptions {
    pub show_partners: bool,
    pub show_partner_siblings: bool,
    pub show_ancestor_siblings: bool,
    pub descendent_gen_limit: ViewLimit,
    pub ancestor_gen_limit: ViewLimit,
}

impl Default for ViewOptions {
    fn default() -> Self {
        Self {
            show_partners: true,
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
        relationships.extend(missing_rels);
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
        let parent_rel = rels
            .iter()
            .find(|rel| rel.children.contains(root))
            .expect("Pid must be child of rel");
        let parent_rels = match options.ancestor_gen_limit {
            ViewLimit::Limit(0) => vec![Relationship {
                id: parent_rel.id,
                parents: [None, None],
                children: parent_rel
                    .children
                    .iter()
                    .filter(|c| options.show_ancestor_siblings || *c == root)
                    .cloned()
                    .collect_vec(),
            }],
            _ => vec![Relationship {
                children: parent_rel
                    .children
                    .iter()
                    .filter(|c| options.show_ancestor_siblings || *c == root)
                    .cloned()
                    .collect_vec(),
                ..parent_rel.clone()
            }],
        };
        let partnerships = match options.descendent_gen_limit {
            ViewLimit::Limit(0) => Vec::new(),
            _ => rels
                .iter()
                .filter(|rel| rel.parents().contains(root))
                .map(|rel| {
                    if options.show_partners {
                        rel.clone()
                    } else {
                        Relationship {
                            id: rel.id,
                            parents: [Some(*root), None],
                            children: rel.children.clone(),
                        }
                    }
                })
                .collect_vec(),
        };
        let ancestor_options = ViewOptions {
            ancestor_gen_limit: options.ancestor_gen_limit - 1,
            descendent_gen_limit: ViewLimit::Limit(0),
            ..options.clone()
        };
        let ancestors = parent_rels
            .iter()
            .flat_map(|rel| rel.parents())
            .flat_map(|pid| Self::filter_relationships(&pid, rels, &ancestor_options))
            .collect_vec();
        let descendent_options = ViewOptions {
            ancestor_gen_limit: ViewLimit::Limit(0),
            descendent_gen_limit: options.descendent_gen_limit - 1,
            ..options.clone()
        };
        let descendents = partnerships
            .iter()
            .flat_map(|rel| &rel.children)
            .flat_map(|pid| Self::filter_relationships(pid, rels, &descendent_options))
            .filter(|rel| !matches!(rel.parents, [None, None]))
            .collect_vec();
        [ancestors, parent_rels, partnerships, descendents].concat()
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
            show_ancestor_siblings: false,
            show_partner_siblings: false,
       },
       one_each with &ViewOptions {
            ancestor_gen_limit: ViewLimit::Limit(1),
            descendent_gen_limit: ViewLimit::Limit(1),
            show_partners: true,
            show_ancestor_siblings: false,
            show_partner_siblings: false,
       }
    );
}
