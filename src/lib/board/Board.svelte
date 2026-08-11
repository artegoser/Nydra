<script lang="ts">
	import { onMount } from 'svelte';
	import Bg from '$lib/board/Bg.svelte';
	import Pieces from './Pieces.svelte';
	import {
		LocalGame,
		stateScalar,
		type ChoiceView,
		type GameView,
		type HistoryTurnView,
		type InteractionView,
		type RulesetId,
		type TransitionView
	} from '$lib/wasm/runtime';

	const rulesets: Array<{ id: RulesetId; label: string; description: string }> = [
		{ id: 'chess', label: 'Chess', description: 'Standard chess with SAN, PGN and FEN.' },
		{ id: 'checkers', label: 'Checkers', description: 'Mandatory captures, chains and kinging.' },
		{ id: 'go', label: 'Go', description: '9×9 placement, capture, pass and simple ko.' },
		{ id: 'rift', label: 'Rift', description: 'Three players, teams, HP, mana and abilities.' }
	];

	let runtime: LocalGame | null = null;
	let game: GameView | null = null;
	let previousGame: GameView | null = null;
	let interaction: InteractionView = emptyInteraction();
	let latestTransition: TransitionView | null = null;
	let animationSeq = 0;
	let history: HistoryTurnView[] = [];
	let selectedRuleset: RulesetId = 'chess';
	let currentFen = '';
	let fenDraft = '';
	let currentPgn = '';
	let pgnDraft = '';
	let orientation: 'white' | 'black' = 'white';
	let error: string | null = null;
	let loading = true;

	function emptyInteraction(): InteractionView {
		return {
			generation: '0',
			selected_entity: null,
			pending_target: null,
			active_ability: null,
			choices: []
		};
	}

	$: actionChoices = interaction.choices.filter(
		(choice) =>
			choice.kind === 'select_ability' ||
			choice.kind === 'finish_turn' ||
			(choice.kind === 'select_option' && !choice.asset_key)
	);
	$: currentRuleset = rulesets.find((ruleset) => ruleset.id === selectedRuleset) ?? rulesets[0];

	function syncMetadata(resetDrafts = false) {
		if (!runtime || !game) return;
		history = runtime.history();
		if (game.ruleset === 'chess') {
			currentFen = runtime.fen();
			currentPgn = runtime.pgn();
			if (resetDrafts || !fenDraft) fenDraft = currentFen;
			if (resetDrafts || !pgnDraft) pgnDraft = currentPgn;
		} else {
			currentFen = '';
			currentPgn = '';
			fenDraft = '';
			pgnDraft = '';
		}
	}

	function applyTransition(transition: TransitionView, animate = true) {
		previousGame = game;
		game = transition.game;
		interaction = transition.interaction;
		latestTransition = transition;
		if (animate && transition.changes.length > 0) animationSeq += 1;
		syncMetadata();
	}

	function run(action: () => TransitionView, animate = true) {
		try {
			applyTransition(action(), animate);
			error = null;
		} catch (cause) {
			error = cause instanceof Error ? cause.message : String(cause);
		}
	}

	function choose(choiceId: string) {
		if (!runtime) return;
		run(() => runtime!.choose(choiceId));
	}

	function cancelSelection() {
		if (!runtime) return;
		run(() => runtime!.cancelSelection(), false);
	}

	function undo() {
		if (!runtime || !game?.can_undo) return;
		run(() => runtime!.undo());
	}

	function redo() {
		if (!runtime || !game?.can_redo) return;
		run(() => runtime!.redo());
	}

	async function installRuntime(next: LocalGame) {
		const old = runtime;
		runtime = next;
		selectedRuleset = next.ruleset();
		previousGame = null;
		latestTransition = null;
		animationSeq += 1;
		game = next.view();
		interaction = next.interaction();
		history = next.history();
		error = null;
		loading = false;
		syncMetadata(true);
		old?.dispose();
	}

	async function switchRuleset(ruleset: RulesetId) {
		if (loading || ruleset === selectedRuleset) return;
		try {
			loading = true;
			await installRuntime(await LocalGame.create(ruleset));
		} catch (cause) {
			loading = false;
			error = cause instanceof Error ? cause.message : String(cause);
		}
	}

	async function resetGame() {
		try {
			loading = true;
			await installRuntime(await LocalGame.create(selectedRuleset));
		} catch (cause) {
			loading = false;
			error = cause instanceof Error ? cause.message : String(cause);
		}
	}

	async function loadFen() {
		try {
			loading = true;
			await installRuntime(await LocalGame.fromFen(fenDraft.trim()));
		} catch (cause) {
			loading = false;
			error = cause instanceof Error ? cause.message : String(cause);
		}
	}

	async function loadPgn() {
		try {
			loading = true;
			await installRuntime(await LocalGame.fromPgn(pgnDraft.trim()));
		} catch (cause) {
			loading = false;
			error = cause instanceof Error ? cause.message : String(cause);
		}
	}

	async function copyText(value: string) {
		if (!value || typeof navigator === 'undefined') return;
		try {
			await navigator.clipboard.writeText(value);
		} catch {
			// The source field remains selectable when clipboard permission is unavailable.
		}
	}

	function actionLabel(choice: ChoiceView) {
		if (choice.label) return choice.label;
		if (choice.kind === 'finish_turn') return 'Finish turn';
		if (choice.kind === 'select_ability') return `Ability ${choice.ability ?? ''}`.trim();
		return choice.option_key ?? 'Choose';
	}

	function entityStat(entity: GameView['entities'][number], key: string) {
		return stateScalar(entity.presentation_data, key);
	}

	onMount(() => {
		let mounted = true;
		LocalGame.create('chess')
			.then((loaded) => {
				if (!mounted) {
					loaded.dispose();
					return;
				}
				return installRuntime(loaded);
			})
			.catch((cause) => {
				loading = false;
				error = cause instanceof Error ? cause.message : String(cause);
			});

		return () => {
			mounted = false;
			runtime?.dispose();
			runtime = null;
		};
	});
</script>

<div class="ruleset-tabs" aria-label="Nydra rulesets">
	{#each rulesets as ruleset}
		<button
			type="button"
			class:active={selectedRuleset === ruleset.id}
			disabled={loading}
			on:click={() => switchRuleset(ruleset.id)}
		>{ruleset.label}</button>
	{/each}
</div>

{#if game}
	<div class="game-layout">
		<div class="board-column">
			<div class="ruleset-heading">
				<div>
					<h1>{game.title}</h1>
					<p>{currentRuleset.description}</p>
				</div>
				<span class="ruleset-id">{game.ruleset}</span>
			</div>

			<div class="board-frame" class:orientation-black={orientation === 'black'} class:go-frame={game.board_style === 'go'}>
				<div class="board">
					<Bg
						width={game.width}
						height={game.height}
						{orientation}
						boardStyle={game.board_style}
						showCoordinates={game.ruleset !== 'rift'}
					/>
					<Pieces
						{game}
						{interaction}
						{previousGame}
						transition={latestTransition}
						{animationSeq}
						{orientation}
						onchoice={choose}
						oncancel={cancelSelection}
					/>
				</div>
			</div>

			<div class="board-status" class:terminal={game.status.outcome != null}>
				<span>{game.status.text}</span>
				<span class="board-status-meta">
					{#if game.active_players.length > 0}active {game.active_players.join(', ')}{/if}
				</span>
			</div>

			{#if actionChoices.length > 0}
				<div class="action-bar" aria-label="Available actions">
					{#each actionChoices as choice}
						<button type="button" on:click={() => choose(choice.id)}>{actionLabel(choice)}</button>
					{/each}
				</div>
			{/if}

			<div class="board-toolbar" aria-label="Board controls">
				<button type="button" on:click={undo} disabled={!game.can_undo}>Undo</button>
				<button type="button" on:click={redo} disabled={!game.can_redo}>Redo</button>
				<button type="button" on:click={resetGame}>Reset</button>
				<button type="button" on:click={() => (orientation = orientation === 'white' ? 'black' : 'white')}>Flip board</button>
			</div>
		</div>

		<aside class="game-panel">
			{#if game.ruleset === 'rift'}
				<section>
					<h2>Units</h2>
					<div class="unit-list">
						{#each game.entities as entity}
							<div class="unit-row">
								<strong>{entity.label ?? `Entity ${entity.id}`}</strong>
								<span>owner {entity.owner} · controller {entity.controller}</span>
								<span>HP {entityStat(entity, 'hp') ?? '?'} · MP {entityStat(entity, 'mana') ?? '?'}</span>
							</div>
						{/each}
					</div>
				</section>
			{/if}

			<section class="history-panel">
				<h2>History</h2>
				{#if history.length === 0}
					<p class="panel-muted">No turns yet.</p>
				{:else}
					<ol>
						{#each history as turn}
							<li><span>{turn.turn_number}. {turn.actor_label}</span> {turn.notation}</li>
						{/each}
					</ol>
				{/if}
			</section>

			{#if game.ruleset === 'chess'}
				<section>
					<h2>Position</h2>
					<div class="fen-current">
						<input readonly value={currentFen} aria-label="Current FEN" />
						<button type="button" on:click={() => copyText(currentFen)}>Copy</button>
					</div>
					<form on:submit|preventDefault={loadFen} class="fen-loader">
						<input bind:value={fenDraft} spellcheck="false" aria-label="Load FEN" />
						<button type="submit">Load FEN</button>
					</form>
				</section>

				<section class="pgn-panel">
					<div class="panel-heading-row">
						<h2>PGN</h2>
						<button type="button" on:click={() => copyText(currentPgn)}>Copy</button>
					</div>
					<textarea readonly value={currentPgn} rows="9" aria-label="Current PGN"></textarea>
					<form on:submit|preventDefault={loadPgn} class="pgn-loader">
						<textarea bind:value={pgnDraft} rows="6" spellcheck="false" aria-label="Load PGN"></textarea>
						<button type="submit">Load PGN</button>
					</form>
				</section>
			{:else}
				<section>
					<h2>Runtime proof</h2>
					<p class="panel-muted">
						This mode uses the same Nydra GameHandle, interaction choices, transactional history, undo/redo and board renderer as chess.
					</p>
				</section>
			{/if}
		</aside>
	</div>
{:else if error}
	<p class="runtime-error">{error}</p>
{:else}
	<p class="runtime-loading">{loading ? 'Loading Nydra WASM runtime…' : 'Runtime unavailable.'}</p>
{/if}

{#if error && game}
	<p class="runtime-error">{error}</p>
{/if}
