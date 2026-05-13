# SDK Docs Style Guide

This is the **canonical spec** for every `.mdx` page in `sdk/docs/`. All drafter and reviewer agents must follow it. Templates in this directory implement these rules — when in doubt, follow the template literally.

## Scope

The Vocs site at `sdk/docs/` documents three SDKs: **TypeScript, Python, Rust**. Each has its own top-level section in the sidebar. SDKs are **not** parity — Python mirrors Hyperliquid's `Exchange`/`Info` shape, Rust is a thin GraphQL client, TypeScript is viem-style.

## Page kinds

Every page is exactly one of:

| Kind | Template | When to use |
|------|----------|-------------|
| **Action** | `action.mdx` | One exported function/method (e.g., `getBalance`, `submitOrder`) |
| **Type** | `type.mdx` | One exported data type (e.g., `Coin`, `Order`, `TxStatus`) |
| **Client** | `client.mdx` | A client/class entry point (e.g., `createPublicClient`, `Exchange`) |
| **Concept** | `concept.mdx` | Free-form narrative guide (no rigid structure) |

If a page doesn't fit, **discuss first** — don't invent a new kind.

## Voice and tone

- **Imperative and direct.** "Pass the address." Not "You can pass the address."
- **Second-person sparingly.** Used only in Concept pages. Reference pages have no "you."
- **No hedging.** No "you might want to" / "consider doing." Say what to do.
- **No marketing.** Don't sell — describe. The reader is on the page because they already chose the SDK.
- **No "easy/simple/just".** The reader is debugging at 2am.
- **One-line description per page** opens every Reference page. Plain English, one sentence, no fluff.

## What's required vs optional

For **Action / Type / Client** pages:

| Section | Required? | When to omit |
|---------|-----------|--------------|
| H1 + one-line description | ✅ always | never |
| Signature (Action/Type/Client) | ✅ always | never |
| Example | ✅ always | never |
| Parameters / Fields | ✅ if any | functions with no params, types with no fields |
| Returns | ✅ for Action | non-Action pages |
| Notes | ❌ optional | when there's nothing surprising |
| See also | ✅ always | place at end |

For **Concept** pages: free-form. Required header: one-line "What this teaches you." Required footer: "Next: [link to next concept]."

## Examples

- **Inline only**, in fenced code blocks. No external example files.
- Examples must be **realistic** — use plausible addresses, denoms, amounts. Not `foo`/`bar`.
- Examples must be **runnable in principle** — every import must exist, every call must match the current signature. Drafters: read source before writing imports.
- Examples must be **minimal** — one purpose per block. Multi-step usage goes in Concept pages, not Action pages.
- Show the **idiomatic style** for the SDK. For TS: prefer the extended-action style (`client.action()`) for primary example; tabs can show the tree-shakable style (`action(client, ...)`) as alternate.

## Code blocks

- Always fence with the right language: `ts`, `tsx`, `py`, `rust`, `bash`, `json`
- For TS, the implementer agent may add `twoslash` — if active, types must check
- No leading `>` prompts in bash blocks unless showing real shell output
- Imports go at the top of every example, never omitted with `// imports omitted`

## Linking

- Inter-page links use **relative paths within the language section**, e.g. `[getBalances](./getBalances)`
- Cross-language links — avoid in Reference pages; use cross-language only in shared concept pages
- Source links — auto-injected by Vocs from frontmatter; do not hand-write
- External links — only when truly necessary (e.g., to a spec or a third-party project)

## Naming

- Page filename = the symbol it documents, in the SDK's native casing:
  - TS: `getBalance.mdx`, `createPublicClient.mdx`
  - Python: `get_balance.mdx`, `Exchange.mdx`
  - Rust: `get_balance.mdx`, `Client.mdx`
- Concept pages: `kebab-case.mdx` (e.g., `rate-limits.mdx`, `error-handling.mdx`)
- Directory names: lowercase, plural for collections (`actions/`, `types/`)

## Status callouts (required where applicable)

### DEX currently disabled

Every page documenting a DEX action (e.g., `swapExactAmountIn`, `provideLiquidity`, `submitOrder`, etc., across TS/Python/Rust) **must** open with a warning callout immediately under the H1 description:

```mdx
:::warning[DEX currently disabled]
The Dango DEX is currently disabled. Calls described on this page will not execute on the live network until the DEX is enabled.
:::
```

Scope: any action under `actions/dex/`, `actions/perps/` (perps trade through the DEX), or Python/Rust pages that document equivalent functionality.

This callout sits above all other content. Reviewers must verify it's present on every applicable page.

## What NOT to include

- ❌ "Why we built this" / marketing
- ❌ Future features / planned work
- ❌ Internal architecture details unless directly user-facing
- ❌ Deprecation notices for things never released
- ❌ "Easy"/"Simple"/"Just"
- ❌ Apologetic language ("unfortunately", "sadly")
- ❌ Emojis unless explicitly approved
- ❌ Generated TypeDoc/Sphinx/rustdoc HTML — Vocs is the only canonical surface

## Verification responsibility

Drafters must verify before writing each page:
1. Function/type still exists in source
2. Signature matches what you're writing
3. Example imports resolve
4. Example calls match current parameter shape

Reviewers verify the same plus:
1. Cross-page references resolve
2. Style guide adherence
3. Conceptual accuracy
