<script lang="ts">
	import type { BoardPieces } from '$lib';
	import Bg from '$lib/board/Bg.svelte';
	import { King } from '$lib/pieces/King';
	import { parseFen } from '$lib/utils';
	import Pieces from './Pieces.svelte';

	let width = 8;
	let height = 8;

	let { board, meta } = parseFen('rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1');

	console.log(meta);
</script>

<div class="board">
	<Bg {width} {height} />
	<Pieces {board} />
</div>
