import { describe, expect, it } from "vitest";
import { formatDisplayNumber, formatDisplayString, formatNumber } from "./formatters.js";
import type { FormatNumberOptions } from "./formatters.js";

const defaultOpts: FormatNumberOptions = {
  language: "en-US",
  mask: 1,
};

const usdOpts: FormatNumberOptions = {
  ...defaultOpts,
  currency: "USD",
};

function fmt(value: string | number, opts = defaultOpts): string {
  return formatDisplayString(formatDisplayNumber(value, opts));
}

describe("formatDisplayNumber tier logic", () => {
  describe("zero", () => {
    it("renders 0 without currency", () => {
      expect(fmt(0)).toBe("0");
      expect(fmt("0")).toBe("0");
      expect(fmt("0.000000")).toBe("0");
    });

    it("renders $0.00 with currency", () => {
      expect(fmt(0, usdOpts)).toBe("$0.00");
    });
  });

  describe("tier 1: < 0.0001 — subscript notation", () => {
    it("formats very small numbers with subscript", () => {
      const parts = formatDisplayNumber("0.00001234", defaultOpts);
      const types = parts.map((p) => p.type);
      expect(types).toContain("subscript");
      // 0.0₄1234 — subscript is rendered as unicode
      expect(fmt("0.00001234")).toBe("0.0₄1234");
    });

    it("formats with currency prefix", () => {
      const result = fmt("0.00001234", usdOpts);
      expect(result).toContain("$");
    });
  });

  describe("tier 2: 0.0001 ≤ num < 1 — 4 significant digits", () => {
    it("formats 0.001234", () => {
      expect(fmt("0.001234")).toBe("0.001234");
    });

    it("formats 0.5678", () => {
      expect(fmt("0.5678")).toBe("0.5678");
    });

    it("formats 0.1 (max 4 sig digits, no trailing zeros)", () => {
      expect(fmt("0.1")).toBe("0.1");
    });

    it("formats 0.99999 (rounds to 1 with 4 sig digits)", () => {
      expect(fmt("0.99999")).toBe("1");
    });
  });

  describe("tier 3: 1 ≤ num < 100 — up to 4 decimal places", () => {
    it("formats integers without trailing zeros", () => {
      expect(fmt(1)).toBe("1");
      expect(fmt(2)).toBe("2");
      expect(fmt(50)).toBe("50");
    });

    it("formats 42.1234", () => {
      expect(fmt("42.1234")).toBe("42.1234");
    });

    it("formats 99.99 without trailing zeros", () => {
      expect(fmt("99.99")).toBe("99.99");
    });

    it("formats 1.23456789 (truncates to 4 decimals)", () => {
      expect(fmt("1.23456789")).toBe("1.2346");
    });

    it("formats 1.5 without padding", () => {
      expect(fmt("1.5")).toBe("1.5");
    });
  });

  describe("tier 4: 100 ≤ num < 10,000 — up to 2 decimal places + grouping", () => {
    it("formats integers without trailing zeros", () => {
      expect(fmt(100)).toBe("100");
      expect(fmt(1040)).toBe("1,040");
    });

    it("formats 300.000000 without trailing zeros", () => {
      expect(fmt("300.000000")).toBe("300");
    });

    it("formats 449.6930", () => {
      expect(fmt("449.6930")).toBe("449.69");
    });

    it("formats 2376.0688", () => {
      expect(fmt("2376.0688")).toBe("2,376.07");
    });

    it("formats 6441.6278", () => {
      expect(fmt("6441.6278")).toBe("6,441.63");
    });

    it("formats 9999.99", () => {
      expect(fmt("9999.99")).toBe("9,999.99");
    });

    it("formats with USD currency", () => {
      expect(fmt("2376.0688", usdOpts)).toBe("$2,376.07");
    });
  });

  describe("tier 5: 10,000 ≤ num < 1,000,000 — integer + grouping", () => {
    it("formats 10000", () => {
      expect(fmt(10000)).toBe("10,000");
    });

    it("formats 84828", () => {
      expect(fmt("84828")).toBe("84,828");
    });

    it("formats 999999.99 (rounds to integer)", () => {
      expect(fmt("999999.99")).toBe("1,000,000");
    });

    it("formats with USD currency", () => {
      expect(fmt("84828", usdOpts)).toBe("$84,828");
    });
  });

  describe("tier 6: ≥ 1,000,000 — compact (M/B/T) + 2 decimals", () => {
    it("formats 1000000", () => {
      expect(fmt(1000000)).toBe("1.00M");
    });

    it("formats 13478818", () => {
      expect(fmt("13478818")).toBe("13.48M");
    });

    it("formats 1500000000", () => {
      expect(fmt("1500000000")).toBe("1.50B");
    });

    it("formats with USD currency", () => {
      expect(fmt("13478818", usdOpts)).toBe("$13.48M");
    });
  });

  describe("fractionDigits override", () => {
    it("bypasses tier logic with fractionDigits: 0", () => {
      expect(fmt(7, { ...defaultOpts, fractionDigits: 0 })).toBe("7");
      expect(fmt(1234, { ...defaultOpts, fractionDigits: 0 })).toBe("1,234");
    });

    it("bypasses tier logic with fractionDigits: 2", () => {
      expect(fmt("84828.123", { ...defaultOpts, fractionDigits: 2 })).toBe("84,828.12");
    });

    it("forces exact decimals regardless of tier", () => {
      expect(fmt("0.5", { ...defaultOpts, fractionDigits: 8 })).toBe("0.50000000");
    });
  });

  describe("negative numbers", () => {
    it("preserves sign across tiers", () => {
      expect(fmt("-42.1234")).toBe("-42.1234"); // tier 3
      expect(fmt("-2")).toBe("-2"); // tier 3 integer
      expect(fmt("-2376.0688")).toBe("-2,376.07"); // tier 4
      expect(fmt("-84828")).toBe("-84,828"); // tier 5
    });
  });
});

describe("formatNumber (string output)", () => {
  it("returns same as formatDisplayString(formatDisplayNumber(...))", () => {
    const value = "2376.0688";
    const expected = formatDisplayString(formatDisplayNumber(value, defaultOpts));
    expect(formatNumber(value, defaultOpts)).toBe(expected);
  });
});

/**
 * Simulate the exact merge that FormattedNumber does:
 *   formatDisplayNumber(number, { ...formatNumberOptions, ...formatOptions })
 *
 * formatNumberOptions = app context (e.g. { language: "en-US", mask: 1 })
 * formatOptions       = component prop (e.g. { currency: "USD" } or undefined)
 */
describe("FormattedNumber context merge simulation", () => {
  // App context defaults (from AppProvider)
  const appCtx: FormatNumberOptions = { language: "en-US", mask: 1 };

  function fmtComponent(
    value: string | number,
    formatOptions?: Partial<FormatNumberOptions>,
  ): string {
    // Exactly what FormattedNumber does:
    const merged = { ...appCtx, ...formatOptions };
    return formatDisplayString(formatDisplayNumber(value, merged));
  }

  describe("order book values (no formatOptions override)", () => {
    it("size 449.693 → tier 4 → 2 decimals", () => {
      expect(fmtComponent("449.693")).toBe("449.69");
    });

    it("size 300.000000000000000000 → tier 4 → no trailing zeros", () => {
      expect(fmtComponent("300.000000000000000000")).toBe("300");
    });

    it("total 2376.0688 → tier 4 → 2 decimals", () => {
      expect(fmtComponent("2376.0688")).toBe("2,376.07");
    });

    it("total 6441.6278 → tier 4 → 2 decimals", () => {
      expect(fmtComponent("6441.6278")).toBe("6,441.63");
    });

    it("BTC price 84828 → tier 5 → integer", () => {
      expect(fmtComponent("84828")).toBe("84,828");
    });
  });

  describe("with currency override (e.g. price displays)", () => {
    it("price display with currency: USD", () => {
      expect(fmtComponent("84828", { currency: "USD" })).toBe("$84,828");
    });

    it("OI value with currency: USD", () => {
      expect(fmtComponent("13478818", { currency: "USD" })).toBe("$13.48M");
    });
  });

  describe("with fractionDigits override (e.g. order book prices)", () => {
    it("order book price with fractionDigits: 2", () => {
      expect(fmtComponent("84828.12", { fractionDigits: 2 })).toBe("84,828.12");
    });

    it("integer points with fractionDigits: 0", () => {
      expect(fmtComponent("1234", { fractionDigits: 0 })).toBe("1,234");
    });
  });

  describe("stale localStorage options (regression guard)", () => {
    it("ignores unknown keys in merged options", () => {
      // Old localStorage might have maximumTotalDigits or other stale keys.
      // They should be silently ignored by formatDisplayNumber since it
      // destructures only known keys.
      const staleCtx = {
        ...appCtx,
        maximumTotalDigits: 8,
        minimumTotalDigits: 2,
      } as FormatNumberOptions;

      const merged = { ...staleCtx };
      expect(formatDisplayString(formatDisplayNumber("300.000000", merged))).toBe("300");
      expect(formatDisplayString(formatDisplayNumber("84828", merged))).toBe("84,828");
    });

    it("undefined formatOptions does not break merge", () => {
      const merged = { ...appCtx  };
      expect(formatDisplayString(formatDisplayNumber("2376.0688", merged))).toBe("2,376.07");
    });

    it("empty formatOptions does not break merge", () => {
      const merged = { ...appCtx, ...{} };
      expect(formatDisplayString(formatDisplayNumber("2376.0688", merged))).toBe("2,376.07");
    });
  });

  describe("DisplayPart structure for React rendering", () => {
    it("tier 4 produces no fraction parts for integer 300", () => {
      const parts = formatDisplayNumber("300.000000", appCtx);
      const text = parts.map((p) => p.value).join("");
      expect(text).toBe("300");

      const fractionParts = parts.filter((p) => p.type === "fraction");
      expect(fractionParts).toHaveLength(0);
    });

    it("tier 4 produces correct part types for 2,376.07", () => {
      const parts = formatDisplayNumber("2376.0688", appCtx);
      const text = parts.map((p) => p.value).join("");
      expect(text).toBe("2,376.07");

      const fractionParts = parts.filter((p) => p.type === "fraction");
      expect(fractionParts).toHaveLength(1);
      expect(fractionParts[0].value).toBe("07");
    });

    it("tier 5 produces no fraction parts for 84,828", () => {
      const parts = formatDisplayNumber("84828", appCtx);
      const text = parts.map((p) => p.value).join("");
      expect(text).toBe("84,828");

      const fractionParts = parts.filter((p) => p.type === "fraction");
      expect(fractionParts).toHaveLength(0);
    });

    it("tier 6 compact produces suffix M for 13.48M", () => {
      const parts = formatDisplayNumber("13478818", appCtx);
      const text = parts.map((p) => p.value).join("");
      expect(text).toBe("13.48M");

      const suffixParts = parts.filter((p) => p.type === "suffix");
      expect(suffixParts).toHaveLength(1);
      expect(suffixParts[0].value).toBe("M");
    });
  });
});
