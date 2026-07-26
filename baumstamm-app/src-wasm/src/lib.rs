use std::sync::Mutex;

use baumstamm_lib::{
    view::{View, ViewOptions},
    FamilyTree, Person, Relationship,
};
use once_cell::sync::Lazy;
use serde_wasm_bindgen as bind;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
#[derive(Debug, Default)]
struct State {
    tree: FamilyTree,
    view: Option<FamilyTree>,
    view_selection: ViewSelection,
}

static STATE: Lazy<Mutex<State>> = Lazy::new(|| Mutex::new(State::default()));

impl State {
    fn get_view(&self) -> &FamilyTree {
        match self.view.as_ref() {
            Some(view) => view,
            None => &self.tree,
        }
    }

    fn update_view(&mut self) {
        match &self.view_selection {
            ViewSelection::Full => self.view = None,
            ViewSelection::Partial { root, options } => {
                self.view = View::new(&self.tree, *root, options)
                    .ok()
                    .map(FamilyTree::from);
            }
        }
    }
}

#[derive(Clone, Debug, Default)]
enum ViewSelection {
    #[default]
    Full,
    Partial {
        root: Pid,
        options: ViewOptions,
    },
}

#[derive(serde::Serialize)]
struct TreeData<'a> {
    persons: &'a [Person],
    relationships: &'a [Relationship],
    grid: Vec<Vec<baumstamm_grid::GridItem>>,
}

type JResult = std::result::Result<JsValue, JsValue>;
type Pid = baumstamm_lib::PersonId;
type Rid = baumstamm_lib::RelationshipId;

#[wasm_bindgen]
pub fn load_tree(input: &str) -> JResult {
    let tree = FamilyTree::try_from(input).map_err(|err| err.to_string())?;
    *STATE.lock().unwrap() = State {
        tree,
        ..State::default()
    };
    Ok(JsValue::NULL)
}

#[wasm_bindgen]
pub fn new_tree() -> JResult {
    *STATE.lock().unwrap() = State::default();
    Ok(JsValue::NULL)
}

#[wasm_bindgen]
pub fn save_tree() -> JResult {
    let string = STATE
        .lock()
        .unwrap()
        .tree
        .save()
        .map_err(|err| err.to_string())?;
    Ok(JsValue::from(string))
}

// get datastructures
#[wasm_bindgen]
pub fn get_persons() -> JResult {
    let persons = STATE.lock().unwrap().get_view().get_persons().to_vec();
    Ok(bind::to_value(&persons)?)
}

#[wasm_bindgen]
pub fn get_relationships() -> JResult {
    let persons = STATE
        .lock()
        .unwrap()
        .get_view()
        .get_relationships()
        .to_vec();
    Ok(bind::to_value(&persons)?)
}

#[wasm_bindgen]
pub fn get_grid() -> JResult {
    let state = STATE.lock().unwrap();
    let tree = state.get_view();
    let grid = baumstamm_grid::generate(tree);
    Ok(bind::to_value(&grid)?)
}

#[wasm_bindgen]
pub fn get_tree_data() -> JResult {
    let state = STATE.lock().unwrap();
    let tree = state.get_view();
    let persons = tree.get_persons();
    let relationships = tree.get_relationships();
    let grid = baumstamm_grid::generate(tree);

    let data = TreeData {
        persons,
        relationships,
        grid,
    };

    Ok(bind::to_value(&data)?)
}

#[wasm_bindgen]
pub fn get_sub_tree_data(root: &str, options: JsValue) -> JResult {
    let root = parse_pid(root)?;
    let options: ViewOptions = bind::from_value(options).map_err(|err| err.to_string())?;

    let state = STATE.lock().unwrap();
    let view = View::new(&state.tree, root, &options).map_err(|err| err.to_string())?;

    let sub_tree = FamilyTree::from(view);
    let persons = sub_tree.get_persons();
    let relationships = sub_tree.get_relationships();
    let grid = baumstamm_grid::generate(&sub_tree);

    let data = TreeData {
        persons,
        relationships,
        grid,
    };

    Ok(bind::to_value(&data)?)
}

#[wasm_bindgen]
pub fn set_partial_view(root: &str, options: JsValue) -> JResult {
    let mut state = STATE.lock().unwrap();
    let root = parse_pid(root)?;
    let options: ViewOptions = bind::from_value(options).map_err(|err| err.to_string())?;

    state.view_selection = ViewSelection::Partial { root, options };
    state.update_view();

    Ok(JsValue::NULL)
}

#[wasm_bindgen]
pub fn set_full_view() -> JResult {
    let mut state = STATE.lock().unwrap();

    state.view_selection = ViewSelection::Full;
    state.update_view();

    Ok(JsValue::NULL)
}

// adding nodes
#[wasm_bindgen]
pub fn add_parent(rid: &str) -> JResult {
    let result = STATE
        .lock()
        .unwrap()
        .tree
        .add_parent(parse_rid(rid)?)
        .map_err(|err| err.to_string())?;
    STATE.lock().unwrap().update_view();
    Ok(bind::to_value(&result)?)
}

#[wasm_bindgen]
pub fn add_child(rid: &str) -> JResult {
    let result = STATE
        .lock()
        .unwrap()
        .tree
        .add_child(parse_rid(rid)?)
        .map_err(|err| err.to_string())?;
    STATE.lock().unwrap().update_view();
    Ok(bind::to_value(&result)?)
}

#[wasm_bindgen]
pub fn add_new_relationship(pid: &str) -> JResult {
    let result = STATE
        .lock()
        .unwrap()
        .tree
        .add_new_relationship(parse_pid(pid)?)
        .map_err(|err| err.to_string())?;
    STATE.lock().unwrap().update_view();
    Ok(bind::to_value(&result)?)
}

#[wasm_bindgen]
pub fn add_relationship_with_partner(pid: &str, partner_pid: &str) -> JResult {
    let result = STATE
        .lock()
        .unwrap()
        .tree
        .add_relationship_with_partner(parse_pid(pid)?, parse_pid(partner_pid)?)
        .map_err(|err| err.to_string())?;
    STATE.lock().unwrap().update_view();
    Ok(bind::to_value(&result)?)
}

#[wasm_bindgen]
pub fn remove_person(pid: &str) -> JResult {
    STATE
        .lock()
        .unwrap()
        .tree
        .remove_person(parse_pid(pid)?)
        .map_err(|err| err.to_string())?;
    STATE.lock().unwrap().update_view();
    Ok(JsValue::NULL)
}

#[wasm_bindgen]
pub fn merge_person(pid1: &str, pid2: &str) -> JResult {
    STATE
        .lock()
        .unwrap()
        .tree
        .merge_person(parse_pid(pid1)?, parse_pid(pid2)?)
        .map_err(|err| err.to_string())?;
    STATE.lock().unwrap().update_view();
    Ok(JsValue::NULL)
}

// info
#[wasm_bindgen]
pub fn insert_info(pid: &str, key: &str, value: &str) -> JResult {
    STATE
        .lock()
        .unwrap()
        .tree
        .insert_info(parse_pid(pid)?, key.to_string(), value.to_string())
        .map_err(|err| err.to_string())?;
    Ok(JsValue::NULL)
}

#[wasm_bindgen]
pub fn remove_info(pid: &str, key: &str) -> JResult {
    let result = STATE
        .lock()
        .unwrap()
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
