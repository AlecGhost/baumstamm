use crate::tree::PersonInfo;

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

    pub fn in_range(&self, point: GridPoint) -> bool {
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
