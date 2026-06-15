import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import { TPSLPositionInfo } from "../src/components/modals/TPSLPositionInfo";

vi.mock("@left-curve/foundation", async (importOriginal) => ({
  ...(await importOriginal<typeof import("@left-curve/foundation")>()),
  useApp: () => ({
    settings: {
      formatNumberOptions: {
        currency: "USD",
        language: "en-US",
      },
    },
  }),
}));

function getByTextContent(text: string, tagName = "P") {
  return screen.getByText((_, node) => node?.tagName === tagName && node.textContent === text);
}

describe("TPSLPositionInfo", () => {
  afterEach(() => {
    cleanup();
  });

  it("renders long position context with success styling and formatted backend prices", () => {
    render(
      <TPSLPositionInfo absSize={1.25} entryPrice="65000" isLong markPrice="66500" symbol="BTC" />,
    );

    expect(screen.getByText(m["modals.tpsl.coin"]())).toBeInTheDocument();
    expect(screen.getByText("BTC")).toBeInTheDocument();
    expect(screen.getByText(m["modals.tpsl.position"]())).toBeInTheDocument();
    expect(getByTextContent(`${m["modals.tpsl.long"]()} 1.25 BTC`)).toHaveClass(
      "text-utility-success-600",
    );
    expect(screen.getByText(m["modals.tpsl.entryPrice"]())).toBeInTheDocument();
    expect(screen.getByText(m["modals.tpsl.markPrice"]())).toBeInTheDocument();
    expect(getByTextContent("$65,000")).toBeInTheDocument();
    expect(getByTextContent("$66,500")).toBeInTheDocument();
  });

  it("renders short position context with error styling", () => {
    render(
      <TPSLPositionInfo
        absSize={2}
        entryPrice="3200"
        isLong={false}
        markPrice="3100"
        symbol="ETH"
      />,
    );

    expect(screen.getByText("ETH")).toBeInTheDocument();
    expect(getByTextContent(`${m["modals.tpsl.short"]()} 2 ETH`)).toHaveClass(
      "text-utility-error-600",
    );
    expect(getByTextContent("$3,200.00")).toBeInTheDocument();
    expect(getByTextContent("$3,100.00")).toBeInTheDocument();
  });
});
