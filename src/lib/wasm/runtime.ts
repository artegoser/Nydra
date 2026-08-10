export interface PositionView {
	x: number;
	y: number;
}

export interface EntityView {
	id: number;
	entity_type: number;
	owner: number;
	controller: number;
	position: PositionView;
	move_count: number;
	asset_key: string;
	label: string | null;
	presentation_data: unknown;
	state: unknown;
}

export interface StatusView {
	side_to_move: 'white' | 'black';
	in_check: boolean;
	outcome: string | null;
	winner: number | null;
	loser: number | null;
	repetition_count: number;
	halfmove_clock: number;
	can_claim_threefold_repetition: boolean;
	can_claim_fifty_move_rule: boolean;
}

export interface GameView {
	width: number;
	height: number;
	entities: EntityView[];
	active_players: number[];
	status: StatusView;
	can_undo: boolean;
	can_redo: boolean;
}

export interface ChoiceView {
	id: string;
	kind: 'select_entity' | 'select_position' | 'select_ability' | 'select_option' | 'finish_turn';
	entity: number | null;
	position: PositionView | null;
	ability: number | null;
	option_key: string | null;
	label: string | null;
	data: unknown;
}

export interface InteractionView {
	generation: string;
	choices: ChoiceView[];
}

export interface ChangeView {
	type: string;
	[key: string]: unknown;
}

export interface PresentationView {
	kind: string;
	data: unknown;
}

export interface TransitionView {
	committed: boolean;
	game: GameView;
	interaction: InteractionView;
	changes: ChangeView[];
	presentation: PresentationView[];
}

export interface HistoryTurnView {
	index: number;
	actor: number;
	actions: Array<{ kind: string; data: unknown }>;
}

interface WasmHandle {
	view(): GameView;
	interaction(): InteractionView;
	choose(choiceId: string): TransitionView;
	undo(): TransitionView;
	redo(): TransitionView;
	fen(): string;
	history(): HistoryTurnView[];
	canUndo(): boolean;
	canRedo(): boolean;
	free(): void;
}

interface WasmModule {
	default(): Promise<unknown>;
	new_chess(): WasmHandle;
	from_fen(fen: string): WasmHandle;
}

let modulePromise: Promise<WasmModule> | null = null;

async function loadModule(): Promise<WasmModule> {
	modulePromise ??= import('./pkg/glorichess.js').then(async (module) => {
		const wasm = module as unknown as WasmModule;
		await wasm.default();
		return wasm;
	});
	return modulePromise;
}

export class LocalChessGame {
	private constructor(private readonly handle: WasmHandle) {}

	static async create(): Promise<LocalChessGame> {
		const wasm = await loadModule();
		return new LocalChessGame(wasm.new_chess());
	}

	static async fromFen(fen: string): Promise<LocalChessGame> {
		const wasm = await loadModule();
		return new LocalChessGame(wasm.from_fen(fen));
	}

	view(): GameView {
		return this.handle.view();
	}

	interaction(): InteractionView {
		return this.handle.interaction();
	}

	choose(choiceId: string): TransitionView {
		return this.handle.choose(choiceId);
	}

	undo(): TransitionView {
		return this.handle.undo();
	}

	redo(): TransitionView {
		return this.handle.redo();
	}

	fen(): string {
		return this.handle.fen();
	}

	history(): HistoryTurnView[] {
		return this.handle.history();
	}

	canUndo(): boolean {
		return this.handle.canUndo();
	}

	canRedo(): boolean {
		return this.handle.canRedo();
	}

	dispose(): void {
		this.handle.free();
	}
}
