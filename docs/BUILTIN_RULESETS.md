# Nydra Built-in Reference Rulesets

## Purpose

Phase 16 now ships three compact reference rulesets alongside chess: `nydra-checkers`, `nydra-go`, and `nydra-rift`. They are intentionally playable through the same WASM and Svelte runtime rather than existing only as test fixtures.

Their purpose is architectural verification. They are not claims of tournament-complete checkers/Go implementations or production-balanced game modes. A reference ruleset may stay deliberately small, but it must exercise the real end-to-end path:

```text
ruleset crate
  -> GameState / InteractionDriver
  -> WASM GameHandle
  -> generic GameView / InteractionView
  -> shared Svelte board
  -> ChoiceId back to authoritative Rust
```

No checkers-, Go-, or Rift-specific variant may be added to a generic `nydra-core` enum merely to make a mode work.

## Checkers

`nydra-checkers` demonstrates a movement game whose turn structure differs from chess:

- one checker entity type with piece-local `king` state;
- mandatory captures;
- a capture may force another capture by the same entity;
- one player turn may contain multiple authoritative `StepRecord`s;
- kinging is an ordinary entity-state mutation with a presentation cue;
- active player changes only when a forced capture chain finishes;
- loss by having no legal move is a ruleset-level `OutcomeRule`;
- click and drag use the same generic entity/position choices as chess.

## Go

`nydra-go` demonstrates a game whose primary action is placement rather than movement:

- the built-in board is 9×9 for compact interactive testing;
- a legal board point is exposed directly as `SelectPosition`, with no selected entity;
- placement uses generic entity spawning;
- connected groups and liberties are derived from current state;
- captures remove arbitrary groups transactionally;
- suicide is rejected by the ruleset;
- pass is an ordinary `SelectOption` choice;
- simple ko is derived from the immediately previous committed board position;
- two consecutive passes feed a ruleset-level terminal outcome.

The current terminal score intentionally uses living-stone count instead of complete territory/dead-stone scoring. Full Go scoring is not required for the architecture proof.

## Rift

`nydra-rift` is a synthetic tactical mode for mechanics that chess/checkers/Go do not cover:

- three players and two teams;
- mage entities with arbitrary HP and mana state;
- movement followed by another action in the same turn;
- Fireball: ability -> target -> mode -> damage/removal;
- Rewind reads committed history and restores old HP;
- Hijack changes `controller` without changing `owner`;
- a player left with no controlled units can skip through the ordinary `FinishTurn` choice instead of deadlocking the match;
- semantic presentation cues remain non-authoritative;
- last-team-standing is an `OutcomeRule`;
- the shared UI renders HP/mana from entity presentation data, not rules duplicated in TypeScript.

## Shared browser runtime

The browser boundary exposes one runtime constructor:

```text
new_game("chess")
new_game("checkers")
new_game("go")
new_game("rift")
```

All return the same `GameHandle` interface for:

- `view`;
- `interaction`;
- `choose`;
- `cancelSelection`;
- `undo` / `redo`;
- generic history.

FEN and PGN remain explicit chess-only capabilities. The generic view carries ruleset metadata, generic terminal outcomes, active players, entity presentations, choices, state deltas, and presentation cues.

## Architectural result

The built-ins cover materially different action models:

```text
chess:    entity -> move -> optional piece-local continuation
checkers: entity -> capture -> forced capture continuation
Go:       board position -> spawn -> group capture -> history legality
Rift:     entity -> move -> ability -> target -> option -> arbitrary mutation
```

Phase 16 is complete when all four can be played through the same browser/runtime pipeline without adding mechanic-specific concepts to `nydra-core`.
