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

export async function getPersons() {
	if ('__TAURI__' in window) {
		return tauriGetPersons();
	} else {
		return new Promise<Person[]>((resolve) => resolve([]));
	}
}

export async function getRelationships() {
	if ('__TAURI__' in window) {
		return tauriGetRelationships();
	} else {
		return new Promise<Relationship[]>((resolve) => resolve([]));
	}
}

export async function getGrid() {
	if ('__TAURI__' in window) {
		return tauriGetGrid();
	} else {
		return new Promise<GridItem[][]>((resolve) => resolve([]));
	}
}

export async function addParent(rid: RelationshipId) {
	if ('__TAURI__' in window) {
		return tauriAddParent(rid);
	} else {
		return new Promise<[string, string]>((resolve) => resolve(['', '']));
	}
}

export async function addChild(rid: RelationshipId) {
	if ('__TAURI__' in window) {
		return tauriAddChild(rid);
	} else {
		return new Promise<string>((resolve) => resolve(''));
	}
}

export async function addNewRelationship(pid: PersonId) {
	if ('__TAURI__' in window) {
		return tauriAddNewRelationship(pid);
	} else {
		return new Promise<string>((resolve) => resolve(''));
	}
}

export async function addRelationshipWithPartner(pid: PersonId, partnerPid: PersonId) {
	if ('__TAURI__' in window) {
		return tauriAddRelationshipWithPartner(pid, partnerPid);
	} else {
		return new Promise<string>((resolve) => resolve(''));
	}
}

export async function removePerson(pid: PersonId) {
	if ('__TAURI__' in window) {
		return tauriRemovePerson(pid);
	} else {
		return new Promise<null>((resolve) => resolve(null));
	}
}

export async function mergePerson(pid1: PersonId, pid2: PersonId) {
	if ('__TAURI__' in window) {
		return tauriMergePerson(pid1, pid2);
	} else {
		return new Promise<null>((resolve) => resolve(null));
	}
}

export async function insertInfo(pid: PersonId, key: string, value: string) {
	if ('__TAURI__' in window) {
		return tauriInsertInfo(pid, key, value);
	} else {
		return new Promise<null>((resolve) => resolve(null));
	}
}

export async function removeInfo(pid: PersonId, key: string) {
	if ('__TAURI__' in window) {
		return tauriRemoveInfo(pid, key);
	} else {
		return new Promise<string>((resolve) => resolve(''));
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
