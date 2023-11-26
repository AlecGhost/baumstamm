use baumstamm_grid;
use baumstamm_lib::FamilyTree;
use serde_wasm_bindgen as bind;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
#[derive(Debug, Default)]
pub struct State {
    tree: FamilyTree,
}

type JResult = std::result::Result<JsValue, JsValue>;
type Pid = baumstamm_lib::PersonId;
type Rid = baumstamm_lib::RelationshipId;

#[wasm_bindgen]
pub fn init_state() -> State {
    State::default()
}

// get datastructures
#[wasm_bindgen]
pub fn get_persons(state: &State) -> JResult {
    let persons = state.tree.get_persons().to_vec();
    Ok(bind::to_value(&persons)?)
}

#[wasm_bindgen]
pub fn get_relationships(state: &State) -> JResult {
    let persons = state.tree.get_relationships().to_vec();
    Ok(bind::to_value(&persons)?)
}

#[wasm_bindgen]
pub fn get_grid(state: &State) -> JResult {
    let tree = &state.tree;
    let grid = baumstamm_grid::generate(tree);
    Ok(bind::to_value(&grid)?)
}

// adding nodes
#[wasm_bindgen]
pub fn add_parent(rid: &str, state: &mut State) -> JResult {
    let result = state
        .tree
        .add_parent(parse_rid(rid)?)
        .map_err(|err| err.to_string())?;
    Ok(bind::to_value(&result)?)
}

#[wasm_bindgen]
pub fn add_child(rid: &str, state: &mut State) -> JResult {
    let result = state
        .tree
        .add_child(parse_rid(rid)?)
        .map_err(|err| err.to_string())?;
    Ok(bind::to_value(&result)?)
}

#[wasm_bindgen]
pub fn add_new_relationship(pid: &str, state: &mut State) -> JResult {
    let result = state
        .tree
        .add_new_relationship(parse_pid(pid)?)
        .map_err(|err| err.to_string())?;
    Ok(bind::to_value(&result)?)
}

#[wasm_bindgen]
pub fn add_relationship_with_partner(pid: &str, partner_pid: &str, state: &mut State) -> JResult {
    let result = state
        .tree
        .add_relationship_with_partner(parse_pid(pid)?, parse_pid(partner_pid)?)
        .map_err(|err| err.to_string())?;
    Ok(bind::to_value(&result)?)
}

#[wasm_bindgen]
pub fn remove_person(pid: &str, state: &mut State) -> JResult {
    state
        .tree
        .remove_person(parse_pid(pid)?)
        .map_err(|err| err.to_string())?;
    Ok(JsValue::NULL)
}

#[wasm_bindgen]
pub fn merge_person(pid1: &str, pid2: &str, state: &mut State) -> JResult {
    state
        .tree
        .merge_person(parse_pid(pid1)?, parse_pid(pid2)?)
        .map_err(|err| err.to_string())?;
    Ok(JsValue::NULL)
}

// info
#[wasm_bindgen]
pub fn insert_info(pid: &str, key: &str, value: &str, state: &mut State) -> JResult {
    state
        .tree
        .insert_info(parse_pid(pid)?, key.to_string(), value.to_string())
        .map_err(|err| err.to_string())?;
    Ok(JsValue::NULL)
}

#[wasm_bindgen]
pub fn remove_info(pid: &str, key: &str, state: &mut State) -> JResult {
    let result = state
        .tree
        .remove_info(parse_pid(pid)?, key)
        .map_err(|err| err.to_string())?;
    Ok(bind::to_value(&result)?)
}

fn parse_rid(rid: &str) -> Result<Rid, JsValue> {
    rid.try_into()
        .map_err(|err: std::num::ParseIntError| JsValue::from_str(&err.to_string()))
}

fn parse_pid(pid: &str) -> Result<Pid, JsValue> {
    pid.try_into()
        .map_err(|err: std::num::ParseIntError| JsValue::from_str(&err.to_string()))
}
