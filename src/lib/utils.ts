import type { BoardPieces } from '$lib';
import { Bishop } from './pieces/Bishop';
import { King } from './pieces/King';
import { Knight } from './pieces/Knight';
import { Pawn } from './pieces/Pawn';
import { Queen } from './pieces/Queen';
import { Rook } from './pieces/Rook';

const alphabet = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ';

export function numToAlpha(num: number) {
	let result = '';
	num++;

	while (num > 0) {
		const index = (num - 1) % 26;
		result = alphabet[index] + result;
		num = Math.floor((num - 1) / 26);
	}

	return result;
}

export interface FenCastlingAvailability {
	kingW: boolean;
	queenW: boolean;
	kingB: boolean;
	queenB: boolean;
}

export interface FenMeta {
	active_color: 'W' | 'B';
	castling_availability: FenCastlingAvailability;
	en_passant_target: null | number[];
	halfmove_clock: number;
	moves: number;
}

export interface FenResult {
	board: BoardPieces;
	meta: FenMeta;
}

export function parseFen(fen: string): FenResult {
	const splitted_fen = fen.split(' ');

	const meta = parseFenMeta(splitted_fen.slice(1, 5));
	const board = parseFenRows(splitted_fen[0].split('/'));

	return { meta, board };
}

function parseFenMeta(splitted_meta: string[]): FenMeta {
	return {
		active_color: splitted_meta[0].toUpperCase() == 'W' ? 'W' : 'B',
		en_passant_target: null, // todo
		castling_availability: {
			kingB: true, // todo
			kingW: true, // todo
			queenB: true, // todo
			queenW: true // todo
		},
		halfmove_clock: parseInt(splitted_meta[3]) || 0,
		moves: parseInt(splitted_meta[4]) || 0
	};
}

function parseFenRows(rows: string[]) {
	const board: BoardPieces = [];

	for (const row of rows) {
		const boardRow = [];
		for (const char of row) {
			switch (char) {
				case 'P':
					boardRow.push(new Pawn('W'));
					break;

				case 'N':
					boardRow.push(new Knight('W'));
					break;

				case 'B':
					boardRow.push(new Bishop('W'));
					break;

				case 'R':
					boardRow.push(new Rook('W'));
					break;

				case 'Q':
					boardRow.push(new Queen('W'));
					break;

				case 'K':
					boardRow.push(new King('W'));
					break;

				case 'p':
					boardRow.push(new Pawn('B'));
					break;

				case 'n':
					boardRow.push(new Knight('B'));
					break;

				case 'b':
					boardRow.push(new Bishop('B'));
					break;

				case 'r':
					boardRow.push(new Rook('B'));
					break;

				case 'q':
					boardRow.push(new Queen('B'));
					break;

				case 'k':
					boardRow.push(new King('B'));
					break;

				default:
					for (let i = 0; i < parseInt(char); i++) {
						boardRow.push(null);
					}
			}
		}
		board.push(boardRow);
	}

	return board;
}
