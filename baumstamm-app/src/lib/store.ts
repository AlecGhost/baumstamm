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
export const grid = writable<GridItem[][]>([]);

export async function updateSelected(pid: PersonId) {
	const person = get(persons).find((person) => person.id == pid);
	if (person !== undefined) {
		selected.update(() => person);
	}
}

export async function update() {
	// update persons
	const newPersons = await getPersons();
	const mappedPersons = newPersons.map((person) => Person.from(person));
	persons.update(() => mappedPersons);

	// update selectedStore when necessary and possible
	const currentSelected = get(selected);
	if (
		mappedPersons.length > 0 &&
		currentSelected !== null &&
		!mappedPersons.map((person) => person.id).includes(currentSelected.id)
	) {
		selected.update(() => mappedPersons[0]);
	}

	// update relationships
	const newRelationships = await getRelationships();
	relationships.update(() => newRelationships);

	// update grid
	const newGrid = await getGrid();
	grid.update(() => newGrid);
}
