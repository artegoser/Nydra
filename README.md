# Nydra

Nydra is a local-first programmable game runtime with a Svelte frontend. Its generic Rust core hosts composable entity, ability, game, interaction, history, and outcome rules; standard chess is the first concrete ruleset and compiles to WebAssembly for the browser.

## Repository layout

- `crates/nydra-core` — generic state, history, turns, interaction, and runtime primitives.
- `crates/nydra-chess` — standard chess rules built on the generic core.
- `crates/nydra-examples` — internal checkers, Go, and Rift architecture proofs; not product modes.
- `crates/nydra-wasm` — browser bindings for the Rust runtime.
- `src` — SvelteKit presentation and input layer.
- `docs/CORE_IMPLEMENTATION_PLAN.md` — architecture and implementation plan.
- `docs/CORE_IMPLEMENTATION_CHECKLIST.md` — implementation checklist.
- `docs/LICHESS_BOARD_UX_SPEC.md` — Lichess board interaction and visual parity specification.
- `docs/GENERIC_ACTION_NOTATION.md` — ruleset-agnostic action recording and deterministic replay specification.
- `docs/GENERIC_OUTCOME_RULES.md` — ruleset-wide terminal outcome contract and precedence rules.
- `docs/GENERIC_GAME_RULES.md` — composition contract for entity-local and ruleset-wide mechanics.
- `docs/EXAMPLE_RULESETS.md` — Phase 16 non-chess architecture proofs.

## Requirements

- Node.js 22+
- pnpm
- Rust 1.80+
- `wasm-pack` for browser WASM builds

Install `wasm-pack` if necessary:

```bash
cargo install wasm-pack
```

## Development

Install frontend dependencies:

```bash
pnpm install --frozen-lockfile
```

Run the local Svelte application backed by the Rust/WASM chess runtime:

```bash
pnpm dev
```

`pnpm dev`, `pnpm build`, and `pnpm check` build the browser WASM package first. You can also build it explicitly:

```bash
pnpm wasm:build
```

The generated bindings are written to `src/lib/wasm/pkg/` and are not committed.

## Verification

Rust:

```bash
cargo test --workspace
cargo check --workspace
cargo clippy --workspace --all-targets --all-features
```

The normal workspace suite keeps expensive depth-4 perft cases ignored so the development loop stays fast. Run the slow correctness gate explicitly when changing move generation, history semantics, or special-move execution:

```bash
cargo test --release -p nydra-chess perft::tests:: -- --ignored
```

Frontend:

```bash
pnpm check
pnpm lint
pnpm build
```

## Development plan

See:

- [`docs/CORE_IMPLEMENTATION_PLAN.md`](docs/CORE_IMPLEMENTATION_PLAN.md)
- [`docs/CORE_IMPLEMENTATION_CHECKLIST.md`](docs/CORE_IMPLEMENTATION_CHECKLIST.md)
- [`docs/GENERIC_ACTION_NOTATION.md`](docs/GENERIC_ACTION_NOTATION.md)
- [`docs/GENERIC_OUTCOME_RULES.md`](docs/GENERIC_OUTCOME_RULES.md)
- [`docs/GENERIC_GAME_RULES.md`](docs/GENERIC_GAME_RULES.md)
- [`docs/EXAMPLE_RULESETS.md`](docs/EXAMPLE_RULESETS.md)
