# Agent Handoff (Grug/Dango Monorepo)

This document is a generic, evergreen guide for new agents working in this repository. Feature-specific deep dives are documented as playbooks and linked at the end.

## Repository Overview

- Rust + TypeScript monorepo: smart contracts (Dango), node (Grug), indexers, SDK, and UI.
- Primary Rust workspace in `Cargo.toml`; CLI of interest is `dango/cli`.
- Config files for running nodes live under:
  - `localdango/configs/dango/config/app.toml`
  - `deploy/roles/full-app/templates/config/dango/app.toml`

## Conventions

- Rust 2024 edition; keep changes minimal and focused; respect existing style.
- Config changes pattern:
  1) Add serde structs/fields in the relevant `config.rs`.
  2) Update both config templates (local + deploy).
  3) Parse with `config_parser::parse_config` in the target binary.
- Feature flags commonly gate tracing/metrics.

## Cargo Features

When adding a new feature behind a cargo feature flag, **always enable it by
default** unless explicitly asked otherwise. Features should be opt-out, not
opt-in. This prevents the common bug where a feature works when tested in
isolation but isn't compiled into the main binary.

## Workspace Dependencies

**Always add new third-party crates to `[workspace.dependencies]` in the
root `Cargo.toml`**, then reference them with `{ workspace = true }` in
each crate's `Cargo.toml`. Never add a version directly in a crate's
`Cargo.toml` — centralising versions in the workspace avoids duplicate
versions in the lock file and makes upgrades easier.

When adding or upgrading dependencies, prefer the **latest stable crates.io
version** whenever possible (unless there is a concrete compatibility or MSRV
constraint). Before adding any new crate, check crates.io first and pin the
current latest stable release in `[workspace.dependencies]`.

```toml
# Root Cargo.toml
[workspace.dependencies]
some-crate = "1.2"

# dango/cli/Cargo.toml
[dependencies]
some-crate = { workspace = true }
```

## Rust Style and Idioms

Write idiomatic, Rustacean code. Prioritize clarity, modularity, and
zero-cost abstractions.

### Traits and generics

- Always use traits to define behaviour boundaries — this allows alternative
  implementations (e.g. swapping MCP transports, storage backends, provider
  SDKs) and makes testing with mocks straightforward.
- Prefer generic parameters (`fn foo<T: MyTrait>(t: T)`) for hot paths where
  monomorphization matters. Use `dyn Trait` (behind `Arc` / `Box`) when you
  need heterogeneous collections or the concrete type isn't known until
  runtime.
- Derive `Default` on structs whenever all fields have sensible defaults — it
  pairs well with struct update syntax and `unwrap_or_default()`.

### Typed data over loose JSON

Use concrete Rust types (`struct`, `enum`) instead of `serde_json::Value`
wherever the shape is known. This gives compile-time guarantees, better
documentation, and avoids stringly-typed field access. Reserve
`serde_json::Value` for truly dynamic / schema-less data.

### Leverage the type system

**Always use types for comparisons — never convert to strings.** The Rust
type system is your best tool for correctness; use it everywhere:

Benefits of type-based matching:
- **Exhaustiveness checking**: compiler warns if you miss a variant
- **Refactoring safety**: renaming a variant updates all match arms
- **No typos**: `Database::Rdis` won't compile, `"rdis"` will
- **IDE support**: autocomplete, go-to-definition, find references

Only convert to strings at boundaries: serialization, database storage,
logging, or display. Keep the core logic type-safe.

### Type conversions

- Avoid manual one-off conversion functions and ad-hoc `match` blocks sprinkled
  through business logic when converting between types.
- Prefer trait-based conversions (`From` / `Into` / `TryFrom` / `TryInto`) or a
  dedicated local conversion trait when orphan rules prevent a direct impl.
- Always prefer typed structs/enums and serde (de)serialization over raw
  `serde_json::Value` access in production code.
- Treat untyped JSON maps as test-only scaffolding unless there is a strict
  boundary requirement (external RPC/tool contract, dynamic schema).
- If trait-based conversion or typed serde mapping is truly not feasible for a
  specific case, stop and ask for user approval before adding a manual
  conversion path.

### Concurrency

- Always prefer streaming over non-streaming API calls when possible.
  Streaming provides a better, friendlier user experience by showing
  responses as they arrive.
- Run independent async work concurrently with `tokio::join!`,
  `futures::join_all`, or `FuturesUnordered` instead of sequential `.await`
  loops. Sequential awaits are fine when each step depends on the previous
  result.
- Never use `block_on` or any blocking call inside an async context (see
  "Async all the way down" below).
- **Code smell (forbidden): `Mutex<()>` / `Arc<Mutex<()>>` as a lock token.**
  The mutex must guard the actual state/resource being synchronized (e.g. a
  `struct` containing the config/file path/cache), not unit `()` sentinels.
  This keeps locking intent explicit and avoids lock/data drift over time.

### Error handling

- Use `anyhow::Result` for application-level errors and `thiserror` for
  library-level errors that callers need to match on.
- Propagate errors with `?`; avoid `.unwrap()` outside of tests.

### Date, time, and crate reuse

Prefer short, readable code that leverages existing workspace crates over
hand-rolled arithmetic. For date/time specifically, use the **`time`** crate
(already a workspace dependency) instead of manual epoch conversions,
calendar math, or magic constants like `86400`:

```rust
// Good — concise, self-documenting
time::Duration::days(30).unsigned_abs()
time::OffsetDateTime::now_utc().date()

// Bad — manual arithmetic, magic constants
days * 86400
days * 24 * 60 * 60
```

This principle applies broadly: if a crate in the workspace already
provides a clear one-liner, use it rather than reimplementing the logic.

### General style

- Prefer iterators and combinators (`.map()`, `.filter()`, `.collect()`)
  over manual loops when they express intent more clearly.
- Use `Cow<'_, str>` when a function may or may not need to allocate.
- Keep public API surfaces small: expose only what downstream crates need
  via `pub use` re-exports in `lib.rs`.
- Prefer `#[must_use]` on functions whose return value should not be
  silently ignored.

### Tracing and Metrics

**All crates must include tracing and metrics instrumentation.** This is
critical for telemetry, debugging, and production observability.

- Add `tracing` feature to crate's `Cargo.toml` and gate instrumentation
  with `#[cfg(feature = "tracing")]`
- Add `metrics` feature and gate counters/gauges/histograms with
  `#[cfg(feature = "metrics")]`
- Use `tracing::instrument` on async functions for automatic span creation
- Record metrics at key points: operation counts, durations, errors, and
  resource usage

```rust
#[cfg(feature = "tracing")]
use tracing::{debug, instrument, warn};

#[cfg(feature = "metrics")]
use moltis_metrics::{counter, histogram, labels};

#[cfg_attr(feature = "tracing", instrument(skip(self)))]
pub async fn process_request(&self, req: Request) -> Result<Response> {
    #[cfg(feature = "metrics")]
    let start = std::time::Instant::now();

    // ... do work ...

    #[cfg(feature = "metrics")]
    {
        counter!("my_crate_requests_total").increment(1);
        histogram!("my_crate_request_duration_seconds")
            .record(start.elapsed().as_secs_f64());
    }

    Ok(response)
}

Telemetry (OTLP + Sentry, graceful shutdown): `docs/telemetry.md`
```

### Commit Messages

- Follow conventional commit format: `type(scope): description`
- Separate subject from body with a blank line
- Limit subject line to 50 characters
- Capitalize the subject line and do not end with a period
- Use imperative mood (e.g., "Add feature" not "Added feature")
- Wrap body at 72 characters
- Explain what and why in the body, not how
- Reference issues with "Closes #123" or "Fixes #456" in footer if applicable
- Include app and LLM user as co-authors only if you produced the code being committed
- Use `users.noreply.github.com` as the domain for co-author emails
- Run `just fmt` and `just lint` before committing to ensure code formatting and linting

## Versions & Compatibility (Examples that matter)

- OpenTelemetry: `opentelemetry = 0.31`, `opentelemetry_sdk = 0.31`, `opentelemetry-otlp = 0.31` (`grpc-tonic`, `trace`), `tracing-opentelemetry = 0.32`.
- Sentry: `sentry = 0.38`.
- Tokio: `1.x`.
If adding/upgrading these, align versions across crates to avoid trait/type mismatches.

## Config Locations

- Local user config: `~/.dango/config/app.toml`.
- Templates to keep in sync:
  - `localdango/configs/dango/config/app.toml`
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

## Useful Commands

- Format: `cargo +nightly fmt --all`
- Lint all: `cargo clippy --bins --tests --benches --examples --all-features --all-targets -- -D warnings`
- Typical `just` recipes: `just fmt`, `just lint`, `just test`

---

If you need to add Jaeger protocol support, prefer OTLP path (Tempo supports OTLP). If still needed, add a config flag to select Jaeger exporter and wire the `opentelemetry-jaeger` crate accordingly, keeping versions aligned.
