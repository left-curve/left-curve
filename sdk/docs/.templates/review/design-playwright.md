# Design Review (Playwright-driven)

## Setup

- Dev server URL: `http://localhost:5174/` (port 5173 was in use; Vocs auto-selected 5174)
- Browser viewports tested: 1920x1080, 1440x900, 390x844 (iPhone-ish), 320x700 (worst-case)
- Theme modes tested: both light and dark
- Pages visited: `/`, `/typescript/`, `/typescript/getting-started/installation`, `/typescript/getting-started/first-call`, `/typescript/concepts/clients`, `/typescript/concepts/rate-limits`, `/typescript/concepts/transactions`, `/typescript/concepts/subscriptions`, `/typescript/clients/createPublicClient`, `/typescript/actions/app/getBalance`, `/typescript/actions/dex/swapExactAmountIn`, `/typescript/types/Coin`, `/python/migration/hyperliquid`, `/rust/concepts/transactions`
- Console: zero errors or warnings across the visit set (one React DevTools info notice only)

## Screenshots

All saved to `sdk/docs/.templates/review/screenshots/`.

| File | Page / viewport |
|------|-----------------|
| `landing-desktop-light.png` | Landing, 1440 wide, light mode |
| `landing-desktop-dark.png` | Landing, 1440 wide, dark mode |
| `landing-mobile-light.png` | Landing, 390 wide, light mode |
| `ts-root-desktop-light.png` | TS root, 1440 wide, light mode |
| `ts-root-desktop-dark.png` | TS root, 1440 wide, dark mode |
| `ts-root-mobile-320.png` | TS root, 320 wide (worst case) |
| `ts-installation-desktop-dark.png` | TS install, sub-packages line truncates |
| `ts-first-call-desktop-dark.png` | TS first call |
| `ts-concepts-clients-desktop-dark.png` | TS concepts/clients, tree-shakable example pair |
| `ts-rate-limits-desktop-dark.png` | TS rate-limits |
| `ts-transactions-desktop-light.png` | TS transactions, Steps + warning, light mode |
| `ts-transactions-desktop-dark.png` | TS transactions, full page, dark mode |
| `ts-transactions-mobile-light.png` | TS transactions, 390 wide |
| `ts-createPublicClient-desktop-dark.png` | Full-page screenshot of Methods cascade |
| `ts-createPublicClient-methods-viewport-dark.png` | Methods at desktop viewport |
| `ts-createPublicClient-mobile-light.png` | createPublicClient mobile |
| `ts-createPublicClient-mobile-methods.png` | Methods table at mobile width |
| `ts-getBalance-desktop-dark.png` | Action page, dark |
| `ts-swap-desktop-dark.png` / `ts-swap-desktop-light.png` | DEX action with warning callout, both modes |
| `ts-coin-desktop-dark.png` / `ts-coin-desktop-light.png` | Type page |
| `ts-subscriptions-mobile-320.png` | Worst-case mobile, longest inline code |
| `ts-mobile-sidebar-open.png` | Mobile menu open (focus ring visible) |
| `ts-transactions-desktop-1920-light.png` | Very wide viewport, content does not expand |
| `python-migration-desktop-dark.png` | The longest page on the site |
| `python-migration-mobile-dark.png` | Migration before/after import block on mobile |
| `rust-transactions-desktop-dark.png` | Rust transactions, full page |
| `rust-transactions-steps-viewport-dark.png` | Rust transactions mental-model viewport |
| `rust-transactions-stepsbox-viewport-dark.png` | The Steps box up close — no numerals |
| `search-overlay-desktop-dark.png` | Search modal open |

## Top 5 ranked changes

1. **Wrap each H3 inside `<Steps>` with Vocs' `<Step title="...">` (singular).** Vocs' `Step` component renders the numbered marker via `counter-increment: step`. Today both TS and Rust transactions pages wrap raw `### Heading` markdown inside `<Steps>` — the `counter-reset: step` on the container never increments, so no numerals render. The result on dark mode is a near-invisible 1.5 px left rule with four H3s indented inside it: visually identical to a plain heading stack, with extra ambiguous chrome. This is the single biggest "visual lie" on the site. Trivial to fix per page, biggest read-time payoff.

2. **Ship a custom `<CodeTabs>` and use it for the "extended vs tree-shakable" example pairs and the "Before / After" migration block.** Vocs does not publicly export `Tabs`. `HomePage.InstallPackage` uses an internal tabs widget on the landing already, but content pages must stack code blocks. The "Tree-shakable style" section of `concepts/clients.mdx` and the import block at the top of `python/migration/hyperliquid.mdx` are the two textbook offenders — they currently show two near-identical code fences in sequence, which the eye reads as repetition, not contrast. Source for `<CodeTabs>` below.

3. **Normalize landing-page card heights with a `<PackageCard>` grid.** The TS card on the landing is visibly taller than the Python and Rust cards because `HomePage.InstallPackage` (3-tab control) ships with a tab bar and the other two cards use a single code fence. At 1440 px wide, the grid lays out 2-on-top + 1-below, and the Rust card on the second row looks orphaned. A small wrapper component with `grid-template-rows: auto auto auto 1fr auto` and a `min-height` on the install slot fixes both the height mismatch and the orphan layout. Source for `<PackageCard>` below.

4. **Fix the install-page sub-packages snippet horizontal overflow.** On `/typescript/getting-started/installation` at 1440 px, the `pnpm add @left-curve/crypto @left-curve/encoding @left-curve/types @left-curve/utils` line has `scrollWidth: 754` against `clientWidth: 700` — 54 px hidden with no visible scroll affordance until you mouse over it. The line is also content the reader needs to copy in full. Either (a) break it onto multiple wrapped lines in the source, (b) force the code block to allow soft-wrap for bash (`white-space: pre-wrap` scoped to `language-bash` blocks), or (c) replace the rendered shell line with one `pnpm add` per sub-package across three lines.

5. **Collapse H2-between-section visual rule.** Every H2 on every Reference page renders `border-top: 1px solid var(--vocs-color_border); margin-top: 56px`. On short pages like `types/Coin.mdx` that visit Definition → Fields → Construction → Notes → See also, the page reads as five identical horizontal-rule + heading bands. Reduce to `margin-top: 48px; border-top: none` and let the heading typography do the section signalling on its own, OR keep the rule but reduce `margin-top` to `40px` so the rule lands closer to the next H2. This is a single Vocs CSS variable override at the project level (when `styles.css` is eventually extracted, the rule belongs there).

## Detailed findings

### Landing page (`/`)

- **Card-height mismatch.** TS card uses `HomePage.InstallPackage` (3 tabs + 1 line of code), Python and Rust use a 1-line code fence each. At 1440 wide the cards reflow into 2+1 because the TS card forces extra height. At 390 wide the cards stack 1-per-row so it doesn't matter. **Where:** desktop, 1440 and below. **Effort:** small (component swap, see Component proposals).
- **The bottom bullet list is visually noisy.** "What every SDK gives you" is five bullets immediately under the dense card grid. Each bullet has a bolded lead phrase ("Account model — …") that the eye reads as a separate label. The result is the bullets feel like a second card row. Visually demote them: drop the bolding on the lead noun, or convert to two-column layout at >960 px. **Where:** lines 73-77 in `pages/index.mdx`. **Effort:** trivial.
- **Mobile landing reads well.** At 390 px the cards stack cleanly, the buttons row stays inline, the "Where to go next" bullets are readable. Do not change.

### Typography and section rhythm (all Reference pages)

- **H1 → lead description → H2 has three visual rules in the first 600 px.** The H1 carries its own `border-bottom`, the H2 carries its own `border-top`, and the resulting alternation `H1 / rule / lead / rule / H2 / content / rule / H2 / ...` is too many horizontals on type pages where each section body is 1-3 lines. See `ts-coin-desktop-light.png` for the cleanest case where this hurts.
- **Steps doesn't increment because pages wrap raw H3s instead of `<Step>` blocks.** Top finding. See ranked change 1.
- **H3 inside Steps lacks visual hierarchy.** Even if we ignore the missing marker, H3 inside the Steps box looks indistinguishable from H3 outside the Steps box, because the Steps box adds no visible color/weight differentiation in dark mode (border rgb(43,41,45) on background rgb(28,28,28)). Dark-mode rule needs ~2 shades more luminance contrast.

### Sidebar

- **Long when expanded but readable.** TS sidebar shows `Getting Started`, `Concepts` (8 items, collapsed by default), then `API Reference` with `Clients` (3, collapsed), seven `Actions: *` groups (all collapsed), `Types` (42 items, collapsed), `Errors` (4, collapsed). On a content page like `getBalance`, only `Actions: App` auto-expands. This is exactly right — the prior beauty-density review's worry about the sidebar feeling like a vertical wall doesn't pan out in practice.
- **One auto-expanded section header gets a dashed blue focus ring after opening the mobile menu.** Visible in `ts-mobile-sidebar-open.png`. This is the Vocs default focus indicator (`outline: rgb(59, 158, 255) dashed 2px`) and is part of keyboard accessibility — do not change.

### Code blocks

- **Sub-packages install line overflows on installation page at 1440 wide.** See ranked change 4.
- **Migration `Before / After` import block on mobile.** The shell-style comments `# Before — hyperliquid` and `# After — dango` mark the split, but on mobile (390 wide) the `dango.hyperliquid_compatibility.exchange` import already truncates and the user can't tell at a glance whether they're seeing "before" or "after." See ranked change 2 — a `<CodeTabs labels={["Before — hyperliquid", "After — dango"]}>` resolves both the visual cohesion and the mobile truncation.
- **`createPublicClient` Methods cascade is long but the right-hand TOC handles it.** The Methods H2 has H3s for each domain (App, Account Factory, DEX, Perps, Oracle, Indexer, Hyperlane), and each H3 is a TOC entry on the right rail. So a reader can jump to "DEX" with one click. Mobile loses the right TOC, but the per-domain tables are short enough to scroll. The prior reviewer's call to convert this to tabs is correct in spirit, but lower priority than the Steps fix and the install-page tabs fix.

### Callouts

- **`:::warning[DEX currently disabled]` renders well in both modes.** Light: pale yellow background, dark amber text, orange triangle icon. Dark: olive background, brighter amber text, same icon. Legible.
- **Steps wrapper border vs callout border consistency.** The Steps `border-left: 1.5px solid var(--vocs-color_border)` is invisible in dark mode because the border color (`#2b292d` / rgb(43,41,45)) sits one shade above the dark background (rgb(28,28,28)). The same border color works fine for callouts because callouts also use a tinted background. Steps needs its own slightly higher-contrast token, or to use `background2` as a sibling background to make the left rail visible.

### Tables

- **The 14-column-wide method tables on `createPublicClient` are not actually wide.** They render with two columns: `Method` (auto-fit to longest method name, ~120 px) and `Description` (rest). The longest method name in the App table is `signAndBroadcastTx`. At desktop the table reads as a tight chip-aligned column followed by short descriptions — exactly what a methods table should be.
- **Method tables render acceptably at 390 px** (some `scrollWidth` of 353 vs `clientWidth` of 343 — a 10 px horizontal overflow on individual cells). The content reflows: descriptions wrap onto a second line per row. No work needed.

### Mobile

- **Hamburger menu opens with a focused first-section header showing dashed blue outline.** A11y feature, leave alone.
- **The "Actions, grouped by domain" 9-link comma-separated run on `/typescript/index.mdx` wraps poorly at 320 px.** Half the links wrap onto a second/third line with commas at the start of a line, reading as orphaned punctuation. The prior beauty-density review flagged this — visual confirmation that it's worse on mobile than desktop. Effort: small (one MDX edit allowed for the language root; out of this review's scope but worth recording).

### What I deliberately did not test

- I did not run a click-through of every internal `[link](...)` to verify no 404s. Console showed no warnings during the navigation set above, and the dev server reported every page rendered.
- I did not load any Python or Rust action page outside `migration/hyperliquid`. The Python and Rust trees follow the same templates as TS, so the findings on `getBalance` and `Coin` generalize.
- Search functionality renders correctly and search results scope correctly when typing — no design issues uncovered.

## Component proposals

Two components in `sdk/docs/components/`. Both use only Vocs CSS custom properties so they inherit theming for free.

### `<CodeTabs />`

A minimal accessible tabs widget for code-pair variants. Vocs does not publicly export its internal `Tabs`. This is a hand-rolled replacement using a single `useState` and ARIA roles. Source at `sdk/docs/components/CodeTabs.tsx`:

```tsx
import {
  Children,
  type ReactElement,
  type ReactNode,
  isValidElement,
  useId,
  useState,
} from "react";

export type CodeTabsProps = {
  labels: readonly string[];
  defaultIndex?: number;
  children: ReactNode;
};

export function CodeTabs({ labels, defaultIndex = 0, children }: CodeTabsProps) {
  const [active, setActive] = useState(defaultIndex);
  const groupId = useId();
  const panels = Children.toArray(children).filter(isCodeTab);

  if (panels.length !== labels.length) {
    throw new Error(
      `<CodeTabs>: labels.length (${labels.length}) must match <CodeTab> children count (${panels.length}).`,
    );
  }

  return (
    <div className="docs-CodeTabs">
      <div role="tablist" aria-orientation="horizontal" className="docs-CodeTabs__list">
        {labels.map((label, i) => {
          const isActive = i === active;
          const tabId = `${groupId}-tab-${i}`;
          const panelId = `${groupId}-panel-${i}`;
          return (
            <button
              key={tabId}
              id={tabId}
              role="tab"
              type="button"
              aria-selected={isActive}
              aria-controls={panelId}
              tabIndex={isActive ? 0 : -1}
              className="docs-CodeTabs__trigger"
              data-active={isActive}
              onClick={() => setActive(i)}
              onKeyDown={(e) => {
                if (e.key === "ArrowRight") setActive((active + 1) % labels.length);
                if (e.key === "ArrowLeft")
                  setActive((active - 1 + labels.length) % labels.length);
                if (e.key === "Home") setActive(0);
                if (e.key === "End") setActive(labels.length - 1);
              }}
            >
              {label}
            </button>
          );
        })}
      </div>
      {panels.map((panel, i) => {
        const isActive = i === active;
        const tabId = `${groupId}-tab-${i}`;
        const panelId = `${groupId}-panel-${i}`;
        return (
          <div
            key={panelId}
            id={panelId}
            role="tabpanel"
            aria-labelledby={tabId}
            hidden={!isActive}
            className="docs-CodeTabs__panel"
          >
            {panel}
          </div>
        );
      })}
      <style>{`
        .docs-CodeTabs { margin: 1.5rem 0; }
        .docs-CodeTabs__list {
          display: flex; gap: 0.25rem;
          border-bottom: 1px solid var(--vocs-color_border);
        }
        .docs-CodeTabs__trigger {
          appearance: none; background: transparent; border: none;
          border-bottom: 2px solid transparent;
          color: var(--vocs-color_text3);
          cursor: pointer; font: inherit; font-size: 0.8125rem;
          padding: 0.5rem 0.75rem;
          transform: translateY(1px);
        }
        .docs-CodeTabs__trigger:hover { color: var(--vocs-color_text2); }
        .docs-CodeTabs__trigger[data-active="true"] {
          color: var(--vocs-color_text);
          border-bottom-color: var(--vocs-color_textAccent, var(--vocs-color_borderAccent));
        }
        .docs-CodeTabs__panel { padding-top: 0.25rem; }
        .docs-CodeTabs__panel > :first-child { margin-top: 0.5rem !important; }
        .docs-CodeTabs__panel > :last-child { margin-bottom: 0 !important; }
      `}</style>
    </div>
  );
}

export type CodeTabProps = { children: ReactNode };

export function CodeTab({ children }: CodeTabProps) {
  return <>{children}</>;
}
CodeTab.displayName = "CodeTab";

function isCodeTab(node: ReactNode): node is ReactElement<CodeTabProps> {
  return (
    isValidElement(node) &&
    typeof node.type !== "string" &&
    (node.type as { displayName?: string }).displayName === "CodeTab"
  );
}
```

**Where to use:**
- `pages/typescript/concepts/clients.mdx` — "Tree-shakable style" section, paired with the extended example above it.
- `pages/python/migration/hyperliquid.mdx` — top-of-page import block split into "Before — hyperliquid" / "After — dango".
- `pages/typescript/actions/app/transfer.mdx` and other Action pages that currently show only the extended form — opt-in tabs for the tree-shakable variant.

**Example MDX usage:**

```mdx
import { CodeTabs, CodeTab } from '../../components/CodeTabs'

<CodeTabs labels={["Extended", "Tree-shakable"]}>
  <CodeTab>

  ```ts
  const amount = await client.getBalance({ address, denom: "dango" })
  ```

  </CodeTab>
  <CodeTab>

  ```ts
  const amount = await getBalance(client, { address, denom: "dango" })
  ```

  </CodeTab>
</CodeTabs>
```

### `<PackageCard />`

A normalized landing-page card that enforces equal height across the three SDK cards. Source at `sdk/docs/components/PackageCard.tsx`:

```tsx
import type { ReactNode } from "react";

export type PackageCardProps = {
  title: string;
  packageName: string;
  summary: string;
  bestFor: string;
  install: ReactNode;
  href: string;
};

export function PackageCard({
  title,
  packageName,
  summary,
  bestFor,
  install,
  href,
}: PackageCardProps) {
  return (
    <article className="docs-PackageCard">
      <h3 className="docs-PackageCard__title">{title}</h3>
      <p className="docs-PackageCard__summary">
        <code>{packageName}</code> — {summary}
      </p>
      <p className="docs-PackageCard__bestFor">
        <strong>Best for:</strong> {bestFor}
      </p>
      <div className="docs-PackageCard__install">{install}</div>
      <p className="docs-PackageCard__link">
        <a href={href}>Get started →</a>
      </p>
    </article>
  );
}

export type PackageCardGridProps = { children: ReactNode };

export function PackageCardGrid({ children }: PackageCardGridProps) {
  return (
    <div className="docs-PackageCardGrid">
      {children}
      <style>{`
        .docs-PackageCardGrid {
          display: grid;
          grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
          gap: 1.25rem;
          margin: 2.5rem 0 1.5rem;
        }
        .docs-PackageCard {
          display: grid;
          grid-template-rows: auto auto auto 1fr auto;
          gap: 0.75rem;
          padding: 1.25rem 1.5rem;
          border: 1px solid var(--vocs-color_border, rgba(127, 127, 127, 0.2));
          border-radius: 12px;
          background: var(--vocs-color_background2, transparent);
          transition: border-color 0.2s ease, transform 0.2s ease;
        }
        .docs-PackageCard:hover {
          border-color: var(--vocs-color_borderAccent, rgba(127, 127, 127, 0.5));
          transform: translateY(-2px);
        }
        .docs-PackageCard__title { margin: 0 !important; font-size: 1.125rem; }
        .docs-PackageCard__summary,
        .docs-PackageCard__bestFor {
          margin: 0 !important; font-size: 0.9375rem; line-height: 1.45;
        }
        /* Reserve the height of a 3-tab install widget so cards with a plain
           code fence don't read as "shorter". */
        .docs-PackageCard__install {
          min-height: 6.5rem;
          display: flex; align-items: flex-start;
        }
        .docs-PackageCard__install > * { width: 100%; margin: 0 !important; }
        .docs-PackageCard__link { margin: 0 !important; font-size: 0.9375rem; }
        .docs-PackageCard__link a { font-weight: 500; }
      `}</style>
    </div>
  );
}
```

**Where to use:** `pages/index.mdx` only. Replaces the three `<div className="sdk-card">` blocks and the trailing `<style>` block.

**Example MDX usage:**

```mdx
import { PackageCard, PackageCardGrid } from './components/PackageCard'
import { HomePage } from 'vocs/components'

<PackageCardGrid>
  <PackageCard
    title="TypeScript"
    packageName="@left-curve/sdk"
    summary="viem-style client. Public and signer clients, action functions, WebSocket subscriptions, tree-shakable imports."
    bestFor="browser dApps, Node.js services, indexers, automated traders."
    install={<HomePage.InstallPackage name="@left-curve/sdk" />}
    href="/typescript/getting-started/installation"
  />
  <PackageCard
    title="Python"
    packageName="dango"
    summary="Exchange + Info classes, plus a Hyperliquid-compatibility layer."
    bestFor="quantitative research, backtests, scripts migrated from Hyperliquid."
    install={<pre><code>uv add dango</code></pre>}
    href="/python/getting-started/installation"
  />
  <PackageCard
    title="Rust"
    packageName="dango-sdk"
    summary="async GraphQL client, WebSocket subscriptions, key management, transaction signing."
    bestFor="low-latency market makers, on-chain services, tooling that already depends on grug."
    install={<pre><code>{`[dependencies]\ndango-sdk = "0"`}</code></pre>}
    href="/rust/getting-started/installation"
  />
</PackageCardGrid>
```

## What surprised me

These looked worse in the text-only review than they actually are in the browser. Do not change them.

- **The 67-row method cascade on `createPublicClient` is fine.** The Methods H2 contains an H3 per domain, each H3 is a right-rail TOC entry, and each per-domain table is short. The reader is one click away from the right domain. The cascade only looks bad in the full-page screenshot.
- **The Methods table at mobile width.** The `Method | Description` two-column layout reflows gracefully at 390 and even 320 px. Method-name chips wrap onto a second line per row when needed. No tabs, no card variant — just leave it.
- **The DEX warning callout in dark mode.** The amber-on-olive contrast is high enough to read without being garish. The icon is a clear caution triangle. Three lines of copy. Nothing to tune.
- **The `Coin` type page Fields section.** The `name — type . description` definition-list pattern (bolded backticked name + em-dash + description) renders very cleanly. Better than a 3-column table. The prior reviewer's call to keep this is correct.
- **Mobile sidebar.** Opens with a soft slide, the section headers are clear, the active route is bolded. The "Project Setup" line carries the active styling correctly.
- **Search overlay.** Renders well in both modes, keyboard shortcuts displayed (`Navigate ↑↓`, `Select Enter`, `Close Esc`, `Reset ⌘ + ⌫`), modal contrast is right. No need to skin it.

## Hard pass

Tempting changes that would violate the style guide or the constraints in the task prompt.

- **Do not adopt twoslash.** The TODO in `vocs.config.ts` is right to wait. Turning twoslash on means every partial snippet must type-check, which becomes a chore on Type pages that intentionally show only the user-facing slice of a definition.
- **Do not introduce a `<HeroSection>` or animated background on the landing.** No marketing surface. The current `HomePage.Tagline + HomePage.Description + HomePage.Buttons + card grid` is the right scope.
- **Do not extract `styles.css` yet just to add one of these CSS-only fixes.** Both components above ship their styles via inline `<style>` blocks (matching the existing pattern on `pages/index.mdx`). The styles.css extraction can happen later when there's enough shared CSS to justify it.
- **Do not redesign the right-hand TOC into a sticky sidebar with progress indicator.** Vocs default works. Anchor scroll is fine.
- **Do not introduce custom dark/light-mode illustrations for the landing.** Vocs already handles theme switching for code blocks; the cards inherit `background2`. Adding mode-specific imagery would inflate the bundle and would never get maintained.
- **Do not weaken the focus ring on `vocs_Sidebar_sectionHeader`.** The dashed blue outline that flashes when the mobile menu opens looks loud in a screenshot but is correct keyboard a11y. Tweaking it would regress.
- **Do not add "Was this page helpful?" / feedback widget / per-page edit-on-GitHub link.** Out of scope.
- **Do not add `font-variant-numeric: tabular-nums` to method tables.** The prior reviewer's instinct here is right — there are no actual digits in the descriptions, so it does nothing visible.
- **Do not change the H1 underline.** The H1 border-bottom is fine. The H2 border-top is what feels heavy when many short sections stack — see ranked change 5.
- **Do not switch the Parameters definition-list to a table.** Already correct.
- **Do not remove the "Last updated" timestamp footer.** The prior beauty-density review's "hard pass" item is right — git history is the source of truth — but the field is already in Vocs default and removing it is one config flag; out of scope for a visual review.
