use crate::error::Error;
use baumstamm_lib::{
    graph::{person_layers, Graph},
    FamilyTree, Person, Relationship,
};
use specta::specta;
use std::path::PathBuf;

type State<'a> = tauri::State<'a, crate::State>;
type Rid = baumstamm_lib::RelationshipId;
type Pid = baumstamm_lib::PersonId;

// io
pub(crate) fn open_file(path: PathBuf, state: State) -> Result<(), Error> {
    let data = std::fs::read_to_string(&path)?;
    let tree = if data.is_empty() {
        let tree = FamilyTree::new();
        let data = tree.save()?;
        std::fs::write(&path, data)?;
        tree
    } else {
        FamilyTree::from_string(&data)?
    };
    let mut state = state.0.lock().unwrap();
    state.path = Some(path);
    state.tree = tree;
    Ok(())
}

pub(crate) fn save_file(path: PathBuf, state: State) -> Result<(), Error> {
    let mut state = state.0.lock().unwrap();
    let data = state.tree.save()?;
    std::fs::write(&path, data)?;
    state.path = Some(path);
    Ok(())
}

// get datastructures
#[tauri::command]
#[specta]
pub(crate) fn get_persons(state: State) -> Result<Vec<Person>, ()> {
    let persons = state.0.lock().unwrap().tree.get_persons().to_vec();
    Ok(persons)
}

#[tauri::command]
#[specta]
pub(crate) fn get_relationships(state: State) -> Result<Vec<Relationship>, ()> {
    let persons = state.0.lock().unwrap().tree.get_relationships().to_vec();
    Ok(persons)
}

#[tauri::command]
#[specta]
pub(crate) fn get_person_layers(state: State) -> Vec<Vec<Pid>> {
    let state = state.0.lock().unwrap();
    let relationships = state.tree.get_relationships();
    let graph = Graph::new(relationships).cut();
    let layers = graph.layers();
    person_layers(&layers, relationships)
}

// adding nodes
#[tauri::command]
#[specta]
pub(crate) fn add_parent(rid: Rid, state: State) -> Result<(Pid, Rid), Error> {
    let mut state = state.0.lock().unwrap();
    let result = state.tree.add_parent(rid)?;
    Ok(result)
}

#[tauri::command]
#[specta]
pub(crate) fn add_child(rid: Rid, state: State) -> Result<Pid, Error> {
    let mut state = state.0.lock().unwrap();
    let result = state.tree.add_child(rid)?;
    Ok(result)
}

#[tauri::command]
#[specta]
pub(crate) fn add_new_relationship(pid: Pid, state: State) -> Result<Rid, Error> {
    let mut state = state.0.lock().unwrap();
    let result = state.tree.add_new_relationship(pid)?;
    Ok(result)
}

#[tauri::command]
#[specta]
pub(crate) fn add_relationship_with_partner(
    pid: Pid,
    partner_pid: Pid,
    state: State,
) -> Result<Rid, Error> {
    let mut state = state.0.lock().unwrap();
    let result = state.tree.add_relationship_with_partner(pid, partner_pid)?;
    Ok(result)
}

// info
#[tauri::command]
#[specta]
pub(crate) fn insert_info(pid: Pid, key: String, value: String, state: State) -> Result<(), Error> {
    state.0.lock().unwrap().tree.insert_info(pid, key, value)?;
    Ok(())
}

#[tauri::command]
#[specta]
pub(crate) fn remove_info(pid: Pid, key: &str, state: State) -> Result<String, Error> {
    let result = state.0.lock().unwrap().tree.remove_info(pid, key)?;
    Ok(result)
}
