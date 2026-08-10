# Nydra Example Rulesets

## Purpose

`nydra-examples` is an internal architecture-proof crate. The modules in it are deliberately not product modes and are not exposed through the current Svelte/WASM chess UI. Their purpose is to exercise substantially different rule shapes against the same `nydra-core` primitives before generic recording/replay, scripting, multiplayer authority, or additional shipped games are built.

The examples must not introduce checkers-, Go-, or Rift-specific variants into core enums. If an example requires a new generic capability, that capability must be justified independently of the example game.

## Checkers proof

`checkers` demonstrates a movement game whose turn structure differs from chess:

- one generic checker entity type with piece-local `king` state;
- mandatory captures;
- a capture may force another capture by the same entity;
- one player turn may therefore contain multiple authoritative `StepRecord`s;
- promotion is an ordinary entity-state mutation with an optional presentation cue;
- the active player changes only when the forced chain is complete;
- loss by having no legal move is a ruleset-level `OutcomeRule`.

The implementation intentionally stays compact. It is a rules-runtime proof, not a claim of complete tournament-variant coverage.

## Go proof

`go` demonstrates a game whose primary action is placement rather than movement:

- a stone placement uses generic entity spawning;
- connected groups and liberties are derived from the board state;
- captures remove arbitrary groups of entities transactionally;
- suicide is rejected by the ruleset;
- pass is represented as an ordinary `SelectOption` choice;
- simple ko is derived from the immediately previous committed board position;
- two consecutive passes feed a ruleset-level terminal outcome.

The terminal example deliberately uses living-stone count rather than full territory/dead-stone scoring. Full Go scoring is outside Phase 16; the goal is to prove placement, group capture, pass, history-dependent legality, and outcome composition without core changes.

## Rift proof

`rift` is a deliberately synthetic tactical mode designed to exercise mechanics that board-game examples do not cover:

- three players and two teams;
- team membership remains independent from player identity;
- mage entities own arbitrary HP and mana state;
- a turn can contain movement followed by an explicit named ability;
- Fireball selects a target and then a mode before applying damage;
- damage can mutate or remove another entity;
- Rewind reads committed history and copies old HP into the current state;
- Hijack changes `controller` without changing `owner`;
- ability execution emits non-authoritative presentation cues;
- eliminating the last entity belonging to one team is evaluated by an `OutcomeRule`.

The mode exists only to prove that the generic state/history/interaction/ability/outcome APIs compose. It should remain small and deterministic.

## Architectural result

Together these examples cover three materially different action models:

```text
checkers: existing entity -> move/capture -> forced continuation
Go:       board position -> spawn -> group reactions -> history legality
Rift:     entity -> move -> ability -> target -> option -> arbitrary mutation
```

All three use the same generic concepts:

- `GameState` / `EntityState` / `StateMap`;
- `TurnSession` and multiple `StepRecord`s;
- `ChoiceKind` / `ChoiceSpec` / `InteractionDriver`;
- `EntityRule` and `AbilityRule` registration;
- transactional mutation and derived `StateDelta`;
- readable committed `History`;
- `owner`, `controller`, players, and teams;
- `PresentationCue`;
- ruleset-level `OutcomeRule`.

Phase 16 is complete when these examples pass without adding mechanic-specific concepts to `nydra-core`.
