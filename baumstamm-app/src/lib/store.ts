import { writable } from 'svelte/store';
import { Person } from './Person';
import { getPersons } from '../bindings';

export const personStore = writable<Person[]>([]);
export const selectedStore = writable<Person | null>(null);

export function update() {
	getPersons().then((persons) => {
		const newPersons = persons.map((person) => Person.from(person));
		personStore.update(() => newPersons);
        // update selectedStore when necessary and possible
		if (newPersons.length > 0) {
            selectedStore.update((currentPerson) => {
                if (currentPerson !== null && newPersons.includes(currentPerson)) {
                    return currentPerson;
                } else {
                    return newPersons[0];
                }
            });
		}
	});
}
