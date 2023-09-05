import { writable, get } from 'svelte/store';
import { Person } from './Person';
import {
	getGrid,
	getPersons,
	getRelationships,
	type GridItem,
	type PersonId,
	type Relationship
} from '../bindings';

export const persons = writable<Person[]>([]);
export const relationships = writable<Relationship[]>([]);
export const selected = writable<Person | null>(null);
export const target = writable<Person | null>(null);
export const grid = writable<GridItem[][]>([]);
export const settings = writable({
	showAvatar: true,
	showNames: true
});

export async function updateSelected(pid: PersonId) {
	const person = get(persons).find((person) => person.id == pid);
	if (person !== undefined) {
		selected.update(() => person);
	}
}

export async function updateTarget(pid: PersonId | null) {
	if (pid == null) {
		target.update(() => null);
	} else {
		const person = get(persons).find((person) => person.id == pid);
		if (person !== undefined) {
			target.update(() => person);
		}
	}
}

export async function update() {
	// update persons
	const newPersons = await getPersons();
	const mappedPersons = newPersons.map((person) => Person.from(person));
	persons.update(() => mappedPersons);

	// update selectedStore when necessary and possible
	const currentSelected = get(selected);
	if (mappedPersons.length > 0 && currentSelected !== null) {
		let person = mappedPersons.find((person) => person.id == currentSelected.id);
		if (person !== undefined) {
			selected.update(() => person!);
		} else {
			selected.update(() => mappedPersons[0]);
		}
	}

	// update relationships
	const newRelationships = await getRelationships();
	relationships.update(() => newRelationships);

	// update grid
	const newGrid = await getGrid();
	grid.update(() => newGrid);
}
