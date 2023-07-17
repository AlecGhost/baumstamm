export type Passing = {
	connection: number;
};

export type Ending = {
	connection: number;
	origin: 'left' | 'right' | 'both';
};

export type Crossing = {
	connection: number;
	origin: 'left' | 'right' | 'none';
};

export type ConnectionParams = {
	total: number;
	passing: Passing[];
	ending: Ending[];
	crossing: Crossing[];
	orientation: 'up' | 'down';
};

export class Connections {
	total: number;
	passing: Passing[];
	ending: Ending[];
	crossing: Crossing[];
	orientation: 'up' | 'down';

	constructor(params: ConnectionParams) {
		this.total = params.total;
		this.passing = params.passing;
		this.ending = params.ending;
		this.crossing = params.crossing;
		this.orientation = params.orientation;
	}

	static empty(): Connections {
		return new Connections({
			total: 0,
			passing: [],
			ending: [],
			crossing: [],
			orientation: 'up'
		});
	}
}
