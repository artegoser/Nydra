<script lang="ts">
	export let width = 8;
	export let height = 8;
	export let hints = true; // coordinates
</script>

<div class="board-bg" style="--width: {width}; --height: {height};">
	{#each { length: height } as _, row}
		{#each { length: width } as _, col}
			<div class="square {(row + col) % 2 === 0 ? 'white' : 'black'}">
				{#if (hints && row == height - 1) || col == width - 1}
					{row},{col}
				{/if}
			</div>
		{/each}
	{/each}
</div>
