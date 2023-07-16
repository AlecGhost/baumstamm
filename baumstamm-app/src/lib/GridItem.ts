import { Connections } from './Connections';
import { Person } from './Person';

export class GridItem {
	value: Person | Connections;

	private constructor(value: Person | Connections) {
		this.value = value;
	}

	static fromPerson(value: Person): GridItem {
		return new GridItem(value);
	}

	static fromConnections(value: Connections): GridItem {
		return new GridItem(value);
	}

	public getPerson(): Person {
		if (this.value instanceof Person) {
			return this.value;
		} else {
			throw TypeError();
		}
	}

	public getConnections(): Connections {
		if (this.value instanceof Connections) {
			return this.value;
		} else {
			throw TypeError();
		}
	}

	public isPerson(): boolean {
		return this.value instanceof Person;
	}
}
