use crate::{
    error::{CommandError, Error},
    grid::{self, GridItem},
};
use baumstamm_lib::{FamilyTree, Person, Relationship};
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
        FamilyTree::try_from(&data)?
    };
    let mut lock = state.0.lock().unwrap();
    lock.path = Some(path);
    lock.tree = tree;
    Ok(())
}

pub(crate) fn save_file(path: PathBuf, state: State) -> Result<(), Error> {
    let mut lock = state.0.lock().unwrap();
    let data = lock.tree.save()?;
    std::fs::write(&path, data)?;
    lock.path = Some(path);
    Ok(())
}

#[tauri::command]
#[specta]
pub(crate) fn get_workspace_path(state: State) -> Result<PathBuf, Error> {
    let path = state
        .0
        .lock()
        .unwrap()
        .path
        .as_ref()
        .and_then(|path| path.parent())
        .ok_or(CommandError::NoWorkspace)?
        .canonicalize()?;
    Ok(path)
}

#[tauri::command]
#[specta]
pub(crate) fn get_canonical_path(path: PathBuf) -> Result<PathBuf, Error> {
    let path = path.canonicalize()?;
    Ok(path)
}

#[tauri::command]
#[specta]
pub(crate) fn get_path_relative_to_workspace(
    path: PathBuf,
    state: State,
) -> Result<PathBuf, Error> {
    let path = path.canonicalize()?;
    let file_name = path.file_name().ok_or(CommandError::NoWorkspace)?;
    let parent = path.parent().ok_or(CommandError::NoWorkspace)?;
    let workspace = get_workspace_path(state)?;
    let mut path_iter = parent.iter();
    let mut workspace_iter = workspace.iter();
    let mut relative_path = PathBuf::new();
    loop {
        match (path_iter.next(), workspace_iter.next()) {
            (Some(path_dir), Some(workspace_dir)) => {
                if path_dir != workspace_dir {
                    relative_path = PathBuf::from("..").join(relative_path);
                    relative_path.push(path_dir);
                }
            }
            (Some(path_dir), None) => relative_path.push(path_dir),
            (None, Some(_)) => relative_path = PathBuf::from("..").join(relative_path),
            (None, None) => break,
        }
    }
    relative_path.push(file_name);
    Ok(relative_path)
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
pub(crate) fn get_grid(state: State) -> Result<Vec<Vec<GridItem>>, ()> {
    let tree = &state.0.lock().unwrap().tree;
    let grid = grid::generate(tree);
    Ok(grid)
}

// adding nodes
#[tauri::command]
#[specta]
pub(crate) fn add_parent(rid: Rid, state: State) -> Result<(Pid, Rid), Error> {
    let mut lock = state.0.lock().unwrap();
    let result = lock.tree.add_parent(rid)?;
    if let Some(path) = lock.path.clone() {
        drop(lock);
        save_file(path, state)?;
    }
    Ok(result)
}

#[tauri::command]
#[specta]
pub(crate) fn add_child(rid: Rid, state: State) -> Result<Pid, Error> {
    let mut lock = state.0.lock().unwrap();
    let result = lock.tree.add_child(rid)?;
    if let Some(path) = lock.path.clone() {
        drop(lock);
        save_file(path, state)?;
    }
    Ok(result)
}

#[tauri::command]
#[specta]
pub(crate) fn add_new_relationship(pid: Pid, state: State) -> Result<Rid, Error> {
    let mut lock = state.0.lock().unwrap();
    let result = lock.tree.add_new_relationship(pid)?;
    if let Some(path) = lock.path.clone() {
        drop(lock);
        save_file(path, state)?;
    }
    Ok(result)
}

#[tauri::command]
#[specta]
pub(crate) fn add_relationship_with_partner(
    pid: Pid,
    partner_pid: Pid,
    state: State,
) -> Result<Rid, Error> {
    let mut lock = state.0.lock().unwrap();
    let result = lock.tree.add_relationship_with_partner(pid, partner_pid)?;
    if let Some(path) = lock.path.clone() {
        drop(lock);
        save_file(path, state)?;
    }
    Ok(result)
}

#[tauri::command]
#[specta]
pub(crate) fn remove_person(pid: Pid, state: State) -> Result<(), Error> {
    let mut lock = state.0.lock().unwrap();
    lock.tree.remove_person(pid)?;
    if let Some(path) = lock.path.clone() {
        drop(lock);
        save_file(path, state)?;
    }
    Ok(())
}

#[tauri::command]
#[specta]
pub(crate) fn merge_person(pid1: Pid, pid2: Pid, state: State) -> Result<(), Error> {
    let mut lock = state.0.lock().unwrap();
    lock.tree.merge_person(pid1, pid2)?;
    if let Some(path) = lock.path.clone() {
        drop(lock);
        save_file(path, state)?;
    }
    Ok(())
}

// info
#[tauri::command]
#[specta]
pub(crate) fn insert_info(pid: Pid, key: String, value: String, state: State) -> Result<(), Error> {
    let mut lock = state.0.lock().unwrap();
    lock.tree.insert_info(pid, key, value)?;
    if let Some(path) = lock.path.clone() {
        drop(lock);
        save_file(path, state)?;
    }
    Ok(())
}

#[tauri::command]
#[specta]
pub(crate) fn remove_info(pid: Pid, key: &str, state: State) -> Result<String, Error> {
    let mut lock = state.0.lock().unwrap();
    let result = lock.tree.remove_info(pid, key)?;
    if let Some(path) = lock.path.clone() {
        drop(lock);
        save_file(path, state)?;
    }
    Ok(result)
}
