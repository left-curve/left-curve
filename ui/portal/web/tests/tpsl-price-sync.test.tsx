import { act, renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { useTPSLPriceSync } from "../src/components/dex/hooks/useTPSLPriceSync";

type TPSLSyncProps = {
  referencePrice?: number;
  leverage?: number;
  isBuyDirection?: boolean;
  enabled?: boolean;
};

function renderTPSLSync({
  referencePrice = 100,
  leverage = 5,
  isBuyDirection = true,
  enabled = true,
}: TPSLSyncProps = {}) {
  const setValue = vi.fn();
  const rendered = renderHook(
    (props: Required<TPSLSyncProps>) =>
      useTPSLPriceSync({
        ...props,
        setValue,
      }),
    {
      initialProps: {
        enabled,
        isBuyDirection,
        leverage,
        referencePrice,
      },
    },
  );

  return {
    ...rendered,
    setValue,
  };
}

describe("useTPSLPriceSync", () => {
  it("syncs long take-profit and stop-loss price edits into leveraged ROI percentages", () => {
    const { result, setValue } = renderTPSLSync();

    act(() => {
      result.current.onTpPriceChange("110");
    });
    expect(setValue).toHaveBeenNthCalledWith(1, "tpPrice", "110");
    expect(setValue).toHaveBeenNthCalledWith(2, "tpPercent", "50");

    setValue.mockClear();
    act(() => {
      result.current.onSlPriceChange("90");
    });
    expect(setValue).toHaveBeenNthCalledWith(1, "slPrice", "90");
    expect(setValue).toHaveBeenNthCalledWith(2, "slPercent", "50");
  });

  it("syncs long ROI percentage edits back to trigger prices", () => {
    const { result, setValue } = renderTPSLSync();

    act(() => {
      result.current.onTpPercentChange("25");
    });
    expect(setValue).toHaveBeenNthCalledWith(1, "tpPercent", "25");
    expect(setValue).toHaveBeenNthCalledWith(2, "tpPrice", "105");

    setValue.mockClear();
    act(() => {
      result.current.onSlPercentChange("25");
    });
    expect(setValue).toHaveBeenNthCalledWith(1, "slPercent", "25");
    expect(setValue).toHaveBeenNthCalledWith(2, "slPrice", "95");
  });

  it("uses the opposite price direction for short take-profit and stop-loss triggers", () => {
    const { result, setValue } = renderTPSLSync({
      isBuyDirection: false,
      leverage: 4,
    });

    act(() => {
      result.current.onTpPriceChange("95");
    });
    expect(setValue).toHaveBeenNthCalledWith(1, "tpPrice", "95");
    expect(setValue).toHaveBeenNthCalledWith(2, "tpPercent", "20");

    setValue.mockClear();
    act(() => {
      result.current.onSlPercentChange("20");
    });
    expect(setValue).toHaveBeenNthCalledWith(1, "slPercent", "20");
    expect(setValue).toHaveBeenNthCalledWith(2, "slPrice", "105");
  });

  it("preserves the edited field and clamps wrong-side trigger percentages for display", () => {
    const { result, setValue } = renderTPSLSync();

    act(() => {
      result.current.onTpPriceChange("90");
    });

    expect(setValue).toHaveBeenNthCalledWith(1, "tpPrice", "90");
    expect(setValue).toHaveBeenNthCalledWith(2, "tpPercent", "0");
  });

  it("clears the paired field for zero values and skips paired updates when disabled", () => {
    const { result, setValue, rerender } = renderTPSLSync();

    act(() => {
      result.current.onTpPercentChange("0");
    });
    expect(setValue).toHaveBeenNthCalledWith(1, "tpPercent", "0");
    expect(setValue).toHaveBeenNthCalledWith(2, "tpPrice", "");

    setValue.mockClear();
    rerender({
      enabled: false,
      isBuyDirection: true,
      leverage: 5,
      referencePrice: 100,
    });

    act(() => {
      result.current.onSlPriceChange("90");
    });
    expect(setValue).toHaveBeenCalledTimes(1);
    expect(setValue).toHaveBeenCalledWith("slPrice", "90");
  });
});
