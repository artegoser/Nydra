# GloriChess

GloriChess is a local-first board-game runtime and Svelte UI. The current implementation milestone is a generic Rust core that will host complete standard chess rules and compile to WebAssembly for the browser.

## Repository layout

- `crates/glorichess-core` — generic state, history, turns, interaction, and runtime primitives.
- `crates/glorichess-chess` — standard chess rules built on the generic core.
- `crates/glorichess-wasm` — browser bindings for the Rust runtime.
- `src` — SvelteKit presentation and input layer.
- `docs/CORE_IMPLEMENTATION_PLAN.md` — architecture and implementation plan.
- `docs/CORE_IMPLEMENTATION_CHECKLIST.md` — implementation checklist.

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
