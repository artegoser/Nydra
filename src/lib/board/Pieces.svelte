<script lang="ts">
	import type { ChoiceView, EntityView, GameView, InteractionView } from '$lib/wasm/runtime';

	export let game: GameView;
	export let interaction: InteractionView;
	export let onchoice: (choiceId: string) => void;

	function squareKey(x: number, y: number) {
		return `${x}:${y}`;
	}

	function assetPath(entity: EntityView) {
		const [, side, kind] = entity.asset_key.split('/');
		const directory = side === 'white' ? 'W' : 'B';
		const filename = kind ? kind[0].toUpperCase() + kind.slice(1) : 'Pawn';
		return `/pieces/${directory}/${filename}.svg`;
	}

	$: entities = new Map(game.entities.map((entity) => [squareKey(entity.position.x, entity.position.y), entity]));
	$: entityChoices = new Map(
		interaction.choices
			.filter((choice): choice is ChoiceView & { entity: number } => choice.kind === 'select_entity' && choice.entity != null)
			.map((choice) => [choice.entity, choice])
	);
	$: positionChoices = new Map(
		interaction.choices
			.filter(
				(choice): choice is ChoiceView & { position: { x: number; y: number } } =>
					choice.kind === 'select_position' && choice.position != null
			)
			.map((choice) => [squareKey(choice.position.x, choice.position.y), choice])
	);

	function choiceAt(x: number, y: number) {
		const key = squareKey(x, y);
		const positionChoice = positionChoices.get(key);
		if (positionChoice) return positionChoice;
		const entity = entities.get(key);
		return entity ? entityChoices.get(entity.id) : undefined;
	}
</script>

<div class="board-layer pieces" style="--width: {game.width}; --height: {game.height};">
	{#each { length: game.height } as _, row}
		{@const y = game.height - row - 1}
		{#each { length: game.width } as _, x}
			{@const entity = entities.get(squareKey(x, y))}
			{@const choice = choiceAt(x, y)}
			<button
				type="button"
				class="square interaction-square"
				class:legal-target={positionChoices.has(squareKey(x, y))}
				class:selectable-piece={entity != null && entityChoices.has(entity.id)}
				disabled={!choice}
				on:click={() => choice && onchoice(choice.id)}
				aria-label={choice?.label ?? entity?.label ?? 'Chess square'}
			>
				{#if entity}
					<img class="piece" src={assetPath(entity)} alt={entity.label ?? 'piece'} draggable="false" />
				{/if}
				{#if positionChoices.has(squareKey(x, y))}
					<span class="legal-marker"></span>
				{/if}
			</button>
		{/each}
	{/each}
</div>
