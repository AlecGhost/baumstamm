import { writable, get } from 'svelte/store';
import { Person } from './Person';
import {
	getPersonLayers,
	getPersons,
	getRelationships,
	type PersonId,
	type Relationship
} from '../bindings';
import { GridItem } from './GridItem';
import { Connections, type ConnectionParams } from './Connections';

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
		!mappedPersons.map((person) => person.id).includes(currentSelected.id)
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
					throw new Error('Could not find person');
				}
			});
		while (personRow.length < longestRow) {
			personRow.push(GridItem.fromConnections(Connections.empty()));
		}

		items[index] = siblingRow(
			personRow,
			newRelationships
				.map((rel) => rel.children)
				.filter((children) => children.length > 1)
				.filter((children) => children.every((child) => layer.includes(child)))
		);
		items[index + 1] = personRow;
		items[index + 2] = relationshipRow(
			personRow,
			newRelationships
				.map((rel) => rel.parents)
				// filter parents with two valid pids
				.map((parents) => parents.filter((parent): parent is string => Boolean(parent)))
				.filter((parents) => parents.length == 2)
				.filter((parents) => parents.every((parent) => layer.includes(parent)))
		);

		index += 3;
	});

	grid.update(() => items);
}

function siblingRow(personRow: GridItem[], childrenArrays: PersonId[][]): GridItem[] {
	const orientation = 'down';
	const ranges = childrenArrays.map((children) =>
		children.map((child) => getIndex(personRow, child)).sort()
	);
	const total = ranges.length;
	let items = [];
	for (let i = 0; i < personRow.length; i++) {
		let params: ConnectionParams = {
			total,
			orientation,
			passing: [],
			ending: [],
			crossing: []
		};
		ranges.forEach((range, connection) => {
			if (i == range[0]) {
				params.ending.push({
					connection,
					origin: 'right'
				});
			} else if (i == range[range.length - 1]) {
				params.ending.push({
					connection,
					origin: 'left'
				});
			} else if (range.includes(i)) {
				params.ending.push({
					connection,
					origin: 'both'
				});
			} else if (range[0] < i && i < range[range.length - 1]) {
				params.passing.push({
					connection
				});
			}
		});
		items.push(GridItem.fromConnections(new Connections(params)));
	}
	return items;
}

function relationshipRow(personRow: GridItem[], parentPairs: PersonId[][]): GridItem[] {
	const orientation = 'up';
	const ranges = parentPairs.map((parents) =>
		parents.map((parent) => getIndex(personRow, parent)).sort()
	);
	const total = ranges.length;
	let items = [];
	for (let i = 0; i < personRow.length; i++) {
		let params: ConnectionParams = {
			total,
			orientation,
			passing: [],
			ending: [],
			crossing: []
		};
		ranges.forEach((range, connection) => {
			if (i == range[0]) {
				params.ending.push({
					connection,
					origin: 'right'
				});
			} else if (i == range[1]) {
				params.ending.push({
					connection,
					origin: 'left'
				});
			} else if (range[0] < i && i < range[1]) {
				params.passing.push({
					connection
				});
			}
		});
		items.push(GridItem.fromConnections(new Connections(params)));
	}
	return items;
}

function getIndex(personRow: GridItem[], pid: PersonId): number {
	const index = personRow.findIndex((item) => item.isPerson() && item.getPerson().id === pid);
	if (index < 0) {
		throw new Error('Index not found');
	}
	return index;
}
