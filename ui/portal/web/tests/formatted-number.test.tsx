import { describe, it, expect } from "vitest";
import React from "react";
import { render, screen } from "@testing-library/react";
import {
  formatDisplayNumber,
  type FormatNumberOptions,
  type DisplayPart,
} from "@left-curve/dango/utils";

/**
 * Minimal replica of FormattedNumber that uses the real formatDisplayNumber
 * but doesn't require the full AppProvider tree.
 */
const defaultOptions: FormatNumberOptions = { mask: 1, language: "en-US" };

const FormattedNumber: React.FC<{
  number: string | number;
  formatOptions?: Partial<FormatNumberOptions>;
}> = ({ number, formatOptions }) => {
  const parts = formatDisplayNumber(number, { ...defaultOptions, ...formatOptions });
  return (
    <p data-testid="formatted">
      {parts.map((part: DisplayPart, i: number) => {
        if (part.type === "subscript") {
          return <sub key={i}>{part.value}</sub>;
        }
        return <span key={i}>{part.value}</span>;
      })}
    </p>
  );
};

function renderAndGetText(
  value: string | number,
  formatOptions?: Partial<FormatNumberOptions>,
): string {
  const { unmount } = render(
    <FormattedNumber number={value} formatOptions={formatOptions} />,
  );
  const text = screen.getByTestId("formatted").textContent ?? "";
  unmount();
  return text;
}

describe("FormattedNumber component — tier rendering", () => {
  // ── Tier 1: < 0.0001 → subscript notation ──
  describe("Tier 1: < 0.0001 (subscript)", () => {
    it("renders 0.00001234 as subscript notation", () => {
      const text = renderAndGetText("0.00001234");
      // Should be 0.0₄1234 — the text content won't show subscript styling
      // but the DOM structure should contain a <sub> element
      const { container, unmount } = render(
        <FormattedNumber number="0.00001234" />,
      );
      const sub = container.querySelector("sub");
      expect(sub).toBeTruthy();
      expect(sub?.textContent).toBe("4");
      unmount();
    });

    it("renders 0.000000005678 with correct subscript count", () => {
      const { container, unmount } = render(
        <FormattedNumber number="0.000000005678" />,
      );
      const sub = container.querySelector("sub");
      expect(sub).toBeTruthy();
      expect(sub?.textContent).toBe("8");
      unmount();
    });
  });

  // ── Tier 2: 0.0001 ≤ num < 1 → 4 significant digits ──
  describe("Tier 2: 0.0001–1 (4 significant digits)", () => {
    it("renders 0.001234 → 0.001234", () => {
      const text = renderAndGetText("0.001234");
      expect(text).toBe("0.001234");
    });

    it("renders 0.5678 → 0.5678", () => {
      const text = renderAndGetText("0.5678");
      expect(text).toBe("0.5678");
    });

    it("renders 0.0001 → 0.0001", () => {
      const text = renderAndGetText("0.0001");
      expect(text).toBe("0.0001");
    });

    it("does NOT add trailing zeros (0.1 stays 0.1, not 0.1000)", () => {
      const text = renderAndGetText("0.1");
      expect(text).toBe("0.1");
    });
  });

  // ── Tier 3: 1 ≤ num < 100 → up to 4 decimal places ──
  describe("Tier 3: 1–100 (up to 4 decimal places)", () => {
    it("renders integers without trailing zeros", () => {
      expect(renderAndGetText("1")).toBe("1");
      expect(renderAndGetText("2")).toBe("2");
      expect(renderAndGetText("50")).toBe("50");
    });

    it("renders 42.123456 → 42.1235 (rounded)", () => {
      const text = renderAndGetText("42.123456");
      expect(text).toBe("42.1235");
    });

    it("renders 99.9999 → 99.9999", () => {
      const text = renderAndGetText("99.9999");
      expect(text).toBe("99.9999");
    });

    it("renders 1.0000000 (7 decimals input) → 1 (no trailing zeros)", () => {
      const text = renderAndGetText("1.0000000");
      expect(text).toBe("1");
    });

    it("renders 1.5 without padding", () => {
      const text = renderAndGetText("1.5");
      expect(text).toBe("1.5");
    });

    it("renders 50.12345000 → 50.1235", () => {
      const text = renderAndGetText("50.12345000");
      expect(text).toBe("50.1235");
    });
  });

  // ── Tier 4: 100 ≤ num < 10,000 → up to 2 decimal places + grouping ──
  describe("Tier 4: 100–10K (up to 2 decimal places + grouping)", () => {
    it("renders integers without trailing zeros", () => {
      expect(renderAndGetText("100")).toBe("100");
      expect(renderAndGetText("1040")).toBe("1,040");
    });

    it("renders 205.00000 → 205 (no trailing zeros)", () => {
      const text = renderAndGetText("205.00000");
      expect(text).toBe("205");
    });

    it("renders 1234.5678 → 1,234.57", () => {
      const text = renderAndGetText("1234.5678");
      expect(text).toBe("1,234.57");
    });

    it("renders 9999.99 → 9,999.99", () => {
      const text = renderAndGetText("9999.99");
      expect(text).toBe("9,999.99");
    });
  });

  // ── Tier 5: 10K ≤ num < 1M → integer + grouping ──
  describe("Tier 5: 10K–1M (integer + grouping)", () => {
    it("renders 10000 → 10,000", () => {
      const text = renderAndGetText("10000");
      expect(text).toBe("10,000");
    });

    it("renders 66856.000 (3 decimals input) → 66,856", () => {
      // Another pattern from order book violations
      const text = renderAndGetText("66856.000");
      expect(text).toBe("66,856");
    });

    it("renders 999999 → 999,999", () => {
      const text = renderAndGetText("999999");
      expect(text).toBe("999,999");
    });
  });

  // ── Tier 6: ≥ 1M → compact (M/B/T) + 2 decimal places ──
  describe("Tier 6: >= 1M (compact)", () => {
    it("renders 1000000 → 1.00M", () => {
      const text = renderAndGetText("1000000");
      expect(text).toBe("1.00M");
    });

    it("renders 1500000 → 1.50M", () => {
      const text = renderAndGetText("1500000");
      expect(text).toBe("1.50M");
    });

    it("renders 2500000000 → 2.50B", () => {
      const text = renderAndGetText("2500000000");
      expect(text).toBe("2.50B");
    });
  });

  // ── fractionDigits override (used by order book prices) ──
  describe("fractionDigits override", () => {
    it("renders 1234.5678 with fractionDigits: 2 → 1,234.57", () => {
      const text = renderAndGetText("1234.5678", { fractionDigits: 2 });
      expect(text).toBe("1,234.57");
    });

    it("renders 0.5 with fractionDigits: 4 → 0.5000", () => {
      const text = renderAndGetText("0.5", { fractionDigits: 4 });
      expect(text).toBe("0.5000");
    });

    it("renders 99999 with fractionDigits: 0 → 99,999", () => {
      const text = renderAndGetText("99999", { fractionDigits: 0 });
      expect(text).toBe("99,999");
    });
  });

  // ── Currency formatting ──
  describe("Currency (USD)", () => {
    it("renders 1234.5 with currency: USD → $1,234.50", () => {
      const text = renderAndGetText("1234.5", { currency: "USD" });
      expect(text).toBe("$1,234.50");
    });

    it("renders 0 with currency: USD → $0.00", () => {
      const text = renderAndGetText("0", { currency: "USD" });
      expect(text).toBe("$0.00");
    });
  });

  // ── Edge cases ──
  describe("Edge cases", () => {
    it("renders 0 → 0", () => {
      const text = renderAndGetText("0");
      expect(text).toBe("0");
    });

    it("renders negative -42.5 → -42.5", () => {
      const text = renderAndGetText("-42.5");
      expect(text).toBe("-42.5");
    });

    it("renders negative -0.001234 → -0.001234", () => {
      const text = renderAndGetText("-0.001234");
      expect(text).toBe("-0.001234");
    });
  });
});
