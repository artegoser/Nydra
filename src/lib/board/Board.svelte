<script lang="ts">
	import type { BoardPieces } from '$lib';
	import Bg from '$lib/board/Bg.svelte';
	import { King } from '$lib/pieces/King';
	import Pieces from './Pieces.svelte';

	let width = 8;
	let height = 8;

	let board: BoardPieces = Array.from({ length: height }, () => Array(width).fill(new King(0, 0)));
</script>

<div class="board">
	<Bg {width} {height} />
	<Pieces {board} />
</div>
