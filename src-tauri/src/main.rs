use std::error::Error;

use baumstamm::tree::{FamilyTree, PersonInfo};
use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
struct Cli {
    relationships: String,
    persons: String,

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
    Relationship(Relationship),
    Info(Info),
    RelationshipWithPartner(RelationshipWithPartner),
}

#[derive(Args)]
struct Child {
    rel_id: u128,
}

#[derive(Args)]
struct Info {
    person_id: u128,
    first_name: String,
    last_name: Option<String>,
    date_of_birth: Option<String>,
    date_of_death: Option<String>,
}

#[derive(Args)]
struct Parent {
    rel_id: u128,
}

#[derive(Args)]
struct Relationship {
    person_id: u128,
}

#[derive(Args)]
struct RelationshipWithPartner {
    person_id: u128,
    partner_id: u128,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Cli::parse();
    let mut tree = if args.new {
        FamilyTree::new(args.relationships, args.persons)?
    } else {
        FamilyTree::from_disk(args.relationships, args.persons)?
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
                Add::Info(info) => tree.add_info(
                    info.person_id,
                    Some(PersonInfo::new(
                        info.first_name,
                        info.last_name,
                        info.date_of_birth,
                        info.date_of_death,
                    )),
                )?,
                Add::RelationshipWithPartner(rel) => {
                    let rel_id = tree.add_rel_with_partner(rel.person_id, rel.partner_id)?;
                    println!("Added relationship {}", rel_id);
                }
                Add::Relationship(rel) => {
                    let rel_id = tree.add_rel(rel.person_id)?;
                    println!("Added relationship {}", rel_id);
                }
            },
        };
    };
    if let Some(out_file) = &args.output {
        tree.export_puml(out_file)?;
    }

    Ok(())
}
