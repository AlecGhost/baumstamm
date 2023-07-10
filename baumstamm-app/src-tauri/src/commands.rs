use baumstamm_lib::{
    error::{DisplayError, Error},
    graph::{DisplayGraph, DisplayOptions, Graph},
    Person,
};
use specta::specta;

type State<'a> = tauri::State<'a, crate::State>;
type Rid = baumstamm_lib::RelationshipId;
type Pid = baumstamm_lib::PersonId;

#[tauri::command]
#[specta]
pub(crate) fn get_persons(state: State) -> Result<Vec<Person>, ()> {
    let persons = state.0.lock().unwrap().tree.get_persons().to_vec();
    Ok(persons)
}

#[tauri::command]
#[specta]
pub(crate) fn add_parent(rid: Rid, state: State) -> Result<(Pid, Rid), Error> {
    state.0.lock().unwrap().tree.add_parent(rid)
}

#[tauri::command]
#[specta]
pub(crate) fn add_child(rid: Rid, state: State) -> Result<Pid, Error> {
    state.0.lock().unwrap().tree.add_child(rid)
}

#[tauri::command]
#[specta]
pub(crate) fn add_new_relationship(pid: Pid, state: State) -> Result<Rid, Error> {
    state.0.lock().unwrap().tree.add_new_relationship(pid)
}

#[tauri::command]
#[specta]
pub(crate) fn add_relationship_with_partner(
    pid: Pid,
    partner_pid: Pid,
    state: State,
) -> Result<Rid, Error> {
    state
        .0
        .lock()
        .unwrap()
        .tree
        .add_relationship_with_partner(pid, partner_pid)
}

#[tauri::command]
#[specta]
pub(crate) fn insert_info(pid: Pid, key: String, value: String, state: State) -> Result<(), Error> {
    state.0.lock().unwrap().tree.insert_info(pid, key, value)
}

#[tauri::command]
#[specta]
pub(crate) fn remove_info(pid: Pid, key: &str, state: State) -> Result<String, Error> {
    state.0.lock().unwrap().tree.remove_info(pid, key)
}

#[tauri::command]
#[specta]
pub(crate) fn display_graph(
    options: DisplayOptions,
    state: State,
) -> Result<DisplayGraph, DisplayError> {
    let guard = state.0.lock().unwrap();
    let relationships = guard.tree.get_relationships();
    Graph::new(relationships).cut().display(options)
}
