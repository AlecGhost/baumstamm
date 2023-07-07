use baumstamm_lib::tree::FamilyTree;
use clap::{Args, Parser, Subcommand};
use std::error::Error;

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
    Relationship(Relationship),
    RelationshipWithPartner(RelationshipWithPartner),
}

#[derive(Args)]
struct Child {
    rel_id: u128,
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
    Ok(())
}
