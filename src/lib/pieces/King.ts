import { Piece } from './main';

export class King extends Piece {
	playable: boolean;
	coordX: number;
	coordY: number;
	svg: string;

	constructor(coordX: number, coordY: number, white: boolean = true, playable = true) {
		super();

		this.coordX = coordX;
		this.coordY = coordY;

		this.playable = playable;

		this.svg = white ? '/pieces/KingW.svg' : '/pieces/KingB.svg';
	}
}
