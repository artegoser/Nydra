<script lang="ts">
	import type {
		ChoiceView,
		EntityView,
		GameView,
		InteractionView,
		PositionView,
		TransitionView
	} from '$lib/wasm/runtime';

	export let game: GameView;
	export let interaction: InteractionView;
	export let previousGame: GameView | null = null;
	export let transition: TransitionView | null = null;
	export let animationSeq = 0;
	export let orientation: 'white' | 'black' = 'white';
	export let onchoice: (choiceId: string) => void;
	export let oncancel: () => void;

	type DragState = {
		pointerId: number;
		entityId: number;
		origin: PositionView;
		startX: number;
		startY: number;
		x: number;
		y: number;
		started: boolean;
		previouslySelected: boolean;
	};

	type Arrow = { from: PositionView; to: PositionView; color: string };
	type SquareAnnotation = { position: PositionView; color: string };

	let boardElement: HTMLDivElement;
	let drag: DragState | null = null;
	let hover: PositionView | null = null;
	let suppressClick = false;
	let drawStart: PositionView | null = null;
	let drawCurrent: PositionView | null = null;
	let drawColor = '#15781b';
	let arrows: Arrow[] = [];
	let squareAnnotations: SquareAnnotation[] = [];
	let motionOffsets: Record<number, { x: number; y: number }> = {};
	let assetOverrides: Record<number, string> = {};
	let fadingPieces: EntityView[] = [];
	let fadingActive = false;
	let seenAnimationSeq = -1;
	let cleanupTimer: ReturnType<typeof setTimeout> | null = null;

	function key(position: PositionView) {
		return `${position.x}:${position.y}`;
	}

	function samePosition(a: PositionView | null | undefined, b: PositionView | null | undefined) {
		return a != null && b != null && a.x === b.x && a.y === b.y;
	}

	function assetPathFromKey(assetKey: string) {
		const [, side, kind] = assetKey.split('/');
		const directory = side === 'white' ? 'W' : 'B';
		const filename = kind ? kind[0].toUpperCase() + kind.slice(1) : 'Pawn';
		return `/pieces/${directory}/${filename}.svg`;
	}

	function assetPath(entity: EntityView) {
		return assetPathFromKey(assetOverrides[entity.id] ?? entity.asset_key);
	}

	function display(position: PositionView) {
		return orientation === 'white'
			? { x: position.x, y: game.height - position.y - 1 }
			: { x: game.width - position.x - 1, y: position.y };
	}

	function entityStyle(entity: EntityView) {
		const shown = display(entity.position);
		const offset = motionOffsets[entity.id] ?? { x: 0, y: 0 };
		if (drag?.started && drag.entityId === entity.id && boardElement) {
			const bounds = boardElement.getBoundingClientRect();
			const square = bounds.width / game.width;
			return `left:${drag.x - square / 2}px;top:${drag.y - square / 2}px;width:${square}px;height:${square}px;transform:none;transition:none;`;
		}
		return `left:${(shown.x * 100) / game.width}%;top:${(shown.y * 100) / game.height}%;width:${100 / game.width}%;height:${100 / game.height}%;transform:translate3d(${offset.x * 100}%,${offset.y * 100}%,0);`;
	}

	function fadingStyle(entity: EntityView) {
		const shown = display(entity.position);
		return `left:${(shown.x * 100) / game.width}%;top:${(shown.y * 100) / game.height}%;width:${100 / game.width}%;height:${100 / game.height}%;`;
	}

	function squareStyle(position: PositionView) {
		const shown = display(position);
		return `left:${(shown.x * 100) / game.width}%;top:${(shown.y * 100) / game.height}%;width:${100 / game.width}%;height:${100 / game.height}%;`;
	}

	function center(position: PositionView) {
		const shown = display(position);
		return {
			x: ((shown.x + 0.5) * 100) / game.width,
			y: ((shown.y + 0.5) * 100) / game.height
		};
	}

	$: entitiesBySquare = new Map(game.entities.map((entity) => [key(entity.position), entity]));
	$: entitiesById = new Map(game.entities.map((entity) => [entity.id, entity]));
	$: entityChoices = new Map(
		interaction.choices
			.filter(
				(choice): choice is ChoiceView & { entity: number } =>
					choice.kind === 'select_entity' && choice.entity != null
			)
			.map((choice) => [choice.entity, choice])
	);
	$: positionChoices = interaction.choices.filter(
		(choice): choice is ChoiceView & { position: PositionView; actor: number } =>
			choice.kind === 'select_position' && choice.position != null && choice.actor != null
	);
	$: visualActor = drag?.entityId ?? interaction.selected_entity;
	$: visibleDestinations =
		visualActor == null ? [] : positionChoices.filter((choice) => choice.actor === visualActor);
	$: destinationBySquare = new Map(visibleDestinations.map((choice) => [key(choice.position), choice]));
	$: selectedPosition =
		visualActor == null ? null : (entitiesById.get(visualActor)?.position ?? drag?.origin ?? null);
	$: optionChoices = interaction.choices.filter(
		(choice): choice is ChoiceView & { option_key: string } =>
			choice.kind === 'select_option' && choice.option_key != null
	);

	function choiceForMove(actor: number, position: PositionView) {
		return positionChoices.find(
			(choice) => choice.actor === actor && samePosition(choice.position, position)
		);
	}

	function positionFromPointer(clientX: number, clientY: number): PositionView | null {
		if (!boardElement) return null;
		const bounds = boardElement.getBoundingClientRect();
		if (
			clientX < bounds.left ||
			clientX >= bounds.right ||
			clientY < bounds.top ||
			clientY >= bounds.bottom
		)
			return null;
		let screenX = Math.floor(((clientX - bounds.left) / bounds.width) * game.width);
		let screenY = Math.floor(((clientY - bounds.top) / bounds.height) * game.height);
		if (orientation === 'black') {
			screenX = game.width - screenX - 1;
			return { x: screenX, y: screenY };
		}
		return { x: screenX, y: game.height - screenY - 1 };
	}

	function clearAnnotations() {
		arrows = [];
		squareAnnotations = [];
	}

	function annotationColor(event: PointerEvent) {
		if (event.shiftKey) return '#882020';
		if (event.altKey) return '#003088';
		if (event.ctrlKey || event.metaKey) return '#e68f00';
		return '#15781b';
	}

	function handlePointerDown(event: PointerEvent, position: PositionView) {
		if (event.button === 2) {
			event.preventDefault();
			drawStart = position;
			drawCurrent = position;
			drawColor = annotationColor(event);
			(event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
			return;
		}
		if (event.button !== 0) return;
		const entity = entitiesBySquare.get(key(position));
		if (!entity || !entityChoices.has(entity.id)) return;
		if (arrows.length || squareAnnotations.length) clearAnnotations();
		drag = {
			pointerId: event.pointerId,
			entityId: entity.id,
			origin: entity.position,
			startX: event.clientX,
			startY: event.clientY,
			x: event.clientX - boardElement.getBoundingClientRect().left,
			y: event.clientY - boardElement.getBoundingClientRect().top,
			started: false,
			previouslySelected: interaction.selected_entity === entity.id
		};
		(event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
	}

	function handlePointerMove(event: PointerEvent) {
		if (drawStart) {
			drawCurrent = positionFromPointer(event.clientX, event.clientY);
			return;
		}
		if (!drag || drag.pointerId !== event.pointerId) return;
		const distance = Math.hypot(event.clientX - drag.startX, event.clientY - drag.startY);
		if (!drag.started && distance >= 3) drag = { ...drag, started: true };
		if (!drag.started) return;
		if (event.pointerType === 'touch') event.preventDefault();
		const bounds = boardElement.getBoundingClientRect();
		drag = {
			...drag,
			x: event.clientX - bounds.left,
			y: event.clientY - bounds.top
		};
		const candidate = positionFromPointer(event.clientX, event.clientY);
		hover = candidate && choiceForMove(drag.entityId, candidate) ? candidate : null;
	}

	function finishDrawing(end: PositionView | null) {
		if (!drawStart) return;
		if (end && samePosition(drawStart, end)) {
			const existing = squareAnnotations.findIndex((item) => samePosition(item.position, end));
			if (existing >= 0) squareAnnotations = squareAnnotations.filter((_, index) => index !== existing);
			else squareAnnotations = [...squareAnnotations, { position: end, color: drawColor }];
		} else if (end) {
			const existing = arrows.findIndex(
				(item) => samePosition(item.from, drawStart) && samePosition(item.to, end)
			);
			if (existing >= 0) arrows = arrows.filter((_, index) => index !== existing);
			else arrows = [...arrows, { from: drawStart, to: end, color: drawColor }];
		}
		drawStart = null;
		drawCurrent = null;
	}

	function handlePointerUp(event: PointerEvent) {
		if (drawStart) {
			event.preventDefault();
			finishDrawing(positionFromPointer(event.clientX, event.clientY));
			suppressClick = true;
			return;
		}
		if (!drag || drag.pointerId !== event.pointerId) return;
		const finished = drag;
		drag = null;
		hover = null;
		if (!finished.started) return;
		suppressClick = true;
		const destination = positionFromPointer(event.clientX, event.clientY);
		const choice = destination ? choiceForMove(finished.entityId, destination) : undefined;
		if (choice && !samePosition(destination, finished.origin)) onchoice(choice.id);
		else if (finished.previouslySelected) oncancel();
	}

	function handleSquareClick(position: PositionView) {
		if (suppressClick) {
			suppressClick = false;
			return;
		}
		const entity = entitiesBySquare.get(key(position));
		if (entity && entityChoices.has(entity.id)) {
			onchoice(entityChoices.get(entity.id)!.id);
			return;
		}
		if (interaction.selected_entity != null) {
			const choice = choiceForMove(interaction.selected_entity, position);
			if (choice) onchoice(choice.id);
			else oncancel();
		}
	}

	function startTransitionAnimation() {
		if (seenAnimationSeq === animationSeq) return;
		seenAnimationSeq = animationSeq;
		if (!transition || !previousGame || typeof window === 'undefined') return;
		if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) return;
		const previousById = new Map(previousGame.entities.map((entity) => [entity.id, entity]));
		const nextById = new Map(game.entities.map((entity) => [entity.id, entity]));
		const offsets: Record<number, { x: number; y: number }> = {};
		const overrides: Record<number, string> = {};
		const removed: EntityView[] = [];

		for (const change of transition.changes) {
			if (change.type === 'entity_moved') {
				const entity = Number(change.entity);
				const from = change.from as PositionView;
				const to = change.to as PositionView;
				const fromDisplay = display(from);
				const toDisplay = display(to);
				offsets[entity] = {
					x: fromDisplay.x - toDisplay.x,
					y: fromDisplay.y - toDisplay.y
				};
			}
			if (change.type === 'entity_removed') {
				const snapshot = change.entity as { id?: number } | undefined;
				const old = snapshot?.id != null ? previousById.get(snapshot.id) : undefined;
				if (old) removed.push(old);
			}
			if (change.type === 'entity_type_changed') {
				const entity = Number(change.entity);
				const old = previousById.get(entity);
				if (old && nextById.has(entity)) overrides[entity] = old.asset_key;
			}
		}

		motionOffsets = offsets;
		assetOverrides = overrides;
		fadingPieces = removed;
		fadingActive = false;
		if (cleanupTimer) clearTimeout(cleanupTimer);
		requestAnimationFrame(() =>
			requestAnimationFrame(() => {
				motionOffsets = Object.fromEntries(Object.keys(offsets).map((id) => [Number(id), { x: 0, y: 0 }]));
				fadingActive = true;
			})
		);
		cleanupTimer = setTimeout(() => {
			motionOffsets = {};
			assetOverrides = {};
			fadingPieces = [];
			fadingActive = false;
		}, 220);
	}

	$: if (animationSeq !== seenAnimationSeq) startTransitionAnimation();
</script>

<div
	class="board-layer board-interaction"
	bind:this={boardElement}
	style="--width: {game.width}; --height: {game.height};"
	on:pointermove={handlePointerMove}
	on:pointerup={handlePointerUp}
	on:pointercancel={handlePointerUp}
	on:contextmenu|preventDefault
>
	<div class="square-state-layer" aria-hidden="true">
		{#each { length: game.height } as _, y}
			{#each { length: game.width } as _, x}
				{@const position = { x, y }}
				{@const destination = destinationBySquare.get(key(position))}
				{@const occupied = entitiesBySquare.has(key(position))}
				<div
					class="board-state-square"
					class:selected={samePosition(selectedPosition, position)}
					class:last-move={samePosition(game.last_move?.from, position) || samePosition(game.last_move?.to, position)}
					class:check={samePosition(game.status.checked_king, position)}
					class:move-dest={destination != null && !occupied}
					class:capture-dest={destination != null && occupied}
					class:destination-hover={destination != null && samePosition(hover, position)}
					style={squareStyle(position)}
				></div>
			{/each}
		{/each}
	</div>

	<div class="annotation-layer" aria-hidden="true">
		{#each squareAnnotations as annotation}
			<div
				class="annotation-square"
				style={`${squareStyle(annotation.position)}background:${annotation.color};`}
			></div>
		{/each}
		<svg viewBox="0 0 100 100" preserveAspectRatio="none">
			<defs>
				<marker id="arrow-green" markerWidth="4" markerHeight="4" refX="3.1" refY="2" orient="auto"><path d="M0,0 L4,2 L0,4 Z" fill="#15781b" /></marker>
				<marker id="arrow-red" markerWidth="4" markerHeight="4" refX="3.1" refY="2" orient="auto"><path d="M0,0 L4,2 L0,4 Z" fill="#882020" /></marker>
				<marker id="arrow-blue" markerWidth="4" markerHeight="4" refX="3.1" refY="2" orient="auto"><path d="M0,0 L4,2 L0,4 Z" fill="#003088" /></marker>
				<marker id="arrow-yellow" markerWidth="4" markerHeight="4" refX="3.1" refY="2" orient="auto"><path d="M0,0 L4,2 L0,4 Z" fill="#e68f00" /></marker>
			</defs>
			{#each arrows as arrow}
				{@const from = center(arrow.from)}
				{@const to = center(arrow.to)}
				{@const marker = arrow.color === '#882020' ? 'red' : arrow.color === '#003088' ? 'blue' : arrow.color === '#e68f00' ? 'yellow' : 'green'}
				<line x1={from.x} y1={from.y} x2={to.x} y2={to.y} stroke={arrow.color} stroke-width="1.8" stroke-linecap="round" marker-end={`url(#arrow-${marker})`} />
			{/each}
			{#if drawStart && drawCurrent && !samePosition(drawStart, drawCurrent)}
				{@const from = center(drawStart)}
				{@const to = center(drawCurrent)}
				<line x1={from.x} y1={from.y} x2={to.x} y2={to.y} stroke={drawColor} stroke-width="1.8" stroke-linecap="round" opacity="0.75" />
			{/if}
		</svg>
	</div>

	<div class="piece-layer" aria-hidden="true">
		{#each fadingPieces as entity (entity.id)}
			<img
				class="piece-node fading-piece"
				class:fading-active={fadingActive}
				style={fadingStyle(entity)}
				src={assetPathFromKey(entity.asset_key)}
				alt=""
				draggable="false"
			/>
		{/each}
		{#each game.entities as entity (entity.id)}
			{#if drag?.started && drag.entityId === entity.id}
				<img class="piece-node origin-ghost" style={fadingStyle(entity)} src={assetPath(entity)} alt="" draggable="false" />
			{/if}
			<img
				class="piece-node"
				class:dragging={drag?.started && drag.entityId === entity.id}
				style={entityStyle(entity)}
				src={assetPath(entity)}
				alt=""
				draggable="false"
			/>
		{/each}
	</div>

	<div class="hit-layer" role="grid" aria-label="Chess board">
		{#each { length: game.height } as _, row}
			{#each { length: game.width } as _, col}
				{@const x = orientation === 'white' ? col : game.width - col - 1}
				{@const y = orientation === 'white' ? game.height - row - 1 : row}
				{@const position = { x, y }}
				{@const entity = entitiesBySquare.get(key(position))}
				<button
					type="button"
					class="board-hit-square"
					class:movable-origin={entity != null && entityChoices.has(entity.id)}
					class:legal-target={destinationBySquare.has(key(position))}
					aria-label={entity?.label ? `${entity.label} on ${x + 1},${y + 1}` : `Square ${x + 1},${y + 1}`}
					on:pointerdown={(event) => handlePointerDown(event, position)}
					on:click={() => handleSquareClick(position)}
				></button>
			{/each}
		{/each}
	</div>

	{#if optionChoices.length > 0 && interaction.pending_target}
		{@const shown = display(interaction.pending_target)}
		<div
			class="promotion-menu"
			class:from-top={shown.y === 0}
			class:from-bottom={shown.y !== 0}
			style={`left:${(shown.x * 100) / game.width}%;${shown.y === 0 ? 'top:0;' : 'bottom:0;'}width:${100 / game.width}%;`}
			aria-label="Choose promotion piece"
		>
			{#each optionChoices as choice}
				<button type="button" on:click={() => onchoice(choice.id)} aria-label={choice.label ?? choice.option_key}>
					{#if choice.asset_key}<img src={assetPathFromKey(choice.asset_key)} alt={choice.label ?? choice.option_key} draggable="false" />{/if}
				</button>
			{/each}
		</div>
	{/if}
</div>
