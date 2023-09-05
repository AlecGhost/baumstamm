import type { Person as RPerson } from '../bindings';

export class Person {
	id: string;
	info: Map<string, string>;
	firstName: string | null = null;
	middleName: string | null = null;
	lastName: string | null = null;
	birthDate: string | null = null;
	deathDate: string | null = null;
	image: string | null = null;

	constructor(id: string, info: Map<string, string>) {
		this.id = id;
		const firstName = info.get('@firstName');
		if (firstName !== undefined) {
			this.firstName = firstName;
			info.delete('@firstName');
		}
		const middleName = info.get('@middleName');
		if (middleName !== undefined) {
			this.middleName = middleName;
			info.delete('@middleName');
		}
		const lastName = info.get('@lastName');
		if (lastName !== undefined) {
			this.lastName = lastName;
			info.delete('@lastName');
		}
		const birthDate = info.get('@birthDate');
		if (birthDate !== undefined) {
			this.birthDate = birthDate;
			info.delete('@birthDate');
		}
		const deathDate = info.get('@deathDate');
		if (deathDate !== undefined) {
			this.deathDate = deathDate;
			info.delete('@deathDate');
		}
		const image = info.get('@image');
		if (image !== undefined) {
			this.image = image;
			info.delete('@image');
		}
		this.info = info;
	}

	static from(person: RPerson): Person {
		return new Person(
			person.id,
			person.info !== null ? new Map(Object.entries(person.info)) : new Map()
		);
	}

	public fullName(): string {
		if (this.firstName == null && this.middleName == null && this.lastName == null) {
			return 'Unknown';
		}
		let name = '';
		if (this.firstName !== null) {
			name += this.firstName;
		}
		if (this.middleName !== null) {
			if (name.length !== 0) {
				name += ' ';
			}
			name += this.middleName;
		}
		if (this.lastName !== null) {
			if (name.length !== 0) {
				name += ' ';
			}
			name += this.lastName;
		}
		return name;
	}

	public nameWithoutMiddle(): string {
		if (this.firstName == null && this.middleName == null && this.lastName == null) {
			return 'Unknown';
		}
		let name = '';
		if (this.firstName !== null) {
			name += this.firstName;
		}
		if (this.lastName !== null) {
			if (name.length !== 0) {
				name += ' ';
			}
			name += this.lastName;
		}
		return name;
	}

	public initials(): string {
		let firstLetter = this.firstName?.at(0);
		let secondLetter = this.lastName
			? [...this.lastName].find((char) => char === char.toUpperCase()) ?? this.lastName?.at(0)
			: undefined;
		if (firstLetter == null && secondLetter == null) {
			return '?';
		} else {
			return (firstLetter ?? '') + (secondLetter ?? '');
		}
	}

	public avatar(): string {
		return this.image ?? '';
	}
}
