import { expect, test } from "@playwright/test";

test.describe("FormattedNumber tier formatting", () => {
  /**
   * Tier rules:
   * 1. < 0.0001 → subscript notation
   * 2. 0.0001–1 → 4 significant digits
   * 3. 1–100 → 4 decimal places
   * 4. 100–10K → 2 decimal places + grouping
   * 5. 10K–1M → integer + grouping
   * 6. ≥1M → compact (M/B/T) + 2 decimals
   *
   * Exception: order book PRICE column uses fractionDigits override (bucket size).
   */

  test("order book size and total columns follow tier formatting", async ({ page }) => {
    await page.goto("/trade");
    await page.waitForTimeout(5000);

    // OrderRow structure:
    //   <div class="relative ... grid grid-cols-2 lg:grid-cols-3 ..."> (row)
    //     <div> (col 0: price — uses fractionDigits override, SKIP)
    //     <div class="hidden lg:flex"> (col 1: size — default tier logic)
    //     <div> (col 2: total — default tier logic)
    //
    // We check only size (col 1) and total (col 2) columns.
    const violations = await page.evaluate(() => {
      const issues: {
        text: string;
        value: number;
        decimals: number;
        expectedDecimals: number;
        tier: number;
        column: string;
      }[] = [];

      // Select all order book row containers
      const rows = document.querySelectorAll(
        ".asks-container > div, .bid-container > div",
      );

      for (const row of rows) {
        const cols = row.children;
        if (cols.length < 2) continue;

        // Check columns: skip col 0 (price), check col 1 (size) and col 2 (total)
        for (let i = 1; i < cols.length; i++) {
          const col = cols[i];
          const text = col.textContent?.trim();
          if (!text || text === "-") continue;

          const cleaned = text.replace(/[$,]/g, "").replace(/[KMBT]$/, "");
          const num = Number.parseFloat(cleaned);
          if (Number.isNaN(num) || num === 0) continue;
          if (/[KMBT]/.test(text)) continue; // compact notation is fine

          const parts = cleaned.split(".");
          const decimals = parts[1]?.length ?? 0;
          const absNum = Math.abs(num);

          let tier = 0;
          let expectedDecimals = -1;

          if (absNum >= 1 && absNum < 100) {
            tier = 3;
            expectedDecimals = 4;
          } else if (absNum >= 100 && absNum < 10_000) {
            tier = 4;
            expectedDecimals = 2;
          } else if (absNum >= 10_000 && absNum < 1_000_000) {
            tier = 5;
            expectedDecimals = 0;
          }

          if (expectedDecimals >= 0 && decimals !== expectedDecimals) {
            issues.push({
              text,
              value: absNum,
              decimals,
              expectedDecimals,
              tier,
              column: i === 1 ? "size" : "total",
            });
          }
        }
      }

      return issues;
    });

    if (violations.length > 0) {
      const report = violations
        .map(
          (v) =>
            `  Tier ${v.tier} [${v.column}]: "${v.text}" → ${v.decimals} decimals (expected ${v.expectedDecimals})`,
        )
        .join("\n");
      expect.soft(violations.length, `Order book tier violations:\n${report}`).toBe(0);
    }
  });

  test("trade page numbers follow tier formatting", async ({ page }) => {
    await page.goto("/trade");
    await page.waitForTimeout(5000);

    // Scan all <p> elements that are rendered by FormattedNumber.
    // FormattedNumber renders as <p> (or <span>) containing <span> children for each part.
    // We read the concatenated textContent of each <p>.
    const violations = await page.evaluate(() => {
      const issues: {
        text: string;
        value: number;
        decimals: number;
        expectedDecimals: string;
        tier: number;
        context: string;
      }[] = [];

      const seen = new Set<string>();

      // Check all <p> and <span> elements that might contain formatted numbers
      const candidates = document.querySelectorAll("p, span");

      for (const el of candidates) {
        // Only check leaf elements (no child elements that are also candidates)
        if (el.querySelector("p, span")) continue;

        const text = el.textContent?.trim();
        if (!text) continue;

        // Match complete number patterns
        const match = text.match(/^([+-]?\$?[\d,]+\.?\d*)([KMBT])?(%)?$/);
        if (!match) continue;
        if (match[2]) continue; // Skip compact notation
        if (match[3]) continue; // Skip percentages

        const numStr = match[1].replace(/[$,+]/g, "");
        const num = Number.parseFloat(numStr);
        if (Number.isNaN(num) || num === 0) continue;

        const parts = numStr.split(".");
        const decimals = parts[1]?.length ?? 0;
        const absNum = Math.abs(num);
        const key = `${text}-${absNum}-${decimals}`;
        if (seen.has(key)) continue;
        seen.add(key);

        let tier = 0;
        let expectedDecimals = "";
        let isViolation = false;

        if (absNum >= 1 && absNum < 100) {
          tier = 3;
          expectedDecimals = "4";
          if (decimals !== 4) isViolation = true;
        } else if (absNum >= 100 && absNum < 10_000) {
          tier = 4;
          expectedDecimals = "2";
          if (decimals !== 2) isViolation = true;
        } else if (absNum >= 10_000 && absNum < 1_000_000) {
          tier = 5;
          expectedDecimals = "0";
          if (decimals !== 0) isViolation = true;
        }

        if (isViolation) {
          // Get some context about where this element is
          const parent = el.closest("[class]");
          const ctx = parent?.className?.split(" ").slice(0, 3).join(" ") ?? "unknown";
          issues.push({ text, value: absNum, decimals, expectedDecimals, tier, context: ctx });
        }
      }

      return issues;
    });

    if (violations.length > 0) {
      const report = violations
        .map(
          (v) =>
            `  Tier ${v.tier}: "${v.text}" → ${v.decimals} dec (expected ${v.expectedDecimals}) [${v.context}]`,
        )
        .join("\n");
      console.log(`\n=== TIER VIOLATIONS (${violations.length}) ===\n${report}\n`);
    }

    // Report violations but use soft assertion to see ALL of them
    expect(
      violations.length,
      `Found ${violations.length} tier violations:\n${violations.map((v) => `  Tier ${v.tier}: "${v.text}" (${v.decimals} dec, expected ${v.expectedDecimals}) [${v.context}]`).join("\n")}`,
    ).toBe(0);
  });
});
