# GloriChess Rust Core Implementation Checklist

This checklist tracks the implementation described in `CORE_IMPLEMENTATION_PLAN.md`.

## Architectural invariants

- [ ] `glorichess-core` contains no chess-specific piece/rule concepts.
- [ ] Core player identity uses `PlayerId`, not white/black.
- [ ] Core supports `TeamId` independently from `PlayerId`.
- [ ] Entities have distinct `owner` and `controller` fields.
- [ ] Entities have stable IDs and an extensible custom-state mechanism.
- [ ] Entities expose `move_count` or equivalent persistent movement state.
- [ ] History is readable by rules, not only by undo code.
- [ ] Normal-play en passant is derived from history rather than a global target flag.
- [ ] Normal-play castling eligibility is derived from entity/current state rather than global castling-right bits.
- [ ] Piece/rule code can mutate a transactional working game state directly.
- [ ] Gameplay is not constrained by a closed central `Effect` enum.
- [ ] Structural state changes are separately exposed for frontend animation/debugging.
- [ ] Optional semantic presentation cues are non-authoritative.
- [ ] One player turn may contain multiple sequential steps.
- [ ] Continuation choices are computed from the updated working state after each step.
- [ ] Frontend interaction is driven by Rust-provided choices/opaque IDs.
- [ ] Chess attack semantics are separate from movement semantics.
- [ ] Speculative legality states are never committed to history.

## Phase 0 — Repository preparation

- [x] Add root `Cargo.toml` workspace.
- [x] Add `crates/glorichess-core`.
- [x] Add `crates/glorichess-chess`.
- [x] Add `crates/glorichess-wasm`.
- [x] Define workspace dependency/version policy.
- [x] Add Rust formatting/lint/test commands.
- [x] Add WASM browser build integration skeleton.
- [x] Keep the existing Svelte application building during migration.
- [x] Document local development/build commands.

## Phase 1 — Generic world state

- [x] Implement `PlayerId`.
- [x] Implement `TeamId`.
- [x] Implement `EntityId`.
- [x] Implement `EntityTypeId`.
- [x] Implement `AbilityId`.
- [x] Implement `ChoiceId`.
- [x] Implement rectangular `Position`/coordinate primitives.
- [x] Implement rectangular `Board`.
- [x] Implement entity placement lookup.
- [x] Implement `PlayerState`/player store.
- [x] Implement `TeamState`/team store.
- [x] Implement `EntityState`/entity store.
- [x] Add entity `owner`.
- [x] Add entity `controller`.
- [x] Add entity `move_count`.
- [x] Add extensible entity-local state storage.
- [x] Add ruleset-local state extension point.
- [x] Define and enforce core state invariants.
- [x] Unit-test board/entity/player/team operations.

## Phase 2 — History and turn sessions

- [x] Implement full `GameState` snapshots.
- [x] Implement `History`.
- [x] Implement `TurnRecord`.
- [x] Implement `StepRecord`.
- [x] Implement `TurnSession` with `before` and mutable `working` state.
- [x] Record multiple steps inside one turn.
- [x] Implement turn commit.
- [x] Implement turn rollback/cancel.
- [x] Add `previous_turn()` history query.
- [x] Add `last_step()` history query.
- [x] Add state-at-turn/history queries.
- [x] Add entity-at-history queries.
- [x] Ensure speculative legality checks do not enter history.
- [x] Implement undo foundation.
- [x] Implement redo foundation.
- [x] Test snapshot/history integrity across multi-step turns.

## Phase 3 — Generic interaction protocol

- [x] Define generic interaction/choice data model.
- [x] Support selecting an entity.
- [x] Support selecting a board position.
- [x] Support selecting an ability.
- [x] Support selecting a generic option.
- [x] Support explicit `FinishTurn` when a ruleset needs it.
- [x] Assign opaque stable-enough `ChoiceId`s for a current interaction state.
- [x] Reject stale/invalid choice IDs safely.
- [x] Allow a simple move to skip an unnecessary ability menu.
- [x] Re-query choices after every committed step in a turn session.
- [x] Add test-only multi-step interaction flow.
- [x] Add test-only forced-continuation flow.

## Phase 4 — Transactional mutation and change tracing

- [x] Implement transactional working-state mutation.
- [x] Provide safe entity lookup/mutation helpers.
- [x] Provide move helper.
- [x] Provide spawn helper.
- [x] Provide remove helper.
- [x] Provide player/team/ruleset-state mutation access where permitted.
- [x] Define controlled raw-state mutation escape hatch.
- [x] Validate transaction before committing a step.
- [x] Compute or record structural before/after state changes.
- [x] Trace entity movement.
- [x] Trace entity addition/removal.
- [x] Trace entity type changes.
- [x] Trace custom entity-state changes.
- [x] Add optional presentation-cue channel.
- [x] Ensure presentation data cannot alter authoritative state.
- [x] Test rollback on invalid mutations.

## Phase 5 — Rules and presentation registration

- [x] Define generic entity-rule interface.
- [x] Define generic ruleset/game-rule interface.
- [x] Reserve a clean ability-rule extension point.
- [x] Implement rule/type registry without spreading centralized matches through core.
- [x] Provide rule context with current state.
- [x] Provide rule context with history.
- [x] Provide rule context with current turn session/steps.
- [x] Define frontend-facing entity presentation metadata.
- [x] Ensure presentation can depend on current entity/state context.
- [x] Test registering a non-chess test entity without modifying core.

## Phase 6 — Standard chess pieces

### Chess setup

- [ ] Define `ChessSide` in `glorichess-chess` only.
- [ ] Map `ChessSide::White` to a concrete `PlayerId`.
- [ ] Map `ChessSide::Black` to a concrete `PlayerId`.
- [ ] Create standard 8×8 chess board setup.
- [ ] Create standard initial entity placement.
- [ ] Register all six standard piece rules.

### Pawn

- [ ] Generate one-square forward movement.
- [ ] Generate legal initial two-square pseudo-movement.
- [ ] Generate diagonal attacked squares separately from movement.
- [ ] Generate ordinary diagonal captures.
- [ ] Respect blocking on forward movement.

### Knight

- [ ] Generate all L-shaped pseudo-moves.
- [ ] Generate knight attacked squares.
- [ ] Handle friendly/enemy occupancy correctly.

### Bishop

- [ ] Generate diagonal rays.
- [ ] Stop rays correctly at first occupied square.
- [ ] Generate attacked squares correctly.

### Rook

- [ ] Generate orthogonal rays.
- [ ] Stop rays correctly at first occupied square.
- [ ] Generate attacked squares correctly.

### Queen

- [ ] Generate combined bishop/rook rays.
- [ ] Generate attacked squares correctly.

### King

- [ ] Generate adjacent pseudo-moves.
- [ ] Generate adjacent attacked squares independently from move legality.

## Phase 7 — Attack maps and legal chess actions

- [ ] Locate each side's king robustly.
- [ ] Query whether a square is attacked by a given side/player.
- [ ] Detect current check.
- [ ] Apply candidate action to a speculative state.
- [ ] Reject actions leaving own king attacked.
- [ ] Handle pinned pieces correctly.
- [ ] Handle discovered checks correctly.
- [ ] Handle double check correctly.
- [ ] Validate king moves into/out of attacked squares.
- [ ] Validate king captures using resulting-state attacks.
- [ ] Expose only legal chess destinations/options to frontend interactions.

## Phase 8 — Special chess moves

### Pawn double move

- [ ] Require appropriate starting geometry/state.
- [ ] Require `move_count == 0` or equivalent chess condition.
- [ ] Require both traversed/destination cells to be free.
- [ ] Increment movement state on execution.

### En passant

- [ ] Do not add a normal-play global en-passant target flag.
- [ ] Pawn detects adjacent enemy pawn.
- [ ] Pawn inspects the immediately preceding completed action/state transition.
- [ ] Verify the exact adjacent pawn was the previous actor/mover.
- [ ] Verify the pawn moved exactly two ranks in a legal initial double move.
- [ ] Generate the correct destination.
- [ ] Remove the bypassed pawn during the candidate transition.
- [ ] Allow en passant only immediately after the double move.
- [ ] Reject en passant if the resulting state exposes own king.
- [ ] Test en passant for both colors/sides.

### Castling

- [ ] Do not add normal-play global castling-right bits.
- [ ] Require king `move_count == 0`.
- [ ] Locate the correct rook.
- [ ] Require rook `move_count == 0`.
- [ ] Require clear path.
- [ ] Reject while king is currently in check.
- [ ] Reject if transit square is attacked.
- [ ] Reject if destination square is attacked.
- [ ] Move king and rook in one chess action/step.
- [ ] Support king-side castling for both sides.
- [ ] Support queen-side castling for both sides.
- [ ] Verify rook moved away and returned still cannot castle.
- [ ] Verify king moved away and returned still cannot castle.

### Promotion

- [ ] Detect promotion rank after pawn move/capture selection.
- [ ] Request explicit promotion choice from Rust interaction layer.
- [ ] Offer queen.
- [ ] Offer rook.
- [ ] Offer bishop.
- [ ] Offer knight.
- [ ] Apply chosen type change.
- [ ] Support promotion after capture.
- [ ] Support underpromotion.

## Phase 9 — Chess outcomes and draw rules

- [ ] Detect checkmate.
- [ ] Detect stalemate.
- [ ] Add resignation outcome/API.
- [ ] Add draw-by-agreement outcome/API.
- [ ] Build a repetition-equivalence position key.
- [ ] Include side to move in repetition equivalence.
- [ ] Include effective castling possibilities in repetition equivalence.
- [ ] Include effective en-passant possibilities in repetition equivalence.
- [ ] Detect claimable threefold repetition.
- [ ] Detect automatic fivefold repetition.
- [ ] Track/reset the halfmove clock semantics needed for draw rules.
- [ ] Detect claimable fifty-move rule.
- [ ] Detect automatic seventy-five-move rule.
- [ ] Implement dead-position detection.
- [ ] Ensure checkmate takes precedence where required over automatic move-count draw handling.

## Phase 10 — FEN

- [ ] Remove TypeScript FEN authority.
- [ ] Parse piece placement in Rust.
- [ ] Parse side to move.
- [ ] Parse castling field.
- [ ] Parse en-passant target field.
- [ ] Parse halfmove clock.
- [ ] Parse fullmove number.
- [ ] Serialize complete FEN.
- [ ] Reconstruct movement metadata consistent with imported castling rights.
- [ ] Ensure absent castling rights do not accidentally reappear from original-piece placement.
- [ ] Synthesize minimal previous pawn state/action for imported en-passant target semantics.
- [ ] Keep synthetic import history clearly distinct from known real history internally if needed.
- [ ] Add FEN validation/errors.
- [ ] Add FEN roundtrip tests.
- [ ] Add imported castling/en-passant continuation tests.

## Phase 11 — WASM bridge

- [ ] Create browser-facing `GameHandle`.
- [ ] Expose `new_chess()`.
- [ ] Expose `from_fen()`.
- [ ] Expose current `GameView`.
- [ ] Expose current `InteractionView`.
- [ ] Expose choice resolution.
- [ ] Expose transition/change results.
- [ ] Expose optional presentation cues.
- [ ] Expose undo.
- [ ] Expose redo.
- [ ] Expose FEN serialization.
- [ ] Expose compact history/move-log view.
- [ ] Keep Rust internals out of frontend DTO contracts.
- [ ] Keep calls coarse-grained across the JS/WASM boundary.

## Phase 12 — Svelte game-model migration

- [ ] Wire the WASM module into the Svelte/Vite application.
- [ ] Make `GameHandle` the authoritative local game state.
- [ ] Remove gameplay use of `src/lib/pieces/*.ts`.
- [ ] Remove TypeScript `BoardPieces` authority.
- [ ] Remove TypeScript FEN parser/authority.
- [ ] Preserve existing SVG piece assets.
- [ ] Adapt `Board.svelte` to `GameView`.
- [ ] Adapt piece rendering to entity/presentation DTOs.
- [ ] Render Rust-provided legal selections.
- [ ] Send selected `ChoiceId`/input back to WASM.
- [ ] Ensure Svelte contains no duplicated legality checks.

## Phase 13 — Interactive local chess UI

- [ ] Select a controllable piece/entity.
- [ ] Highlight legal quiet destinations.
- [ ] Highlight legal captures distinctly.
- [ ] Clear/recompute selections after state changes.
- [ ] Animate ordinary movement from structural deltas.
- [ ] Animate captures/removal from structural deltas.
- [ ] Animate king and rook movement for castling.
- [ ] Display promotion chooser from Rust-provided options.
- [ ] Show active side/player.
- [ ] Show check status.
- [ ] Show checkmate/stalemate/draw outcome.
- [ ] Add undo control.
- [ ] Add redo control.
- [ ] Add reset-to-start control.
- [ ] Add FEN input/load development control.
- [ ] Add current FEN display/copyable field.
- [ ] Add basic move/history display.
- [ ] Verify a complete local two-player chess game can be played without reloads.

## Phase 14 — Correctness suite

### Perft

- [ ] Initial position depth 1 = 20.
- [ ] Initial position depth 2 = 400.
- [ ] Initial position depth 3 = 8,902.
- [ ] Initial position depth 4 = 197,281.
- [ ] Add known castling-heavy perft position(s).
- [ ] Add known en-passant/check interaction perft position(s).
- [ ] Add known promotion perft position(s).

### Regressions

- [ ] En passant cannot persist for an extra turn.
- [ ] En passant can expose a rook/bishop/queen line and become illegal.
- [ ] Castling cannot pass through check.
- [ ] Castling cannot leave/enter check.
- [ ] Returned rook does not recover castling eligibility.
- [ ] Returned king does not recover castling eligibility.
- [ ] Pinned piece legal actions are filtered.
- [ ] Double-check responses are correct.
- [ ] All four promotion types work.
- [ ] Stalemate examples are correct.
- [ ] Checkmate examples are correct.
- [ ] Threefold/fivefold repetition behavior is correct.
- [ ] Fifty/seventy-five move behavior is correct.
- [ ] Dead-position examples are correct.
- [ ] Undo/redo restores all gameplay-relevant state/history.
- [ ] FEN import/export keeps continuation semantics correct.

## Phase 15 — SAN and PGN

- [ ] Generate SAN for ordinary moves.
- [ ] Generate SAN captures.
- [ ] Generate SAN disambiguation.
- [ ] Generate SAN castling.
- [ ] Generate SAN promotions.
- [ ] Generate SAN check/checkmate suffixes.
- [ ] Export PGN from game history.
- [ ] Import standard PGN main line.
- [ ] Test PGN/FEN interoperability where applicable.

## Phase 16 — Architecture proof

Create internal/test-only non-chess rules. Do not expose them as a product mode yet.

- [ ] Register a test custom entity type without editing `glorichess-core`.
- [ ] Give the entity custom state such as HP/mana.
- [ ] Expose an explicit named ability choice.
- [ ] Mutate another entity's custom state.
- [ ] Remove another entity based on resulting custom state.
- [ ] Perform move + ability in two steps during one turn.
- [ ] Force an additional continuation step.
- [ ] Read historical state from the rule.
- [ ] Restore/copy selected data from an older state in a test.
- [ ] Create a test game with at least three `PlayerId`s.
- [ ] Demonstrate team membership independent of player identity.
- [ ] Change `controller` without changing `owner`.
- [ ] Emit an optional semantic presentation cue.
- [ ] Confirm all of the above required no new chess-specific or mechanic-specific core enum variants.

## Final acceptance

- [ ] Standard chess is fully playable locally through Svelte + Rust/WASM.
- [ ] Rust is the sole authority for game rules and state transitions.
- [ ] All standard legal-move edge cases are covered.
- [ ] All required terminal/draw conditions are covered.
- [ ] En passant uses pawn-observable history in normal play.
- [ ] Castling uses king/rook state in normal play.
- [ ] History supports rule queries and undo/redo.
- [ ] Generic turns support multiple sequential steps.
- [ ] Frontend animations are driven by state changes/presentation metadata rather than duplicated game logic.
- [ ] Perft/regression suites pass.
- [ ] Architecture-proof tests pass.
- [ ] No AI/search engine has been added yet.
- [ ] No multiplayer/server implementation has been added yet.
- [ ] No user-facing dynamic-piece DSL/runtime has been added yet.
