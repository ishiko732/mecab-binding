# Repository Guidelines

## Project Structure & Module Organization
`src/` contains the Rust native binding. `src/lib.rs` is the N-API entry, while `src/grammar/` implements grammar parsing and matching. `scripts/*.mjs` handle dictionary compilation, grammar processing, and dynamic WASI packaging. `__test__/` holds Vitest integration specs. `sources/` stores raw dictionary assets, `.output/` is local generated state, `npm/` contains per-platform packages, and `examples/vite/` is the browser/WASM demo.

## Build, Test, and Development Commands
Use Node 24 to match CI when local Node versions drift.

- `pnpm install`: install JS dependencies.
- `pnpm build`: build the native addon into `dist/`; `postbuild` also packs `ipadic.data`.
- `pnpm build:wasm`: build the `wasm32-wasip1-threads` target.
- `pnpm build:dict`: compile dictionary assets from `sources/` into `.output/dict`.
- `pnpm test`: run Vitest integration tests. Run `pnpm build` first, or at minimum `pnpm build:dict` when tests need dictionary output.
- `pnpm lint`: run Biome checks.
- `pnpm format`: run Biome, `cargo fmt`, and Taplo formatting.
- `cargo clippy`: extra Rust linting used in CI.

## Coding Style & Naming Conventions
For JS/TS/JSON, follow Biome’s existing style: tabs, single quotes, trailing commas where allowed, and no unnecessary semicolons. For Rust, rely on `cargo fmt`; keep modules in `snake_case` and exported N-API names aligned with the JS surface. Name tests `*.spec.ts`. In `examples/vite/`, React components use `PascalCase`; scripts should stay descriptive and file-based, for example `compile-dict.mjs`.

## Testing Guidelines
Prefer integration tests around `Tagger`, dictionary packing, and `GrammarMatcher` behavior over isolated helpers. Cover success paths, empty matches, and invalid input. When changing grammar rules, dictionary assets, or WASM loading, add or update a spec in `__test__/` and verify with `pnpm test`.

## Commit & Pull Request Guidelines
Recent history mostly uses Conventional Commit prefixes such as `feat:`, `fix:`, and `refactor:`. Keep that pattern and avoid vague subjects like `clear` or `fmt`. PRs should explain the reason for the change, affected targets (native, WASM, or both), and the commands you ran, for example `pnpm test` and `cargo clippy`. Include screenshots only for visible changes under `examples/vite/`.
