# Telemetry Playbook (OTLP + Sentry)

This playbook documents how traces are exported via OpenTelemetry (OTLP) and how Sentry is integrated, including graceful shutdown paths so data is not lost on exit or Ctrl-C.

## Versions
- `opentelemetry = 0.31`
- `opentelemetry_sdk = 0.31`
- `opentelemetry-otlp = 0.31` with features: `grpc-tonic`, `trace`
- `tracing-opentelemetry = 0.32`
- `sentry = 0.38`

Keep these aligned across the workspace to avoid trait/type mismatches.

## Config Schema
- `[trace]` in `app.toml` (local or template):
  - `enabled = true|false`
  - `protocol = "otlp_grpc" | "otlp_http"`
  - `endpoint = "http://host:port[/v1/traces]"`
- Rust types in `dango/cli/src/config.rs`:
  - `TraceConfig { enabled: bool, endpoint: String, protocol: TraceProtocol }`
  - `TraceProtocol = OtlpGrpc | OtlpHttp` (serde snake_case)

## Initialization (dango/cli/src/main.rs)
1) Build `Resource` with service name and attributes (e.g., `chain.id`).
2) Build exporter depending on protocol:
   - gRPC: `SpanExporter::builder().with_tonic().with_export_config(ExportConfig{ protocol: Protocol::Grpc, endpoint: Some(..) }).build()?`
   - HTTP: `SpanExporter::builder().with_http().with_export_config(ExportConfig{ protocol: Protocol::HttpBinary, endpoint: Some(..) }).build()?`
3) Build provider/tracer:
   - `SdkTracerProvider::builder().with_batch_exporter(exporter).with_resource(resource).build()`
   - `let tracer = provider.tracer("dango");`
4) Register provider via `telemetry::set_provider(provider)`; compose `tracing_opentelemetry` layer with the tracer.
5) Sentry: initialize with `sentry::init(..)` and include `sentry_tracing` layer when enabled.

## Graceful Shutdown
- Module: `dango/cli/src/telemetry.rs`
  - Stores provider in `OnceLock`; `shutdown()` calls `provider.shutdown()`.
  - `shutdown_sentry()` fetches current client and `client.close(None)`.
- Main: after running the selected command, call both shutdown helpers.
- Signals: `dango/cli/src/start.rs` listens for SIGINT/SIGTERM and calls both shutdown helpers before return.

## Example app.toml
```toml
[trace]
enabled = true
# OTLP over gRPC (4317)
protocol = "otlp_grpc"
endpoint = "http://collector:4317"

# Or OTLP over HTTP (4318)
# protocol = "otlp_http"
# endpoint = "http://collector:4318/v1/traces"
```

## Notes
- Default `service.name` is "dango"; consider making it configurable per binary.
- For HTTP, ensure your collector path matches (often `/v1/traces`).
- Jaeger/Tempo both accept OTLP; prefer OTLP over vendor-specific exporters.
