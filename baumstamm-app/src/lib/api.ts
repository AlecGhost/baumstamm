import {
	type PersonId,
	type RelationshipId,
	getPersons as tauriGetPersons,
	getRelationships as tauriGetRelationships,
	getGrid as tauriGetGrid,
	addParent as tauriAddParent,
	addChild as tauriAddChild,
	addNewRelationship as tauriAddNewRelationship,
	addRelationshipWithPartner as tauriAddRelationshipWithPartner,
	removePerson as tauriRemovePerson,
	mergePerson as tauriMergePerson,
	insertInfo as tauriInsertInfo,
	removeInfo as tauriRemoveInfo,
	type Person,
	type Relationship,
	type GridItem
} from '../bindings-tauri';
import {
	add_child as wasmAddChild,
	add_new_relationship as wasmAddNewRelationship,
	add_parent as wasmAddParent,
	add_relationship_with_partner as wasmAddRelationshipWithPartner,
	get_grid as wasmGetGrid,
	get_persons as wasmGetPersons,
	get_relationships as wasmGetRelationships,
	merge_person as wasmMergePersons,
	remove_info as wasmRemoveInfo,
	remove_person as wasmRemovePerson,
	insert_info as wasmInsertInfo
} from '$lib/baumstamm-wasm/baumstamm_wasm';
import { listen as tauriListen, type UnlistenFn } from '@tauri-apps/api/event';
import { toastStore, type ToastSettings } from '@skeletonlabs/skeleton';
import { update } from '$lib/store';

export async function listen() {
	let unlisten: UnlistenFn[] = [];
	if ('__TAURI__' in window) {
		unlisten.push(
			await tauriListen('open', () => {
				update();
			})
		);
		unlisten.push(
			await tauriListen('open-error', (e) => {
				const toast: ToastSettings = {
					message: e.payload as string
				};
				toastStore.trigger(toast);
			})
		);
		unlisten.push(
			await tauriListen('save-as-error', (e) => {
				const toast: ToastSettings = {
					message: e.payload as string
				};
				toastStore.trigger(toast);
			})
		);
	}
	return unlisten;
}

export async function getPersons(): Promise<Person[]> {
	if ('__TAURI__' in window) {
		return tauriGetPersons();
	} else {
		return wasmGetPersons(window.state);
	}
}

export async function getRelationships(): Promise<Relationship[]> {
	if ('__TAURI__' in window) {
		return tauriGetRelationships();
	} else {
		return wasmGetRelationships(window.state);
	}
}

export async function getGrid(): Promise<GridItem[][]> {
	if ('__TAURI__' in window) {
		return tauriGetGrid();
	} else {
		return wasmGetGrid(window.state);
	}
}

export async function addParent(rid: RelationshipId): Promise<[PersonId, RelationshipId]> {
	if ('__TAURI__' in window) {
		return tauriAddParent(rid);
	} else {
		return wasmAddParent(rid, window.state);
	}
}

export async function addChild(rid: RelationshipId): Promise<PersonId> {
	if ('__TAURI__' in window) {
		return tauriAddChild(rid);
	} else {
		return wasmAddChild(rid, window.state);
	}
}

export async function addNewRelationship(pid: PersonId): Promise<RelationshipId> {
	if ('__TAURI__' in window) {
		return tauriAddNewRelationship(pid);
	} else {
		return wasmAddNewRelationship(pid, window.state);
	}
}

export async function addRelationshipWithPartner(
	pid: PersonId,
	partnerPid: PersonId
): Promise<RelationshipId> {
	if ('__TAURI__' in window) {
		return tauriAddRelationshipWithPartner(pid, partnerPid);
	} else {
		return wasmAddRelationshipWithPartner(pid, partnerPid, window.state);
	}
}

export async function removePerson(pid: PersonId): Promise<null> {
	if ('__TAURI__' in window) {
		return tauriRemovePerson(pid);
	} else {
		return wasmRemovePerson(pid, window.state);
	}
}

export async function mergePerson(pid1: PersonId, pid2: PersonId): Promise<null> {
	if ('__TAURI__' in window) {
		return tauriMergePerson(pid1, pid2);
	} else {
		return wasmMergePersons(pid1, pid2, window.state);
	}
}

export async function insertInfo(pid: PersonId, key: string, value: string): Promise<null> {
	if ('__TAURI__' in window) {
		return tauriInsertInfo(pid, key, value);
	} else {
		return wasmInsertInfo(pid, key, value, window.state);
	}
}

export async function removeInfo(pid: PersonId, key: string): Promise<string> {
	if ('__TAURI__' in window) {
		return tauriRemoveInfo(pid, key);
	} else {
		return wasmRemoveInfo(pid, key, window.state);
	}
}

export type {
	Connections,
	Crossing,
	Ending,
	GridItem,
	Orientation,
	Origin,
	Passing,
	Person,
	PersonId,
	Relationship,
	RelationshipId
} from '../bindings-tauri';
