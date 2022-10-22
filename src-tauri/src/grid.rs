use crate::tree::{FamilyTree, PersonInfo};
use std::error::Error;

#[derive(Debug, serde::Serialize)]
pub enum Connector {
    Straight,
    T,
    LeftCorner,
    RightCorner,
}

#[derive(Debug, serde::Serialize)]
pub enum Item {
    None,
    Person(PersonInfo),
    Connector(Connector),
}

#[derive(Debug, serde::Deserialize)]
pub struct GridSize {
    rows: u8,
    columns: u8,
}

impl GridSize {
    pub fn new(rows: u8, columns: u8) -> GridSize {
        GridSize { rows, columns }
    }

    pub fn in_range(&self, point: &GridPoint) -> bool {
        self.columns > point.x && self.rows > point.y
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct SourcePoint {
    pub id: u32,
    pub point: GridPoint,
}

#[derive(Debug, serde::Deserialize)]
pub struct GridPoint {
    pub x: u8,
    pub y: u8,
}

pub struct Grid {
    size: GridSize,
    source: SourcePoint,
}

impl Grid {
    pub fn new(size: GridSize, source: SourcePoint) -> Result<Self, Box<dyn Error>> {
        if !size.in_range(&source.point) {
            return Err("Source point is out of bounds".into());
        }
        Ok(Grid { size, source })
    }

    pub fn generate(&self, _tree: &FamilyTree) -> Vec<Item> {
        let connector = Item::Connector(Connector::T);
        let person = Item::Person(PersonInfo::new(
            String::from("John"),
            Some(String::from("Doe")),
            Some(String::from("01.11.1111")),
            None,
        ));
        let empty = Item::None;
        vec![connector, person, empty]
    }
}
