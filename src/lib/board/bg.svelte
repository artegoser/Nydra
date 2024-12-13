<script lang="ts">
	import { numToAlpha } from '$lib/utils';

	export let width = 50;
	export let height = 50;
	export let showCoordinates = true;
	export let allCoordinates = false;
</script>

<div class="board-bg" style="--width: {width}; --height: {height};">
	{#each { length: height } as _, row}
		{#each { length: width } as _, col}
			<div
				class="square {(row + col) % 2 === 0 ? 'white' : 'black'}"
				class:right-bottom-corner={row == height - 1 && col == width - 1}
				class:right-top-corner={row == 0 && col == width - 1}
				class:left-bottom-corner={row == height - 1 && col == 0}
				class:left-top-corner={row == 0 && col == 0}
			>
				{#if allCoordinates}
					<div class="coordinates all">
						{numToAlpha(col)}{height - row}
					</div>
				{:else if showCoordinates && row == height - 1 && showCoordinates && col == width - 1}
					<div class="coordinates columns">
						{numToAlpha(col)}
					</div>
					<div class="coordinates rows">
						{height - row}
					</div>
				{:else if showCoordinates && row == height - 1}
					<div class="coordinates columns">
						{numToAlpha(col)}
					</div>
				{:else if showCoordinates && col == width - 1}
					<div class="coordinates rows">
						{height - row}
					</div>
				{/if}
			</div>
		{/each}
	{/each}
</div>
