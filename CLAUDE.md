# Grug/Dango Monorepo

Rust + TypeScript monorepo: smart contracts (Dango), node (Grug), indexers, SDK, and UI.

## Build & Test

- Rust Format: `just fmt`
- Rust Lint: `just lint`
- Rust tests: `just test`
- TS lint: `pnpm lint`
- TS tests: `pnpm test`
- E2E: `pnpm test:e2e`
- Dev server: `pnpm dev:portal-web`

## Git Workflow

Conventional commits: `type(scope): description` in imperative mood.
Types: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`.

PR descriptions must include:
- `## Summary` (what changed and why)
- `## Validation` with checkboxes (`### Completed` / `### Remaining`)
- `## Manual QA` (steps or "None")

Never overwrite existing tags — always create a new version.

Cargo.lock sync: run `cargo fetch` after merging main, verify with `cargo fetch --locked`.

When resolving merge conflicts, keep both sides — integrate, don't discard.

## CI Expectations

- `clippy -D warnings` must pass (zero warnings)
- Biome lint must pass for all TS/TSX
- Never weaken test assertions — adapt to new APIs but verify same postconditions
