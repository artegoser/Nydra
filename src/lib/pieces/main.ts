export abstract class Piece {
	abstract coordX: number;
	abstract coordY: number;
	abstract playable: boolean;

	abstract svg: string;

	moved: boolean = false;
}
