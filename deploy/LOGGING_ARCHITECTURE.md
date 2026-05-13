# Logging Architecture, Post-Promtail

## Goal

Replace Promtail with a single per-host log collector that:

- sends logs to Loki for Grafana queries
- archives the same logs to Backblaze B2 over the S3-compatible API
- stores archives under daily prefixes
- avoids running two different agents against the same Docker logs

## Why change this

The current deploy uses Promtail as the log shipper:

- [deploy/roles/promtail/templates/promtail-config.yml](./roles/promtail/templates/promtail-config.yml)
- [deploy/roles/loki/templates/loki-config.yml](./roles/loki/templates/loki-config.yml)

That works for Loki, but not for the new requirement:

- Promtail is effectively a Loki shipper, not a general fan-out log router.
- Promtail has also been deprecated by Grafana and reached EOL on March 2, 2026.
- Loki itself should not be treated as the raw-log backup pipeline. Loki storage is optimized for Loki internals, not clean archive layout.

## Recommended architecture

Use **Vector** as the single log collector on each app host.

Per host:

- `Vector`
  - source: Docker container logs
  - source: host logs if we still want `/var/log/*`
  - sink 1: `Loki`
  - sink 2: `Backblaze B2` via the S3-compatible API

Central:

- `Loki` remains the query backend for Grafana
- `Grafana` continues to query Loki
- `Backblaze B2` becomes the raw archive / backup target

```mermaid
flowchart LR
    subgraph Host["Per server"]
        Docker[Docker containers]
        Syslog[/var/log]
        Vector[Vector]
    end

    Loki[Loki on ovh3]
    Grafana[Grafana]
    B2[Backblaze B2 bucket]

    Docker --> Vector
    Syslog --> Vector
    Vector --> Loki
    Vector --> B2
    Grafana --> Loki
```

## Why Vector

Vector is the suggested choice here because it cleanly supports:

- one collector, multiple sinks
- Loki sink for Grafana search
- S3-compatible object storage sink for B2
- date-based object prefixes
- buffering, retries, and checkpointing in one place

This is cleaner than:

- keeping Promtail and bolting on a second agent
- trying to back up Loki's internal storage directory
- trying to make Loki produce a human-shaped archive

Grafana Alloy is also a valid replacement for Promtail in general, but for this specific requirement, "Loki plus S3-style daily archives", Vector is the simpler fit.

## Archive layout

The B2 archive should be raw log objects, not Loki chunks.

Suggested prefix shape:

```text
dango-logs/
  host=<hostname>/
    date=YYYY-MM-DD/
      service=<service_name>/
        <timestamp>-<uuid>.log.gz
```

Practical `key_prefix` examples:

```text
dango-logs/host={{ host }}/date=%F/service={{ service_name }}/
```

or, if we want deployment separation too:

```text
dango-logs/environment={{ environment }}/deployment={{ deployment }}/date=%F/service={{ service_name }}/
```

The important bit is the `date=%F/` partition so a single day is easy to inspect or expire.

## Data shape in B2

Archive objects should be newline-delimited JSON, ideally gzipped.

Each event should preserve useful routing metadata, for example:

- `timestamp`
- `message`
- `host`
- `container_name`
- `service_name`
- `environment`
- `deployment`
- `chain_id`
- `level`
- `target`

That gives us a boring, durable archive format. Boring is good here.

## Operational rules

1. Do not run Promtail and Vector against the same Docker log files long term.

That usually turns into duplicate ingestion, weird checkpoint behavior, and arguments with ghosts.

2. Do not treat `/tmp/loki` as the backup source.

Loki storage contains Loki's chunk/index layout, compaction behavior, and retention semantics. It is the wrong backup abstraction if the goal is "keep a second copy of raw logs".

3. Keep Loki and B2 as separate concerns.

- Loki is for querying recent logs in Grafana
- B2 is for durable archive and recovery

4. Expect at-least-once delivery, not exactly-once.

The archive may contain occasional duplicates during retries or restarts. That is acceptable for a backup stream.

5. Put lifecycle rules on the B2 bucket.

Decide retention explicitly, for example:

- keep 30 or 90 days hot in Loki
- keep 180 or 365 days in B2

## Migration plan

### Phase 1, add Vector in parallel

- create a new `vector` Ansible role
- deploy Vector to one non-critical host first
- point Vector at Loki and B2
- verify log volume, labels, and object layout

### Phase 2, verify parity

- compare Loki queries before and after on the same host
- verify B2 daily prefixes are created correctly
- verify gzip objects are readable and contain expected metadata

### Phase 3, replace Promtail

- disable Promtail on the test host
- confirm Loki still receives the same logs through Vector
- roll out Vector to the remaining hosts
- remove the Promtail role from regular deploy paths

### Phase 4, clean up docs and dashboards

- update [deploy/README.md](./README.md) to replace Promtail with Vector
- update any operational notes or host setup docs
- keep Loki labels compatible where possible so Grafana queries do not break

## Config direction

The future config should look roughly like this:

- source:
  - Docker logs
  - optional file source for system logs
- transform:
  - parse JSON where useful
  - attach labels and archive fields consistently
- sink:
  - Loki
  - AWS S3-compatible sink pointed at Backblaze B2

Backblaze notes:

- use the B2 S3-compatible endpoint
- use path-style access if needed by the client config
- keep TLS enabled
- use the existing deploy pattern for S3 credentials where possible

## What not to do

- Do not add a cron job that copies Loki's data directory to B2 and call it solved.
- Do not keep Promtail as the long-term primary collector for new work.
- Do not archive only "pretty" filtered logs and throw away raw context.
- Do not split log collection across multiple agents unless there is a very specific reason.

## Useful references

- Promtail deprecation notice:
  - https://grafana.com/docs/loki/latest/send-data/promtail/installation/
- Vector S3 sink:
  - https://vector.dev/docs/reference/configuration/sinks/aws_s3/
- Vector template syntax for date-based prefixes:
  - https://vector.dev/docs/reference/configuration/template-syntax/
- Backblaze B2 S3-compatible API:
  - https://www.backblaze.com/docs/en/cloud-storage-call-the-s3-compatible-api

## Decision

When this is implemented later, the intended end state is:

- **Vector on each host**
- **Loki for search**
- **Backblaze B2 for daily raw-log archives**
- **Promtail removed**
