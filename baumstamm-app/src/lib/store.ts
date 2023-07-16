import { writable, get } from 'svelte/store';
import { Person } from './Person';
import { getPersonLayers, getPersons, getRelationships, type Relationship } from '../bindings';
import { GridItem } from './GridItem';
import { Connections } from './Connections';

export const persons = writable<Person[]>([]);
export const relationships = writable<Relationship[]>([]);
export const selected = writable<Person | null>(null);
export const grid = writable<GridItem[][]>([]);

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
		!mappedPersons.map((person) => person).includes(currentSelected)
	) {
		selected.update(() => mappedPersons[0]);
	}

	// update relationships
	const newRelationships = await getRelationships();
	relationships.update(() => newRelationships);

	// update grid
	const layers = await getPersonLayers();
	const longestRow = layers.map((layer) => layer.length).reduce((a, b) => Math.max(a, b), 0);
	let items: GridItem[][] = [];
	let index = 0;
	layers.forEach((layer) => {
		let personRow = layer
			.map((pid) => mappedPersons.find((person) => pid == person.id))
			.map((person) => {
				if (person !== undefined) {
					return GridItem.fromPerson(person);
				} else {
					throw new ReferenceError();
				}
			});
		while (personRow.length < longestRow) {
			personRow.push(GridItem.fromConnections(Connections.empty()));
		}

		items[index] = siblingRow(personRow, newRelationships);
		items[index + 1] = personRow;
		items[index + 2] = relationshipRow(personRow, newRelationships);

		index += 3;
	});

	grid.update(() => items);
}

function siblingRow(personRow: GridItem[], relationships: Relationship[]): GridItem[] {
	let items = [];
	for (let _ in personRow) {
		items.push(GridItem.fromConnections(Connections.empty()));
	}
	return items;
}

function relationshipRow(personRow: GridItem[], relationships: Relationship[]): GridItem[] {
	let items = [];
	for (let _ in personRow) {
		items.push(GridItem.fromConnections(Connections.empty()));
	}
	return items;
}
