// place files you want to import through the `$lib` alias in this folder.

import type { Piece } from './pieces/main';

export type BoardPieces = (Piece | null)[][];
