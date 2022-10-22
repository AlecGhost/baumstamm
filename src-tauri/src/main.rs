use std::error::Error;

use baumstamm::tree::FamilyTree;
use clap::Parser;

#[derive(Parser)]
struct Args {
    relationships: String,
    persons: String,

    #[arg(short, long)]
    output: Option<String>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let tree = FamilyTree::from_disk(args.relationships, args.persons)?;

    if let Some(out_file) = &args.output {
        tree.export_puml(out_file)?;
    }

    Ok(())
}
