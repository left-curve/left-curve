# Agent Handoff (Grug/Dango Monorepo)

This document is a generic, evergreen guide for new agents working in this repository. Feature-specific deep dives are documented as playbooks and linked at the end.

## Repository Overview
- Rust + TypeScript monorepo: smart contracts (Dango), node (Grug), indexers, SDK, and UI.
- Primary Rust workspace in `Cargo.toml`; CLI of interest is `dango/cli`.
- Config files for running nodes live under:
  - `networks/localdango/configs/dango/config/app.toml`
  - `deploy/roles/full-app/templates/config/dango/app.toml`

## Conventions
- Rust 2021 edition; keep changes minimal and focused; respect existing style.
- Config changes pattern:
  1) Add serde structs/fields in the relevant `config.rs`.
  2) Update both config templates (local + deploy).
  3) Parse with `config_parser::parse_config` in the target binary.
- Feature flags commonly gate tracing/metrics.

### Commit Messages
- Follow conventional commit format: `type(scope): description`
- Separate subject from body with a blank line
- Limit subject line to 50 characters
- Capitalize the subject line and do not end with a period
- Use imperative mood (e.g., "Add feature" not "Added feature")
- Wrap body at 72 characters
- Explain what and why in the body, not how
- Reference issues with "Closes #123" or "Fixes #456" in footer if applicable

## Versions & Compatibility (Examples that matter)
- OpenTelemetry: `opentelemetry = 0.31`, `opentelemetry_sdk = 0.31`, `opentelemetry-otlp = 0.31` (`grpc-tonic`, `trace`), `tracing-opentelemetry = 0.32`.
- Sentry: `sentry = 0.38`.
- Tokio: `1.x`.
If adding/upgrading these, align versions across crates to avoid trait/type mismatches.

## Config Locations
- Local user config: `~/.dango/config/app.toml`.
- Templates to keep in sync:
  - `networks/localdango/configs/dango/config/app.toml`
  - `deploy/roles/full-app/templates/config/dango/app.toml`

## Runtime & Signals
- `dango/cli/src/start.rs` listens for SIGINT/SIGTERM.
- Global resources (e.g., tracer provider, Sentry) should flush at signal and clean exit.
- Shared shutdown helpers live in `dango/cli/src/telemetry.rs`.

## Testing & CI Expectations
- Keep `clippy -D warnings` clean across the workspace.
- Prefer targeted tests; avoid changing unrelated behavior.

## Common Gotchas
- Keep key crate versions aligned (OTEL, tracing, Sentry).
- Avoid reassigning tracing subscribers; compose with `.with(optional_layer)`.
- Config updates must touch both runtime templates.
- When adding new imports (especially under `#[cfg(feature = "...")]`), always place them in the file header's import section, not inline in functions.

## Playbooks
- Telemetry (OTLP + Sentry, graceful shutdown): `docs/telemetry.md`

## Useful Commands
- Format: `cargo +nightly fmt --all`
- Lint all: `cargo clippy --bins --tests --benches --examples --all-features --all-targets -- -D warnings`
- Typical `just` recipes: `just fmt`, `just lint`, `just test`

---
If you need to add Jaeger protocol support, prefer OTLP path (Tempo supports OTLP). If still needed, add a config flag to select Jaeger exporter and wire the `opentelemetry-jaeger` crate accordingly, keeping versions aligned.
