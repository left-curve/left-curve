# Visual + Density Recommendations

Scope: presentation only — templates, voice, and content unchanged. All recommendations respect the style guide bans (no emojis, no marketing, imperative voice).

Sample reviewed: `pages/index.mdx`, `pages/typescript/index.mdx`, `pages/typescript/getting-started/{installation,first-call,project-setup}.mdx`, `pages/typescript/concepts/{clients,rate-limits,transactions}.mdx`, `pages/typescript/clients/createPublicClient.mdx`, `pages/typescript/actions/app/{transfer,getBalance}.mdx`, `pages/typescript/actions/dex/swapExactAmountIn.mdx`, `pages/typescript/types/Coin.mdx`, `pages/python/{index,migration/hyperliquid}.mdx`, `pages/rust/{index,concepts/transactions}.mdx`.

## Top 5 highest-impact changes

1. **Adopt Vocs `:::callout` directives everywhere; retire the `> **Warning:**` blockquote style.** Two visual styles for the same notion exist today (70 files use directives, 10 files use blockquotes). The blockquote form is what the concept template recommends — that template is out of date.
2. **Wrap the multi-domain Methods table in `createPublicClient.mdx` with `<Tabs>` per domain.** 67 method rows across 7 H3 sections is the longest scan on the site. Tabs collapse the visible surface to one domain at a time, keep the URLs/anchors stable, and let the cross-domain reader still see the labels at a glance.
3. **Collapse `Actions: Perps` and `Types` sidebar sections by default for all three languages**; expand only the current section programmatically (Vocs auto-expands the active path). The TS sidebar alone lists 28 Perps actions and 38 Types. New readers see a vertical wall.
4. **Use `<Tabs>` for `Installation` package-manager blocks and for the "extended vs tree-shakable" example pair in `concepts/clients.mdx`.** Both are textbook tabs use-cases: same shape, different variant, currently stacked vertically.
5. **Introduce a single shared MDX partial for the "DEX currently disabled" warning** and import it on every applicable page. Today the warning is hand-pasted; one source means one change when the DEX flips on.

## Detailed recommendations

### Landing page (`pages/index.mdx`)

- **Recommendation:** Keep the card grid; tighten the "What every SDK gives you" list. Convert each bullet's bolded lead phrase into an H4 followed by a one-line body, or fold the dependent clauses tighter. Currently each line wraps to two lines on mid-width viewports, which fights the card grid above it visually.
  - **Why:** The grid is dense and balanced; the bullet list below it is loose and reads as filler.
  - **Where:** `pages/index.mdx` lines 75-81.
  - **Effort:** small.

- **Recommendation:** Move the inline `<style>` block to a `styles.css` imported once. Vocs supports a top-level CSS file (`theme.css` / `styles.css` referenced from `vocs.config.ts`). Inline style on the landing is fine, but if any other page needs the `.sdk-cards` grid (e.g., section roots), the rule should be central.
  - **Why:** Pre-empts future drift; one place to tune spacing.
  - **Where:** `pages/index.mdx` lines 89-114.
  - **Effort:** small.

- **Recommendation:** Replace the bare horizontal rule on line 73 with a single empty line — the section heading "What every SDK gives you" is enough separator, and `<hr>` between an H2 and a card grid reads as visual noise on top of the card borders.
  - **Why:** Two visual dividers in a row.
  - **Where:** `pages/index.mdx` line 73.
  - **Effort:** trivial.

### Language root pages (`pages/{ts,py,rust}/index.mdx`)

- **Recommendation:** Make all three language roots structurally identical: one-paragraph intro, then exactly three H2s in this order — `Start here`, `Concepts`, `Reference`. TS uses `## Start here / ## Concepts / ## API Reference`; Python uses `## Where to start / ## Reference / ## Migrating from Hyperliquid`; Rust uses `## Start here / ## Concepts` (no reference section). The lists themselves are bullet-link sets, which is fine, but the section names and order should match.
  - **Why:** Cross-language consistency is the primary navigation contract of a three-language docs site.
  - **Where:** `pages/typescript/index.mdx`, `pages/python/index.mdx`, `pages/rust/index.mdx`.
  - **Effort:** small.

- **Recommendation:** On `pages/typescript/index.mdx`, the inline list at line 25 ("Actions, grouped by domain: App, DEX, …") is a 9-link comma-separated run. Split into a definition list or compact 2-column grid using the same `.sdk-cards` pattern, or collapse to one link per line.
  - **Why:** Scanability — readers lose track midway through.
  - **Where:** `pages/typescript/index.mdx` line 25.
  - **Effort:** small.

- **Recommendation:** Python root's lead paragraph (lines 3-5) buries the entry-point list inside a sentence and then has a code block listing imports. Lift the import list above the prose paragraph and let the block speak first.
  - **Why:** Code-first readers want the import shape immediately.
  - **Where:** `pages/python/index.mdx` lines 3-13.
  - **Effort:** small.

### Reference pages (Action / Type / Client)

- **Recommendation:** **Action pages — wrap "extended-style vs tree-shakable" example in `<Tabs>` on every Action page that has both.** Today Action pages show only the extended form (`client.transfer(...)`). The style guide says "tabs can show the tree-shakable style as alternate." Make the tabs convention concrete in the template so future pages adopt it. Skip it on Action pages where there is no tree-shakable equivalent (e.g., subscriptions, Rust pages).
  - **Why:** Bundle-size-sensitive readers currently have to dig into the Concepts page; tabs make both styles visible in one click.
  - **Where:** template `action.mdx`; sample target `pages/typescript/actions/app/transfer.mdx`.
  - **Effort:** medium (template + sweep of existing Action pages by drafter agents).

- **Recommendation:** **Client pages — convert the multi-domain Methods table to `<Tabs>` keyed by domain.** The current H3-per-domain layout in `createPublicClient.mdx` produces a 67-row scroll. Tabs preserve every link and label but show one domain at a time. Keep the H3s as the tab triggers — content stays the same. Apply to TS `createPublicClient`, `createSignerClient`, Python `Exchange`/`Info`/`API`, and Rust `HttpClient`.
  - **Why:** Largest reduction in scroll-fatigue across the site for a contained edit.
  - **Where:** `pages/typescript/clients/createPublicClient.mdx` lines 27-124; equivalents in Python/Rust.
  - **Effort:** medium.

- **Recommendation:** **Action pages — collapse `Signature` and `Example` whitespace.** Today there's a full blank line between `## Signature`, the code block, the next H2, and the next code block. The visual rhythm is "heading, fence, heading, fence" with three blank lines between each — could be two. Net effect is the first-fold view shows only the H1 and signature on a typical laptop screen.
  - **Why:** Reference pages are scanned, not read top-to-bottom. Tighter spacing lets the Parameters table reach the fold.
  - **Where:** Visible in `pages/typescript/actions/app/transfer.mdx` and `swapExactAmountIn.mdx`.
  - **Effort:** small (theme CSS tweak: reduce `--vocs-content_horizontalPadding`-equivalent vertical rhythm on `h2 + pre` pairs).

- **Recommendation:** **Type pages with no `Notes` section should omit the heading entirely**, not include an empty `## Notes` block. Sample today: `Coin.mdx` correctly omits when there's no content; ensure all Type pages follow.
  - **Why:** Empty H2s in Type pages create false structure.
  - **Where:** template `type.mdx` already says optional — verify in inventory.
  - **Effort:** small (audit).

- **Recommendation:** **Standardize the "DEX currently disabled" warning as a shared MDX partial.** Create `pages/_partials/dex-disabled.mdx` with the warning, import once per applicable page (`import DexDisabled from "../../_partials/dex-disabled.mdx"`). Eliminates the trailing blank line that's drifted into `swapExactAmountIn.mdx` (line 9 is a stray blank between the callout and the next H2 — same in Rust's `transactions.mdx` line 9 and TS `concepts/transactions.mdx` line 9).
  - **Why:** One source of truth; one moment to remove the warning when the DEX is enabled.
  - **Where:** Currently duplicated across `pages/typescript/actions/dex/*.mdx`, `pages/typescript/actions/perps/*.mdx`, `pages/python/...`, `pages/rust/concepts/transactions.mdx`.
  - **Effort:** medium (create partial, sweep ~60 pages).

### Concept pages

- **Recommendation:** **Add a leading H2 anchor to every concept page's "What this teaches" line.** Today the line lives as bold text directly under the H1; it has no anchor, and on long concept pages (Python `migration/hyperliquid.mdx` is 129 lines) the reader cannot deep-link to the top-of-content. Either keep the bold but wrap it in an `<a id="overview">…</a>`, or convert to a short `## Overview` H2.
  - **Why:** Linkability for in-page navigation.
  - **Where:** Template `concept.mdx` line 12.
  - **Effort:** small.

- **Recommendation:** **Switch the concept template's example callout from `> **Warning:**` to `:::warning`.** The concept template (line 23, 26-29) recommends blockquote callouts; the rest of the docs use Vocs directives. Fix the template so future concept drafters write the directive form.
  - **Why:** Visual consistency across the site (warnings should look the same in all surfaces).
  - **Where:** `.templates/concept.mdx` lines 23, 27-29.
  - **Effort:** trivial.

- **Recommendation:** **Convert numbered procedure paragraphs to `<Steps>` on `concepts/transactions.mdx` (TS and Rust).** The TS page lists "1. Build messages. 2. Sign. 3. Broadcast. 4. Poll." as a numbered list at lines 12-15; the Rust page does the same at lines 16-19. `<Steps>` from Vocs renders these as a progress-rail with vertical connectors, which signals "do this in order" much harder than a plain list.
  - **Why:** "Lifecycle" sections are the highest-value place for the Steps component on the entire site.
  - **Where:** `pages/typescript/concepts/transactions.mdx` lines 12-15; `pages/rust/concepts/transactions.mdx` lines 14-19.
  - **Effort:** small.

- **Recommendation:** **Break the "What you must change explicitly" / "Methods that wrap native equivalents" / "Methods that diverge" cascade on `python/migration/hyperliquid.mdx` with an H2 table of contents at the top.** The page is one long scroll with 7 H2 sections, each containing sub-prose plus a 25-row table. A top-of-page "On this page: …" bullet list lets the migrating reader jump straight to their concern.
  - **Why:** Migration pages are diagnostic — readers come with a specific failure mode in mind.
  - **Where:** `pages/python/migration/hyperliquid.mdx`.
  - **Effort:** small.

- **Recommendation:** **Use `<Tabs>` for the "Before / After" import block at the top of `migration/hyperliquid.mdx`.** Today the two import shapes are stacked vertically in one code block, separated by comments. Tabs labeled "Before (hyperliquid)" and "After (dango)" make the contrast a one-glance read.
  - **Why:** Migration begins at the import line — make that contrast crisp.
  - **Where:** `pages/python/migration/hyperliquid.mdx` lines 5-15.
  - **Effort:** trivial.

### Sidebar / Navigation

- **Recommendation:** **Set `collapsed: true` on `Concepts` for all three languages.** Today TS and Rust have Concepts uncollapsed; the section runs 8 items for TS. Most readers know the concept they want before they arrive, and the section root link is visible regardless via the topNav and language `index.mdx`.
  - **Why:** Saves ~150 vertical pixels on every reference page load.
  - **Where:** `vocs.config.ts` lines 44, 491 (TS, Rust concepts).
  - **Effort:** trivial.

- **Recommendation:** **Group the TS `Clients` section as collapsed by default**, same logic as above. Three items, but uncollapsed by default while `Actions: App` is collapsed creates visual imbalance — a reader on `getBalance` sees `Clients` open and 7 `Actions:` sections closed.
  - **Why:** Symmetry of the navigation tree.
  - **Where:** `vocs.config.ts` line 58.
  - **Effort:** trivial.

- **Recommendation:** **Add an intermediate "API Reference" parent in the TS sidebar that wraps `Clients`, all `Actions: *`, `Types`, and `Errors`.** Python and Rust already have a single `API Reference` parent. TS has 9 top-level sidebar groups (`Getting Started`, `Concepts`, `Clients`, six `Actions: *`, `Types`, `Errors`). That's twice as many top-level groups as the other two languages.
  - **Why:** Cross-language sidebar parity. Today TS feels heavier.
  - **Where:** `vocs.config.ts` lines 56-278.
  - **Effort:** small.

- **Recommendation:** **Verify that all action subgroups (e.g., `Actions: Perps`) keep `collapsed: true`**, which they already do — *do not* change this. Confirm. Verified: lines 67, 97, 117, 140, 173, 181, 202, 209, 219, 269 all have `collapsed: true`.
  - **Why:** This is correct as-is; calling it out so a reviewer doesn't second-guess.
  - **Where:** `vocs.config.ts`.
  - **Effort:** none.

### Vocs components to adopt

- **`<Tabs>` — adopt:**
  - Package manager installs (`pnpm` / `npm` / `yarn`) on `installation.mdx` (TS) and equivalents.
  - "Extended vs tree-shakable" example pair in Action pages and `concepts/clients.mdx`.
  - "Before / After" migration import on `migration/hyperliquid.mdx`.
  - Methods table per domain on `createPublicClient.mdx`, `createSignerClient.mdx`, Python `Exchange`/`Info`, Rust `HttpClient`.
  - **Do not** adopt: inside Reference page `Example` sections that are already minimal — one example, one purpose remains the rule.

  Example shape (mock — for clients.mdx):
  ```mdx
  import { Tabs, TabsList, TabsTrigger, TabsContent } from 'vocs/components'

  <Tabs defaultValue="extended">
    <TabsList>
      <TabsTrigger value="extended">Extended (client.action)</TabsTrigger>
      <TabsTrigger value="treeshake">Tree-shakable (action(client))</TabsTrigger>
    </TabsList>
    <TabsContent value="extended">

    ```ts
    const amount = await client.getBalance({ address, denom: "dango" })
    ```

    </TabsContent>
    <TabsContent value="treeshake">

    ```ts
    const amount = await getBalance(client, { address, denom: "dango" })
    ```

    </TabsContent>
  </Tabs>
  ```

- **`<Steps>` — adopt:**
  - `concepts/transactions.mdx` (TS + Rust) "lifecycle" lists.
  - `python/migration/hyperliquid.mdx` "What you must change explicitly" — three required changes, currently as H3 subsections.
  - **Do not** adopt: in `installation.mdx`, where each "step" is independent (install → tsconfig → next).

- **`:::warning` / `:::note` / `:::tip` / `:::danger`** — already widely adopted; the work is removing the inconsistent `> **Warning:**` blockquote form (10 files, listed below) and updating the concept template.
  - Files to migrate from blockquote to directive: `rust/concepts/subscriptions.mdx`, `rust/concepts/transactions.mdx`, `typescript/concepts/encoding-and-types.mdx`, `python/migration/hl-compat/exchange/cancel_by_cloid.mdx`, `typescript/actions/app/getBalance.mdx`, `typescript/actions/app/getAppConfig.mdx`, `rust/api/clients/Keystore.mdx`, `rust/api/clients/Session.mdx`, `python/concepts/error-handling.mdx`, `python/getting-started/project-setup.mdx`.

- **`<Callout>` (the React component form)** — do not adopt over `:::` directives. The directive form is shorter and reads better in MDX source.

- **`HomePage.*`** — currently used only on `pages/index.mdx`. Keep as-is. Do not adopt on language root pages — they're navigation hubs, not marketing surfaces.

- **`Sponsors` / `Authors`** — do not adopt. Out of scope for a reference site.

- **`Button`** — do not adopt outside `HomePage`. The doc body uses inline links; mixing buttons in would feel salesy.

### Theme / CSS suggestions

Vocs defaults are good. Keep changes minimal.

- **Recommendation:** Set `--vocs-content_horizontalPadding` slightly larger on viewports >1400px so the Methods table on `createPublicClient.mdx` doesn't run line-to-line with no breathing room. Or accept current; verify in dev.
  - **Effort:** trivial.

- **Recommendation:** **Adopt a monospace-tabular numeric in tables** (`font-variant-numeric: tabular-nums`) on the global Methods/Description tables. Today method names like `getPerpsLiquidityDepth` align with backticks but column widths jiggle row-to-row when the description wraps. Tabular numbers help when columns include row counts or version numbers, less so for pure prose — apply only if a quick scan shows wins.
  - **Effort:** small. Probably skip unless drafter agents confirm a visible win.

- **Recommendation:** **Add `code { word-break: break-word; }` scoped to mobile breakpoints** so long backticked symbols like `eventsByAddressesSubscription` don't overflow the viewport on narrow widths.
  - **Why:** Methods tables are the worst-case for narrow viewports.
  - **Where:** A `styles.css` referenced from `vocs.config.ts` (does not exist yet).
  - **Effort:** small.

- **Recommendation:** Do not tweak the typography scale, font family, or color palette. Vocs' defaults are well-balanced and changing them invites bikeshedding.
  - **Effort:** zero — confirm as a non-change.

## What I'd leave alone

- **Code-fence languages.** Spot check shows consistent `ts` (not `tsx`/`typescript`), `py`, `rust`, `bash`, `json` use. No work needed.
- **Imports-on-top rule.** Verified across the sample; every code block starts with imports per style guide.
- **The `## See also` placement.** Always at the end of Reference pages. Consistent and correct.
- **Action page Parameters section as a definition list** (not a table). The template uses bolded backticked names + em-dash + description — works well and beats a 3-column table for variable-length descriptions.
- **The `vocs.config.ts` topNav.** Three links, one per language — exactly right.
- **Socials.** GitHub, Discord, X — appropriate set; nothing to add.
- **Logo / favicon / title.** All in place.
- **The DEX warning callout text.** It's clear and direct. Only the duplication is the problem.

## Hard pass list

These would be tempting suggestions a reviewer might make — they're bad ideas given the constraints.

- **Do not add emojis to callouts, headings, or section markers.** Style guide bans them explicitly.
- **Do not add a hero illustration or animated background to the landing page.** No marketing surface here; readers are debugging.
- **Do not "warm up" the voice on reference pages.** No "Welcome!", no "Let's get started!", no second-person elsewhere than Concept pages.
- **Do not introduce a search/filter widget on the Methods table.** Native browser find (Ctrl+F) covers it; a custom search adds JS overhead for a problem solved by the tab proposal above.
- **Do not redesign the sidebar to a tree-with-icons or to flat alphabetical.** The current domain grouping (`Actions: App`, `Actions: DEX`, …) is correct; resist the urge to "modernize" it.
- **Do not auto-generate "Related pages" sidebars from frontmatter.** The hand-curated `See also` sections at page bottom are higher signal.
- **Do not introduce dark/light mode-specific illustrations.** Vocs already handles theme switching for code blocks; nothing more is needed.
- **Do not adopt twoslash docs comments site-wide.** The config TODO is right to wait — turning twoslash on means every partial snippet must type-check, which becomes a chore on Type pages that intentionally show only the user-facing slice of a definition.
- **Do not switch from `definition list` style to tables for Parameters/Fields.** The bold-name + em-dash + description form scales better when descriptions span multiple lines.
- **Do not add a "Last updated" timestamp footer.** Git history is the source of truth; an in-page stamp invites maintenance drift and is meaningless to a reader who doesn't know the codebase cadence.
- **Do not introduce per-page meta-frontmatter (tags, categories, difficulty).** Out of scope; sidebar grouping is sufficient.
- **Do not move the "DEX currently disabled" warning to the sidebar/header.** It belongs above the per-page content so it can't be skimmed past.
