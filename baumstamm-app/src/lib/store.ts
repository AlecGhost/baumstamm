import { writable, get } from 'svelte/store';
import { Person } from './Person';
import { getPersons } from '../bindings';

export const persons = writable<Person[]>([]);
export const selected = writable<Person | null>(null);

export function update() {
    const currentSelected = get(selected);
    getPersons().then((newPersons) => {
        const mappedPersons = newPersons.map((person) => Person.from(person));
        persons.update(() => mappedPersons);
        // update selectedStore when necessary and possible
        if (
            mappedPersons.length > 0 &&
            currentSelected !== null &&
            !mappedPersons.includes(currentSelected)
        ) {
            selected.update(() => mappedPersons[0]);
        }
    });
}
