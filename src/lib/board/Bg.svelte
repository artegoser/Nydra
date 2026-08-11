<script lang="ts">
	import { numToAlpha } from '$lib/utils';

	export let width: number;
	export let height: number;
	export let orientation: 'white' | 'black' = 'white';
	export let boardStyle: 'checkerboard' | 'go' = 'checkerboard';
	export let showCoordinates = true;

	function boardX(col: number) {
		return orientation === 'white' ? col : width - col - 1;
	}

	function boardY(row: number) {
		return orientation === 'white' ? height - row - 1 : row;
	}
</script>

<div class="board-layer board-background" class:go-board={boardStyle === 'go'} style="--width: {width}; --height: {height};" aria-hidden="true">
	{#each { length: height } as _, row}
		{#each { length: width } as _, col}
			{@const x = boardX(col)}
			{@const y = boardY(row)}
			<div class="board-square" class:light={(x + y) % 2 === 1} class:dark={(x + y) % 2 === 0}>
				{#if showCoordinates && col === 0}
					<span class="board-coordinate rank">{y + 1}</span>
				{/if}
				{#if showCoordinates && row === height - 1}
					<span class="board-coordinate file">{numToAlpha(x)}</span>
				{/if}
			</div>
		{/each}
	{/each}
</div>
