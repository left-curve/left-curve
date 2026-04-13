# Security Audit Guide

This guide documents the architecture of **Grug** (the blockchain state machine) and
**Dango** (the smart contract system built on Grug), targeting security auditors with
blockchain and DeFi experience. It covers:

1. **[Grug Architecture](1-grug-architecture.md)** -- Database, Jellyfish Merkle Tree,
   storage layer, the App/ABCI interface, virtual machines, and gas metering.
2. **[Smart Contract Semantics](2-contract-semantics.md)** -- Entry points, context types,
   message passing, storage abstractions, authentication model, and the testing framework.
3. **[Dango Contract System](3-dango-contracts.md)** -- Each smart contract (bank, accounts,
   oracle, DEX, perps, taxman, gateway, etc.), their state layout, access control, and
   inter-contract interactions.
4. **[Indexer & Node](4-indexer-and-node.md)** -- The indexer pipeline, SQL schema,
   GraphQL API, and the CLI that wires everything together.

## Repository layout

| Directory | Contents |
|-----------|----------|
| `grug/` | State machine: `app`, `db/disk`, `db/memory`, `vm/rust`, `vm/wasm`, `types`, `storage`, `jellyfish-merkle`, `ffi`, `macros`, `crypto`, `math`, `std`, `testing` |
| `dango/` | Smart contracts: `bank`, `account`, `account-factory`, `auth`, `oracle`, `dex`, `perps`, `taxman`, `gateway`, `vesting`, `warp`, `upgrade`, `types`, `cli` |
| `indexer/` | Indexing: `hooked`, `sql`, `sql-migration`, `cache`, `httpd`, `client` |
| `sdk/`, `ui/` | TypeScript frontend (out of scope for this guide) |
| `deploy/` | Ansible playbooks (out of scope) |

## Trust model at a glance

```text
┌──────────────────────────────────────────────────────────────┐
│  TRUSTED: Node Binary                                        │
│   grug/app (ABCI + state transitions)                        │
│   grug/db  (RocksDB persistence)                             │
│   grug/vm/rust (native contract execution, no sandbox)       │
│   grug/jellyfish-merkle (state commitment)                   │
│   dango/* system contracts (bank, taxman, accounts, etc.)    │
│   indexer/* (read-only; cannot affect consensus)              │
└────────────────────── ▼ ─────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────┐
│  UNTRUSTED: Third-Party WASM Contracts                       │
│   Executed inside grug/vm/wasm (Wasmer sandbox)              │
│   All storage access namespaced via StorageProvider           │
│   All operations metered via gas tracker                      │
│   Host function calls go through Gatekeeper middleware        │
└──────────────────────────────────────────────────────────────┘
```

> **Note:** In the current Dango deployment, all contracts are first-party and executed
> natively via `RustVm`. The `WasmVm` path exists for future third-party contract
> support. Both paths share the same `Vm` trait interface.
