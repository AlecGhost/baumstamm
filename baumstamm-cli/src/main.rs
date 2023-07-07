use baumstamm_lib::{error::Error, FamilyTree, PersonId, RelationshipId};
use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
struct Cli {
    data: String,

    #[command(subcommand)]
    action: Option<Action>,

    #[arg(short, long)]
    new: bool,

    #[arg(short, long)]
    output: Option<String>,
}

#[derive(Subcommand)]
enum Action {
    #[command(subcommand)]
    Add(Add),
}

#[derive(Subcommand)]
enum Add {
    Child(Child),
    Parent(Parent),
    NewRelationship(NewRelationship),
    RelationshipWithPartner(RelationshipWithPartner),
}

#[derive(Args)]
struct NewRelationship {
    person_id: PersonId,
}

#[derive(Args)]
struct Child {
    rel_id: RelationshipId,
}

#[derive(Args)]
struct Parent {
    rel_id: RelationshipId,
}

#[derive(Args)]
struct RelationshipWithPartner {
    person_id: RelationshipId,
    partner_id: RelationshipId,
}

fn main() -> Result<(), Error> {
    let args = Cli::parse();
    let mut tree = if args.new {
        FamilyTree::new(args.data)?
    } else {
        FamilyTree::from_disk(args.data)?
    };

    if let Some(action) = args.action {
        match action {
            Action::Add(add) => match add {
                Add::Child(child) => {
                    let child_id = tree.add_child(child.rel_id)?;
                    println!("Added child as {}", child_id);
                }
                Add::Parent(parent) => {
                    let result = tree.add_parent(parent.rel_id)?;
                    println!(
                        "Added parent as {} and child of relationship {}",
                        result.0, result.1
                    );
                }
                Add::NewRelationship(rel) => {
                    let rel_id = tree.add_new_relationship(rel.person_id)?;
                    println!("Added new relationship {}", rel_id);
                }
                Add::RelationshipWithPartner(rel) => {
                    let rel_id =
                        tree.add_relationship_with_partner(rel.person_id, rel.partner_id)?;
                    println!("Added relationship {}", rel_id);
                }
            },
        };
    };
    Ok(())
}
