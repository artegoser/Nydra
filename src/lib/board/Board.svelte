<script lang="ts">
	import { onMount } from 'svelte';
	import Bg from '$lib/board/Bg.svelte';
	import Pieces from './Pieces.svelte';
	import {
		LocalChessGame,
		type GameView,
		type HistoryTurnView,
		type InteractionView,
		type TransitionView
	} from '$lib/wasm/runtime';

	let runtime: LocalChessGame | null = null;
	let game: GameView | null = null;
	let previousGame: GameView | null = null;
	let interaction: InteractionView = {
		generation: '0',
		selected_entity: null,
		pending_target: null,
		choices: []
	};
	let latestTransition: TransitionView | null = null;
	let animationSeq = 0;
	let history: HistoryTurnView[] = [];
	let currentFen = '';
	let fenDraft = '';
	let currentPgn = '';
	let pgnDraft = '';
	let orientation: 'white' | 'black' = 'white';
	let error: string | null = null;
	let loading = true;

	function syncMetadata() {
		if (!runtime) return;
		history = runtime.history();
		currentFen = runtime.fen();
		currentPgn = runtime.pgn();
		if (!fenDraft) fenDraft = currentFen;
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

	async function installRuntime(next: LocalChessGame) {
		const old = runtime;
		runtime = next;
		previousGame = null;
		latestTransition = null;
		game = next.view();
		interaction = next.interaction();
		history = next.history();
		currentFen = next.fen();
		currentPgn = next.pgn();
		fenDraft = currentFen;
		pgnDraft = currentPgn;
		error = null;
		loading = false;
		old?.dispose();
	}

	async function resetGame() {
		try {
			loading = true;
			await installRuntime(await LocalChessGame.create());
		} catch (cause) {
			loading = false;
			error = cause instanceof Error ? cause.message : String(cause);
		}
	}

	async function loadFen() {
		try {
			loading = true;
			await installRuntime(await LocalChessGame.fromFen(fenDraft.trim()));
		} catch (cause) {
			loading = false;
			error = cause instanceof Error ? cause.message : String(cause);
		}
	}

	async function copyFen() {
		if (!currentFen || typeof navigator === 'undefined') return;
		try {
			await navigator.clipboard.writeText(currentFen);
		} catch {
			// Clipboard availability is browser-dependent; the field remains selectable.
		}
	}

	async function loadPgn() {
		try {
			loading = true;
			await installRuntime(await LocalChessGame.fromPgn(pgnDraft.trim()));
		} catch (cause) {
			loading = false;
			error = cause instanceof Error ? cause.message : String(cause);
		}
	}

	async function copyPgn() {
		if (!currentPgn || typeof navigator === 'undefined') return;
		try {
			await navigator.clipboard.writeText(currentPgn);
		} catch {
			// The textarea remains selectable if clipboard permission is unavailable.
		}
	}

	function statusText(view: GameView) {
		if (!view.status.outcome) return `${view.status.side_to_move} to move${view.status.in_check ? ' · check' : ''}`;
		return view.status.outcome.replaceAll('_', ' ');
	}

	onMount(() => {
		let mounted = true;
		LocalChessGame.create()
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

{#if game}
	<div class="chess-layout">
		<div class="board-column">
			<div class="board-frame" class:orientation-black={orientation === 'black'}>
				<div class="board">
					<Bg width={game.width} height={game.height} {orientation} />
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
				<span>{statusText(game)}</span>
				<span class="board-status-meta">halfmove {game.status.halfmove_clock} · repetition {game.status.repetition_count}</span>
			</div>

			<div class="board-toolbar" aria-label="Board controls">
				<button type="button" on:click={undo} disabled={!game.can_undo}>Undo</button>
				<button type="button" on:click={redo} disabled={!game.can_redo}>Redo</button>
				<button type="button" on:click={resetGame}>Reset</button>
				<button type="button" on:click={() => (orientation = orientation === 'white' ? 'black' : 'white')}>Flip board</button>
			</div>
		</div>

		<aside class="chess-panel">
			<section>
				<h2>Position</h2>
				<div class="fen-current">
					<input readonly value={currentFen} aria-label="Current FEN" />
					<button type="button" on:click={copyFen}>Copy</button>
				</div>
				<form on:submit|preventDefault={loadFen} class="fen-loader">
					<input bind:value={fenDraft} spellcheck="false" aria-label="Load FEN" />
					<button type="submit">Load FEN</button>
				</form>
			</section>

			<section class="history-panel">
				<h2>History</h2>
				{#if history.length === 0}
					<p class="panel-muted">No moves yet.</p>
				{:else}
					<ol>
						{#each history as turn}
							<li><span>{turn.move_number}{turn.side === 'black' ? '...' : '.'}</span> {turn.san}</li>
						{/each}
					</ol>
				{/if}
			</section>

			<section class="pgn-panel">
				<div class="panel-heading-row">
					<h2>PGN</h2>
					<button type="button" on:click={copyPgn}>Copy</button>
				</div>
				<textarea readonly value={currentPgn} rows="9" aria-label="Current PGN"></textarea>
				<form on:submit|preventDefault={loadPgn} class="pgn-loader">
					<textarea bind:value={pgnDraft} rows="6" spellcheck="false" aria-label="Load PGN"></textarea>
					<button type="submit">Load PGN</button>
				</form>
			</section>
		</aside>
	</div>
{:else if error}
	<p class="runtime-error">{error}</p>
{:else}
	<p class="runtime-loading">{loading ? 'Loading Rust/WASM chess runtime…' : 'Chess runtime unavailable.'}</p>
{/if}

{#if error && game}
	<p class="runtime-error">{error}</p>
{/if}
