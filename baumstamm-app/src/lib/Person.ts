export class Person {
	id: number;
	info: Map<string, string>;
	firstName: string | null = null;
	lastName: string | null = null;
	image: string | null = null;

	constructor(id: number, info: Map<string, string>) {
		this.id = id;
		const firstName = info.get('@firstName');
		if (firstName !== undefined) {
			this.firstName = firstName;
			info.delete('@firstName');
		}
		const lastName = info.get('@lastName');
		if (lastName !== undefined) {
			this.lastName = lastName;
			info.delete('@lastName');
		}
		const image = info.get('@image');
		if (image !== undefined) {
			this.image = image;
			info.delete('@image');
		}
		this.info = info;
	}

	public name(): string {
		if (this.firstName != null && this.lastName != null) {
			return this.firstName + ' ' + this.lastName;
		} else if (this.firstName != null && this.lastName == null) {
			return this.firstName;
		} else if (this.firstName == null && this.lastName != null) {
			return this.lastName;
		} else {
			return 'Unknown';
		}
	}

	public initials(): string {
		let firstLetter = this.firstName?.at(0);
		let secondLetter = this.lastName
			? [...this.lastName].find((char) => char === char.toUpperCase()) ?? this.lastName?.at(0)
			: undefined;
		return (firstLetter ?? '') + (secondLetter ?? '');
	}

	public avatar(): string {
		return this.image ?? '';
	}
}
