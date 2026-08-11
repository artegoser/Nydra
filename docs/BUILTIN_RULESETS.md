# Nydra Built-in Rulesets

## Purpose

Phase 16 ships three built-in rulesets alongside chess: `nydra-checkers`, `nydra-go`, and `nydra-rift`. They are intentionally playable through the same WASM and Svelte runtime rather than existing only as test fixtures.

All built-ins serve architectural verification, but ruleset completeness is tracked independently. In particular, `nydra-go` now targets the full digital AGA board/scoring rules described in [`GO_RULES_AUDIT.md`](./GO_RULES_AUDIT.md); it is no longer a 9×9/simple-ko scoring proof. Checkers and Rift retain their own current scope. Every built-in must exercise the real end-to-end path:

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

`nydra-go` is a playable digital AGA ruleset and a placement-game architecture proof:

- default 19×19 board, plus 9×9 and 13×13 even/one-stone-handicap games;
- legal board points are exposed directly as `SelectPosition`, with no selected entity;
- placement uses generic entity spawning;
- connected groups and liberties are derived from current state;
- captures remove arbitrary groups transactionally and become prisoners;
- self-capture is rejected;
- AGA situational superko is derived from the complete committed play history;
- pass stones are tracked as prisoners;
- two consecutive passes enter dead-group scoring review;
- disagreement can resume ordinary play and immediate post-dispute passes score remaining stones alive;
- White makes the required final pass when Black would otherwise have passed last;
- territory and area counting are both available with exact half-point komi;
- current AGA 7.5 even-game / 0.5 handicap komi is supported;
- standard 19×19 fixed handicaps 2–9 and one-stone handicap semantics are supported;
- resignation is an authoritative terminal outcome;
- the browser renders stones on Go-line intersections with conventional hoshi points.

Detailed coverage and deliberate tournament/session boundaries are documented in [`GO_RULES_AUDIT.md`](./GO_RULES_AUDIT.md).

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

The browser boundary exposes the generic runtime constructor plus ruleset-specific configuration constructors where a game genuinely has setup parameters:

```text
new_game("chess")
new_game("checkers")
new_game("go")          # standard 19x19 AGA game
new_game("rift")
new_go(size, scoring, handicap)
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
Go:       board position -> spawn -> group capture -> superko -> scoring review
Rift:     entity -> move -> ability -> target -> option -> arbitrary mutation
```

Phase 16 is complete when all four can be played through the same browser/runtime pipeline without adding mechanic-specific concepts to `nydra-core`.
