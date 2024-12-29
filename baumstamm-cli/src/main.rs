use baumstamm_lib::{
    graph::Graph,
    view::{View, ViewLimit},
    FamilyTree, PersonId, RelationshipId,
};
use clap::{Args, Parser, Subcommand};
use std::{error::Error, fs, path::Path};

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
    View(ViewOptions),
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
    person_id: String,
}

#[derive(Args)]
struct Child {
    rel_id: String,
}

#[derive(Args)]
struct Parent {
    rel_id: String,
}

#[derive(Args)]
struct RelationshipWithPartner {
    person_id: String,
    partner_id: String,
}

#[derive(Subcommand)]
enum Info {
    Insert(InsertInfo),
    Remove(RemoveInfo),
}

#[derive(Args)]
struct InsertInfo {
    person_id: String,
    key: String,
    value: String,
}

#[derive(Args)]
struct RemoveInfo {
    person_id: String,
    key: String,
}

#[derive(Subcommand)]
enum Show {
    Persons,
    Relationships,
    Layers,
    PersonLayers,
}

#[derive(Args)]
struct ViewOptions {
    root: String,
    #[arg(short='p', long)]
    show_partners: bool,
    #[arg(short='u', long)]
    show_ancestor_siblings: bool,
    #[arg(short='v', long)]
    show_partner_siblings: bool,
    #[arg(short='d', long)]
    descendent_gen_limit: Option<usize>,
    #[arg(short='a', long)]
    ancestor_gen_limit: Option<usize>,
}

impl From<ViewOptions> for baumstamm_lib::view::ViewOptions {
    fn from(options: ViewOptions) -> Self {
        baumstamm_lib::view::ViewOptions {
            show_partners: options.show_partners,
            show_ancestor_siblings: options.show_ancestor_siblings,
            show_partner_siblings: options.show_partner_siblings,
            descendent_gen_limit: options
                .descendent_gen_limit
                .map(ViewLimit::Limit)
                .unwrap_or(ViewLimit::Unlimited),
            ancestor_gen_limit: options
                .ancestor_gen_limit
                .map(ViewLimit::Limit)
                .unwrap_or(ViewLimit::Unlimited),
        }
    }
}

fn save<P: AsRef<Path>>(path: P, tree: &FamilyTree) -> Result<(), Box<dyn Error>> {
    fs::write(path, tree.save()?)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Cli::parse();
    let mut tree = if args.new {
        let tree = FamilyTree::new();
        save(&args.file, &tree)?;
        tree
    } else {
        let json_str = fs::read_to_string(&args.file)?;
        FamilyTree::try_from(&json_str)?
    };

    if let Some(action) = args.action {
        match action {
            Action::Add(add) => match add {
                Add::Child(child) => {
                    let child_id =
                        tree.add_child(RelationshipId::try_from(child.rel_id.as_str())?)?;
                    save(&args.file, &tree)?;
                    println!("Added child as \"{}\"", child_id);
                }
                Add::Parent(parent) => {
                    let result =
                        tree.add_parent(RelationshipId::try_from(parent.rel_id.as_str())?)?;
                    save(&args.file, &tree)?;
                    println!(
                        "Added parent as \"{}\" and child of relationship \"{}\"",
                        result.0, result.1
                    );
                }
                Add::NewRelationship(rel) => {
                    let rel_id =
                        tree.add_new_relationship(PersonId::try_from(rel.person_id.as_str())?)?;
                    save(&args.file, &tree)?;
                    println!("Added new relationship \"{}\"", rel_id);
                }
                Add::RelationshipWithPartner(rel) => {
                    let rel_id = tree.add_relationship_with_partner(
                        PersonId::try_from(rel.person_id.as_str())?,
                        PersonId::try_from(rel.partner_id.as_str())?,
                    )?;
                    save(&args.file, &tree)?;
                    println!("Added relationship \"{}\"", rel_id);
                }
            },
            Action::Info(info) => match info {
                Info::Insert(insert) => {
                    tree.insert_info(
                        PersonId::try_from(insert.person_id.as_str())?,
                        insert.key.clone(),
                        insert.value.clone(),
                    )?;
                    save(&args.file, &tree)?;
                    println!(
                        "Inserted \"{}\": \"{}\" to \"{}\"",
                        insert.key, insert.value, insert.person_id
                    );
                }
                Info::Remove(remove) => {
                    let value = tree
                        .remove_info(PersonId::try_from(remove.person_id.as_str())?, &remove.key)?;
                    save(&args.file, &tree)?;
                    println!(
                        "Removed \"{}\": \"{}\" from \"{}\"",
                        remove.key, value, remove.person_id
                    );
                }
            },
            Action::Show(show) => match show {
                Show::Persons => println!("Persons: {:#?}", tree.get_persons()),
                Show::Relationships => println!("Relationships: {:#?}", tree.get_relationships()),
                Show::Layers => {
                    let graph = Graph::new(tree.get_relationships()).cut();
                    println!("Layers: {:#?}", graph.layers())
                }
                Show::PersonLayers => {
                    let graph = Graph::new(tree.get_relationships()).cut();
                    println!(
                        "Person Layers: {:#?}",
                        graph.person_layers(tree.get_relationships())
                    );
                }
            },
            Action::View(options) => {
                let view = View::new(
                    &tree,
                    PersonId::try_from(options.root.clone().as_str())?,
                    &options.into(),
                )?;
                let view_tree: FamilyTree = view.into();
                let tree_string = view_tree.save()?;
                println!("{}", tree_string);
            }
        };
    };
    Ok(())
}
