export class Piece {
	moved: boolean = false;

	svg: string;

	constructor(name: string, color: string = 'W') {
		this.svg = `/pieces/${color}/${name}.svg`;
	}
}
