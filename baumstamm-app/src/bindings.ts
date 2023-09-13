/* eslint-disable */
// This file was generated by [tauri-specta](https://github.com/oscartbeaumont/tauri-specta). Do not edit this file manually.

declare global {
    interface Window {
        __TAURI_INVOKE__<T>(cmd: string, args?: Record<string, unknown>): Promise<T>;
    }
}

// Function avoids 'window not defined' in SSR
const invoke = () => window.__TAURI_INVOKE__;

export function getPersons() {
    return invoke()<Person[]>("get_persons")
}

export function getRelationships() {
    return invoke()<Relationship[]>("get_relationships")
}

export function getGrid() {
    return invoke()<GridItem[][]>("get_grid")
}

export function addParent(rid: RelationshipId) {
    return invoke()<[PersonId, RelationshipId]>("add_parent", { rid })
}

export function addChild(rid: RelationshipId) {
    return invoke()<PersonId>("add_child", { rid })
}

export function addNewRelationship(pid: PersonId) {
    return invoke()<RelationshipId>("add_new_relationship", { pid })
}

export function addRelationshipWithPartner(pid: PersonId, partnerPid: PersonId) {
    return invoke()<RelationshipId>("add_relationship_with_partner", { pid,partnerPid })
}

export function removePerson(pid: PersonId) {
    return invoke()<null>("remove_person", { pid })
}

export function mergePerson(pid1: PersonId, pid2: PersonId) {
    return invoke()<null>("merge_person", { pid1,pid2 })
}

export function insertInfo(pid: PersonId, key: string, value: string) {
    return invoke()<null>("insert_info", { pid,key,value })
}

export function removeInfo(pid: PersonId, key: string) {
    return invoke()<string>("remove_info", { pid,key })
}

/**
 * UUID for a `Person`, stored as u128.
 */
export type PersonId = string
export type Ending = { connection: number; color: [number, number, number]; origin: Origin; x_index: number; y_index: number }
/**
 * UUID for a `Relationship`, stored as u128.
 */
export type RelationshipId = string
export type Crossing = { connection: number; color: [number, number, number]; origin: Origin; x_index: number; y_index: number }
export type Passing = { connection: number; color: [number, number, number]; y_index: number }
export type GridItem = { Person: PersonId } | { Connections: Connections }
/**
 * A person with a unique identifier and arbitrary attached information
 */
export type Person = { id: PersonId; info: { [key: string]: string } | null }
export type Origin = "Left" | "Right" | "None"
/**
 * A relationship referencing two optional parents and the resulting children.
 */
export type Relationship = { id: RelationshipId; parents: (PersonId | null)[]; children: PersonId[] }
export type Connections = { orientation: Orientation; total_x: number; total_y: number; passing: Passing[]; ending: Ending[]; crossing: Crossing[] }
export type Orientation = "Up" | "Down"
