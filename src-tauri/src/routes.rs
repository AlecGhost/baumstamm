use crate::grid::{Connector, GridSize, Item, Person, SourcePoint};

#[tauri::command]
pub fn generate_grid(size: GridSize, source: SourcePoint) -> Result<Vec<Item>, String> {
    println!("Size: {:?}", size);
    println!("Source: {:?}", source);
    if !size.in_range(source.point) {
        return Err(String::from("Source point is out of bounds"));
    }
    let connector = Item::Connector(Connector::T);
    let person = Item::Person(Person::new(
        0,
        String::from("John"),
        Some(String::from("Doe")),
        Some(String::from("01.11.1111")),
        None,
    ));
    let empty = Item::None;
    Ok(vec![connector, person, empty])
}
