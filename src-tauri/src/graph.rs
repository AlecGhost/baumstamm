use crate::grid::PersonInfo;
use crate::util::UniqueIterator;
use uuid::Uuid;

mod consistency;
mod io;

type PersonId = Uuid;
type RelationshipId = Uuid;

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct Relationship {
    id: RelationshipId,
    p1: Option<PersonId>,
    p2: Option<PersonId>,
    children: Vec<PersonId>,
}

impl Relationship {
    fn new(p1: Option<PersonId>, p2: Option<PersonId>, children: Vec<PersonId>) -> Self {
        Self {
            id: PersonId::new_v4(),
            p1,
            p2,
            children,
        }
    }

    fn parents(&self) -> Vec<PersonId> {
        vec![self.p1, self.p2].iter().flatten().cloned().collect()
    }

    fn persons(&self) -> Vec<PersonId> {
        self.parents()
            .iter()
            .cloned()
            .chain(self.children.clone())
            .collect()
    }

    fn descendants(&self, relationships: &[Relationship]) -> Vec<PersonId> {
        let mut descendants = self.children.clone();
        let mut index = 0;
        while index < descendants.len() {
            let descendant = &descendants[index].clone();
            relationships
                .iter()
                .filter(|rel| rel.parents().contains(descendant))
                .flat_map(|rel| rel.children.clone())
                .unique()
                .for_each(|child| {
                    if !descendants.contains(&child) {
                        descendants.push(child)
                    }
                });

            index += 1;
        }
        descendants
    }
}

fn extract_persons(relationships: &[Relationship]) -> Vec<PersonId> {
    let parents = relationships.iter().flat_map(|rel| rel.parents());
    let children = relationships.iter().flat_map(|rel| rel.children.to_vec());
    parents.chain(children).unique().collect()
}

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct Person {
    id: PersonId,
    info: Option<PersonInfo>,
}

impl Person {
    fn new() -> Self {
        Person {
            id: PersonId::new_v4(),
            info: None,
        }
    }

    fn add_info(self, info: PersonInfo) -> Self {
        Self {
            info: Some(info),
            ..self
        }
    }
}
