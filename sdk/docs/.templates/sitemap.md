# SDK Docs Sitemap

The Vocs site has **three top-level sections** in the sidebar — one per language. Each section has the **same IA shape** with language-specific contents.

## Shared shape

```
[Language Root]
├── Getting Started
│   ├── Installation
│   ├── First Call         (5-minute hello world)
│   └── Project Setup      (networks, auth keys, env vars)
├── Concepts                                    (free-form guides)
│   ├── Clients            (kinds of clients, when to use which)
│   ├── Signers & Authentication
│   ├── Transactions       (signing, broadcasting, status)
│   ├── Subscriptions      (WebSocket model)
│   ├── Encoding & Types   (base units, brands, Decimal/bigint)
│   ├── Error Handling
│   └── Rate Limits & Quotas
├── API Reference                               (rigid templates)
│   ├── Clients            (Client template per entry)
│   ├── Actions            (Action template per fn, grouped by domain)
│   │   ├── App
│   │   ├── DEX
│   │   ├── Perps
│   │   ├── Account Factory
│   │   ├── Gateway
│   │   ├── Indexer
│   │   ├── Oracle
│   │   └── Hyperlane
│   ├── Types              (Type template per major type)
│   └── Errors             (one page per error class)
└── Migration / Compatibility                   (Python only)
    └── Hyperliquid SDK migration
```

## Per-language adaptations

### TypeScript

- **Clients**: `createPublicClient`, `createSignerClient`, `createBaseClient`
- **Actions**: full 8-domain split as above. Each action gets its own Action page.
- **Packages note**: a single intro page `concepts/packages.mdx` explains the split between `@left-curve/sdk`, `@left-curve/crypto`, `@left-curve/encoding`, `@left-curve/types`, `@left-curve/utils`, `@left-curve/config`.

### Python

- **Clients section → "Classes"**: `Exchange`, `Info`, `Subscription` (or `WebSocketManager`)
- **Actions → methods on classes**: pages are organized under `Exchange/` and `Info/` directories
- **Migration section**: dedicated Hyperliquid-compat layer pages

### Rust

- **Clients section**: `Client`, `Signer`, `Keystore`
- **Actions → Client methods**: each public method on `Client` gets an Action page
- **Subscriptions**: dedicated section under API Reference (the `Subscription` type + per-subscription helpers)

## What goes where — disambiguation rules

| If the symbol is... | Page kind | Section |
|---------------------|-----------|---------|
| A free function or method that does one thing (`getBalance`, `transfer`) | Action | Actions |
| A constructor / factory for a long-lived stateful object (`createPublicClient`, `Exchange()`) | Client | Clients |
| A data shape (record / struct / TypedDict / class without behavior) | Type | Types |
| An error class | Error | Errors |
| Conceptual material (transactions, signing, rate limits) | Concept | Concepts |

## Cross-language linking policy

- Inside a Reference page: **only intra-language links**
- Inside a Concept page: cross-language links are **allowed** (e.g., the Rate Limits concept can link to all three SDKs' rate-limit helpers)
- The site has **no** "cross-language Reference" — readers stay within one language while looking up APIs
