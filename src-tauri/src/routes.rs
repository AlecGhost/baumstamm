use baumstamm::{
    grid::{Grid, GridSize, Item, SourcePoint},
    tree::FamilyTree,
};

#[tauri::command]
pub fn generate_grid(size: GridSize, source: SourcePoint) -> Result<Vec<Item>, &'static str> {
    println!("Size: {:?}", size);
    println!("Source: {:?}", source);
    let grid = Grid::new(size, source).expect("Grid failed");
    println!("Grid");
    let tree =
        FamilyTree::new("rels.json".to_string(), "persons.json".to_string()).expect("Tree failed");
    println!("Tree");
    Ok(grid.generate(&tree))
}
