use baumstamm_lib::{FamilyTree, PersonId, RelationshipId};
use clap::{Args, Parser, Subcommand};
use std::{error::Error, fs};

#[derive(Parser)]
struct Cli {
    file: String,

    #[command(subcommand)]
    action: Option<Action>,

    #[arg(short, long)]
    new: bool,
}

#[derive(Subcommand)]
enum Action {
    #[command(subcommand)]
    Add(Add),
    #[command(subcommand)]
    Info(Info),
    #[command(subcommand)]
    Show(Show),
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
    person_id: PersonId,
    partner_id: PersonId,
}

#[derive(Subcommand)]
enum Info {
    Insert(InsertInfo),
    Remove(RemoveInfo),
}

#[derive(Args)]
struct InsertInfo {
    person_id: PersonId,
    key: String,
    value: String,
}

#[derive(Args)]
struct RemoveInfo {
    person_id: PersonId,
    key: String,
}

#[derive(Subcommand)]
enum Show {
    Persons,
    Relationships,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Cli::parse();
    let mut tree = if args.new {
        let tree = FamilyTree::new();
        let json_str = tree.save()?;
        fs::write(args.file, json_str)?;
        tree
    } else {
        let json_str = fs::read_to_string(args.file)?;
        FamilyTree::from_string(&json_str)?
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
            Action::Info(info) => match info {
                Info::Insert(insert) => {
                    tree.insert_info(insert.person_id, insert.key.clone(), insert.value.clone())?;
                    println!(
                        "Inserted \"{}\": \"{}\" to {}",
                        insert.key, insert.value, insert.person_id
                    );
                }
                Info::Remove(remove) => {
                    let value = tree.remove_info(remove.person_id, &remove.key)?;
                    println!(
                        "Removed \"{}\": \"{}\" from {}",
                        remove.key, value, remove.person_id
                    );
                }
            },
            Action::Show(show) => match show {
                Show::Persons => println!("Persons: {:#?}", tree.get_persons()),
                Show::Relationships => println!("Relationships: {:#?}", tree.get_relationships()),
            },
        };
    };
    Ok(())
}
