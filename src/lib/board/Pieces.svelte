<script lang="ts">
	import type { BoardPieces } from '$lib';

	export let board: BoardPieces;

	let width = board.length;
	let height = board[0].length;
</script>

<div class="board-layer pieces" style="--width: {width}; --height: {height};">
	{#each board as row}
		{#each row as piece}
			<div class="square">
				{#if piece != null}
					<img class="piece" src={piece.svg} alt="piece" />
				{/if}
			</div>
		{/each}
	{/each}
</div>
