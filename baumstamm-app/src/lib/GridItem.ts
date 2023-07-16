import type { Connection } from './Connection';
import { Person } from './Person';

export class GridItem {
	value: Person | Connection[];

	private constructor(value: Person | Connection[]) {
		this.value = value;
	}

	static fromPerson(value: Person): GridItem {
		return new GridItem(value);
	}

	static fromConnectionArray(value: Connection[]): GridItem {
		return new GridItem(value);
	}

	public getPerson(): Person {
		if (this.value instanceof Person) {
			return this.value;
		} else {
			throw TypeError();
		}
	}

	public getConnections(): Connection[] {
		// use Array for the lack of runtime generic type checking
		if (this.value instanceof Array) {
			return this.value;
		} else {
			throw TypeError();
		}
	}

	public isPerson(): boolean {
		return this.value instanceof Person;
	}
}
