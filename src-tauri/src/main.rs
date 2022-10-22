use std::error::Error;

use baumstamm::tree::FamilyTree;
use clap::Parser;

#[derive(Parser)]
struct Args {
    relationships: String,
    persons: String,
    output: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let tree = FamilyTree::from_disk(args.relationships, args.persons)?;
    tree.export_puml(args.output.as_str())?;
    Ok(())
}
