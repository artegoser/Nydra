<script lang="ts">
	import { onMount } from 'svelte';
	import Bg from '$lib/board/Bg.svelte';
	import Pieces from './Pieces.svelte';
	import {
		LocalChessGame,
		type ChoiceView,
		type GameView,
		type InteractionView,
		type TransitionView
	} from '$lib/wasm/runtime';

	let runtime: LocalChessGame | null = null;
	let game: GameView | null = null;
	let interaction: InteractionView = { generation: '0', choices: [] };
	let error: string | null = null;

	function applyTransition(transition: TransitionView) {
		game = transition.game;
		interaction = transition.interaction;
	}

	function choose(choiceId: string) {
		if (!runtime) return;
		try {
			applyTransition(runtime.choose(choiceId));
			error = null;
		} catch (cause) {
			error = cause instanceof Error ? cause.message : String(cause);
		}
	}

	$: optionChoices = interaction.choices.filter(
		(choice): choice is ChoiceView & { option_key: string } =>
			choice.kind === 'select_option' && choice.option_key != null
	);

	onMount(() => {
		let mounted = true;
		LocalChessGame.create()
			.then((loaded) => {
				if (!mounted) {
					loaded.dispose();
					return;
				}
				runtime = loaded;
				game = loaded.view();
				interaction = loaded.interaction();
			})
			.catch((cause) => {
				error = cause instanceof Error ? cause.message : String(cause);
			});

		return () => {
			mounted = false;
			runtime?.dispose();
			runtime = null;
		};
	});
</script>

{#if game}
	<div class="board-shell">
		<div class="board">
			<Bg width={game.width} height={game.height} />
			<Pieces {game} {interaction} onchoice={choose} />
		</div>

		<div class="runtime-state">
			<span>{game.status.side_to_move} to move</span>
			{#if game.status.in_check}<span>check</span>{/if}
			{#if game.status.outcome}<span>{game.status.outcome}</span>{/if}
		</div>

		{#if optionChoices.length > 0}
			<div class="runtime-options" aria-label="Choose an option">
				{#each optionChoices as choice}
					<button type="button" on:click={() => choose(choice.id)}>
						{choice.label ?? choice.option_key}
					</button>
				{/each}
			</div>
		{/if}
	</div>
{:else if error}
	<p class="runtime-error">{error}</p>
{:else}
	<p class="runtime-loading">Loading Rust/WASM chess runtime…</p>
{/if}

{#if error && game}
	<p class="runtime-error">{error}</p>
{/if}
