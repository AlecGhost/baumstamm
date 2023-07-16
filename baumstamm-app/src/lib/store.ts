import { writable, get } from 'svelte/store';
import { Person } from './Person';
import { getPersonLayers, getPersons } from '../bindings';
import { GridItem } from './GridItem';

export const persons = writable<Person[]>([]);
export const selected = writable<Person | null>(null);
export const grid = writable<GridItem[][]>([]);

export function update() {
	getPersons().then(async (newPersons) => {
		const currentSelected = get(selected);
		const mappedPersons = newPersons.map((person) => Person.from(person));
		persons.update(() => mappedPersons);
		// update selectedStore when necessary and possible
		if (
			mappedPersons.length > 0 &&
			currentSelected !== null &&
			!mappedPersons.map((person) => person).includes(currentSelected)
		) {
			selected.update(() => mappedPersons[0]);
		}

		// update grid
		const layers = await getPersonLayers();
		const longestRow = layers.map((layer) => layer.length).reduce((a, b) => Math.max(a, b), 0);
		let items: GridItem[][] = [];
		let index = 0;
		layers.forEach((layer) => {
			// sibling row
			const siblingRow = [];
			for (let i = 0; i < longestRow; i++) {
				siblingRow.push(GridItem.fromConnectionArray([]));
			}
			items[index] = siblingRow;

			// person row
			const personRow = layer
				.map((pid) => mappedPersons.find((person) => pid == person.id))
				.map((person) => {
					if (person !== undefined) {
						return GridItem.fromPerson(person);
					} else {
						throw new ReferenceError();
					}
				});
			items[index + 1] = personRow;
			while (items[index + 1].length < longestRow) {
				items[index + 1].push(GridItem.fromConnectionArray([]));
			}
			// relationship row
			const relationshipRow = [];
			for (let i = 0; i < longestRow; i++) {
				relationshipRow.push(GridItem.fromConnectionArray([]));
			}
			items[index + 2] = relationshipRow;

			index += 3;
		});

		grid.update(() => items);
	});
}
