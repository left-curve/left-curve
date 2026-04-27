import { describe, it, expect } from "vitest";

import {
  priceFromRoi,
  roiFromPrice,
  type TpslMathInput,
} from "../src/components/dex/lib/tpslMath";

/**
 * Helper: strict equality to ~10 decimals, which is well below any precision
 * we care about for display (UI trims to 2–4 decimals) but tight enough to
 * catch algebraic mistakes.
 */
const EPS = 1e-9;
const approx = (a: number, b: number, eps = EPS) => expect(Math.abs(a - b)).toBeLessThan(eps);

/** Shorthand for building a {@link TpslMathInput} with a few overrides. */
const input = (overrides: Partial<TpslMathInput>): TpslMathInput => ({
  referencePrice: 100,
  leverage: 1,
  isLong: true,
  kind: "tp",
  ...overrides,
});

describe("tpslMath.priceFromRoi", () => {
  // At 1× leverage, ROI% equals the raw price delta %, so the math is the
  // simplest possible baseline. These tests pin down the four quadrants
  // (long/short × tp/sl) and guarantee sign correctness.
  describe("baseline at leverage 1× — ROI equals raw price delta", () => {
    it("long TP: 10% ROI above entry moves price +10%", () => {
      approx(priceFromRoi(10, input({ isLong: true, kind: "tp" })), 110);
    });

    it("long SL: 10% ROI below entry moves price −10%", () => {
      approx(priceFromRoi(10, input({ isLong: true, kind: "sl" })), 90);
    });

    it("short TP: 10% ROI below entry moves price −10%", () => {
      approx(priceFromRoi(10, input({ isLong: false, kind: "tp" })), 90);
    });

    it("short SL: 10% ROI above entry moves price +10%", () => {
      approx(priceFromRoi(10, input({ isLong: false, kind: "sl" })), 110);
    });
  });

  // Leverage is a straight multiplier: at L× leverage, an R% ROI corresponds
  // to an (R / L)% price move. These cases exercise the leverage-amplification
  // path, which is the whole point of this change.
  describe("leverage amplification", () => {
    it("long TP: 100% ROI at 10× leverage needs only 10% price move", () => {
      approx(priceFromRoi(100, input({ leverage: 10, kind: "tp" })), 110);
    });

    it("long TP: 50% ROI at 10× leverage needs only 5% price move", () => {
      approx(priceFromRoi(50, input({ leverage: 10, kind: "tp" })), 105);
    });

    it("long TP: 50% ROI at 20× leverage needs 2.5% price move", () => {
      approx(priceFromRoi(50, input({ leverage: 20, kind: "tp" })), 102.5);
    });

    it("long SL: 100% ROI loss at 10× leverage is 10% price drop (wipes capital)", () => {
      approx(priceFromRoi(100, input({ leverage: 10, kind: "sl" })), 90);
    });

    it("short TP: 100% ROI at 10× leverage is 10% price drop", () => {
      approx(priceFromRoi(100, input({ leverage: 10, isLong: false, kind: "tp" })), 90);
    });

    it("short SL: 100% ROI loss at 10× leverage is 10% price rise", () => {
      approx(priceFromRoi(100, input({ leverage: 10, isLong: false, kind: "sl" })), 110);
    });

    it("long TP at 100× leverage: 100% ROI is just a 1% price move", () => {
      approx(priceFromRoi(100, input({ leverage: 100, kind: "tp" })), 101);
    });
  });

  // Realistic worked examples straight out of the plan document.
  describe("worked examples from the design plan", () => {
    it("entry $100, 10× long, $110 TP → 100% ROI ⇒ priceFromRoi(100) = 110", () => {
      approx(priceFromRoi(100, input({ leverage: 10, kind: "tp" })), 110);
    });

    it("entry $100, 10× long, $105 TP → 50% ROI ⇒ priceFromRoi(50) = 105", () => {
      approx(priceFromRoi(50, input({ leverage: 10, kind: "tp" })), 105);
    });

    it("entry $100, 5× long, $105 TP → 25% ROI ⇒ priceFromRoi(25) = 105", () => {
      approx(priceFromRoi(25, input({ leverage: 5, kind: "tp" })), 105);
    });

    it("entry $100, 10× short, $90 TP → 100% ROI ⇒ priceFromRoi(100) = 90", () => {
      approx(priceFromRoi(100, input({ leverage: 10, isLong: false, kind: "tp" })), 90);
    });
  });

  // A zero ROI always resolves to the reference price, regardless of side,
  // kind, or leverage. The hook uses a non-zero guard before calling this,
  // but the identity should hold for pure-math callers.
  describe("zero ROI identity", () => {
    for (const isLong of [true, false]) {
      for (const kind of ["tp", "sl"] as const) {
        for (const leverage of [1, 5, 10, 50]) {
          it(`isLong=${isLong} kind=${kind} L=${leverage}× → priceFromRoi(0) = ref`, () => {
            approx(priceFromRoi(0, input({ isLong, kind, leverage })), 100);
          });
        }
      }
    }
  });

  // Real assets have a wide price range — make sure no hidden scale
  // assumption is baked in (e.g. assuming ref~$100).
  describe("non-unit reference prices", () => {
    it("BTC-ish: ref 60000, 10× long TP, 50% ROI ⇒ 63000", () => {
      approx(priceFromRoi(50, input({ referencePrice: 60000, leverage: 10, kind: "tp" })), 63000);
    });

    it("micro-asset: ref 0.0001, 10× long TP, 50% ROI ⇒ 0.000105", () => {
      approx(
        priceFromRoi(50, input({ referencePrice: 0.0001, leverage: 10, kind: "tp" })),
        0.000105,
      );
    });
  });
});

describe("tpslMath.roiFromPrice", () => {
  // Mirror of the priceFromRoi baseline: convert an absolute trigger price
  // back to a ROI% and verify the sign/magnitude is correct.
  describe("baseline at leverage 1× — ROI equals raw price delta", () => {
    it("long TP at 110 (entry 100) ⇒ +10% ROI", () => {
      approx(roiFromPrice(110, input({ isLong: true, kind: "tp" })), 10);
    });

    it("long SL at 90 (entry 100) ⇒ +10% loss-ROI", () => {
      approx(roiFromPrice(90, input({ isLong: true, kind: "sl" })), 10);
    });

    it("short TP at 90 (entry 100) ⇒ +10% ROI", () => {
      approx(roiFromPrice(90, input({ isLong: false, kind: "tp" })), 10);
    });

    it("short SL at 110 (entry 100) ⇒ +10% loss-ROI", () => {
      approx(roiFromPrice(110, input({ isLong: false, kind: "sl" })), 10);
    });
  });

  describe("leverage amplification", () => {
    it("long TP at 110 with 10× ⇒ 100% ROI", () => {
      approx(roiFromPrice(110, input({ leverage: 10, kind: "tp" })), 100);
    });

    it("long TP at 105 with 10× ⇒ 50% ROI", () => {
      approx(roiFromPrice(105, input({ leverage: 10, kind: "tp" })), 50);
    });

    it("long TP at 105 with 5× ⇒ 25% ROI", () => {
      approx(roiFromPrice(105, input({ leverage: 5, kind: "tp" })), 25);
    });

    it("short TP at 95 with 10× ⇒ 50% ROI", () => {
      approx(roiFromPrice(95, input({ leverage: 10, isLong: false, kind: "tp" })), 50);
    });

    it("long SL at 95 with 10× ⇒ 50% loss-ROI", () => {
      approx(roiFromPrice(95, input({ leverage: 10, kind: "sl" })), 50);
    });

    it("short SL at 105 with 10× ⇒ 50% loss-ROI", () => {
      approx(roiFromPrice(105, input({ leverage: 10, isLong: false, kind: "sl" })), 50);
    });
  });

  // A "wrong-side" trigger (long TP below entry, or long SL above entry, etc.)
  // would immediately fire. The validation layer rejects these before submit,
  // but the pure math still returns a signed (negative) ROI so callers can
  // detect the situation. The UI hook clamps to 0 at the display layer; we
  // explicitly do NOT clamp here so tests can distinguish "valid zero" from
  // "invalid wrong-side".
  describe("wrong-side triggers return negative ROI", () => {
    it("long TP at 95 (below entry) ⇒ negative ROI", () => {
      expect(roiFromPrice(95, input({ leverage: 10, kind: "tp" }))).toBeLessThan(0);
    });

    it("long SL at 110 (above entry) ⇒ negative ROI", () => {
      expect(roiFromPrice(110, input({ leverage: 10, kind: "sl" }))).toBeLessThan(0);
    });

    it("short TP at 105 (above entry) ⇒ negative ROI", () => {
      expect(roiFromPrice(105, input({ leverage: 10, isLong: false, kind: "tp" }))).toBeLessThan(
        0,
      );
    });

    it("short SL at 95 (below entry) ⇒ negative ROI", () => {
      expect(roiFromPrice(95, input({ leverage: 10, isLong: false, kind: "sl" }))).toBeLessThan(
        0,
      );
    });
  });

  describe("trigger equal to reference ⇒ 0 ROI", () => {
    for (const isLong of [true, false]) {
      for (const kind of ["tp", "sl"] as const) {
        it(`isLong=${isLong} kind=${kind} triggerPrice === referencePrice ⇒ 0`, () => {
          approx(roiFromPrice(100, input({ isLong, kind, leverage: 10 })), 0);
        });
      }
    }
  });
});

describe("tpslMath round-trip invariance", () => {
  // The two functions are exact inverses (modulo float rounding). We sweep
  // a broad grid of (side, kind, leverage, ROI, ref) combinations and
  // verify roiFromPrice(priceFromRoi(r, x), x) === r.
  const LEVERAGES = [1, 2, 5, 10, 25, 50, 100];
  const ROIS = [0.01, 1, 5, 10, 25, 50, 100, 250];
  const REFS = [0.0001, 1, 100, 60000];

  for (const isLong of [true, false]) {
    for (const kind of ["tp", "sl"] as const) {
      for (const leverage of LEVERAGES) {
        for (const roi of ROIS) {
          for (const referencePrice of REFS) {
            const label = `isLong=${isLong} kind=${kind} L=${leverage}× roi=${roi}% ref=${referencePrice}`;
            it(label, () => {
              const cfg = { referencePrice, leverage, isLong, kind } as const;
              const trigger = priceFromRoi(roi, cfg);
              const recovered = roiFromPrice(trigger, cfg);
              // Relative tolerance: at high ROI/leverage the absolute error
              // grows, so compare relatively.
              const err = Math.abs(recovered - roi) / Math.max(Math.abs(roi), 1);
              expect(err).toBeLessThan(1e-9);
            });
          }
        }
      }
    }
  }
});

describe("tpslMath.clampLeverage (via public API)", () => {
  // clampLeverage is internal, so we verify its behavior indirectly:
  // inputs that should clamp to 1 must behave identically to leverage=1.
  const baseline = (roi: number) => priceFromRoi(roi, input({ leverage: 1, kind: "tp" }));

  it("leverage 0 → treated as 1×", () => {
    approx(priceFromRoi(10, input({ leverage: 0, kind: "tp" })), baseline(10));
  });

  it("negative leverage → treated as 1×", () => {
    approx(priceFromRoi(10, input({ leverage: -5, kind: "tp" })), baseline(10));
  });

  it("NaN leverage → treated as 1×", () => {
    approx(priceFromRoi(10, input({ leverage: Number.NaN, kind: "tp" })), baseline(10));
  });

  it("Infinity leverage → treated as 1× (safe fallback rather than zero price move)", () => {
    // Infinity is clamped because isFinite is false. The safe fallback is 1×.
    approx(priceFromRoi(10, input({ leverage: Number.POSITIVE_INFINITY, kind: "tp" })), baseline(10));
  });

  it("fractional leverage (0.5) → treated as 1× (we never amplify below 1)", () => {
    approx(priceFromRoi(10, input({ leverage: 0.5, kind: "tp" })), baseline(10));
  });

  it("leverage exactly 1 → unchanged", () => {
    approx(priceFromRoi(10, input({ leverage: 1, kind: "tp" })), baseline(10));
  });

  // Roundtrip with bad leverage on both sides should still work.
  it("round-trips cleanly when leverage is zero on both sides", () => {
    const cfg = input({ leverage: 0, kind: "tp" });
    approx(roiFromPrice(priceFromRoi(42, cfg), cfg), 42);
  });
});

describe("tpslMath edge cases", () => {
  // A zero or negative reference price is nonsensical (there's no position
  // to compute ROI against). Both functions degrade to safe defaults.
  it("priceFromRoi returns referencePrice for zero reference", () => {
    expect(priceFromRoi(50, input({ referencePrice: 0 }))).toBe(0);
  });

  it("priceFromRoi returns referencePrice for negative reference", () => {
    expect(priceFromRoi(50, input({ referencePrice: -10 }))).toBe(-10);
  });

  it("roiFromPrice returns 0 for zero reference", () => {
    expect(roiFromPrice(100, input({ referencePrice: 0 }))).toBe(0);
  });

  it("roiFromPrice returns 0 for negative reference", () => {
    expect(roiFromPrice(100, input({ referencePrice: -10 }))).toBe(0);
  });

  it("roiFromPrice returns 0 for NaN trigger price", () => {
    expect(roiFromPrice(Number.NaN, input({}))).toBe(0);
  });

  it("roiFromPrice returns 0 for Infinity trigger price", () => {
    expect(roiFromPrice(Number.POSITIVE_INFINITY, input({}))).toBe(0);
  });

  it("priceFromRoi returns reference for NaN ROI", () => {
    expect(priceFromRoi(Number.NaN, input({}))).toBe(100);
  });

  it("priceFromRoi returns reference for Infinity ROI", () => {
    expect(priceFromRoi(Number.POSITIVE_INFINITY, input({}))).toBe(100);
  });
});
