import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import type React from "react";

import { AssetCard } from "../src/components/foundation/AssetCard";

const assetCardMocks = vi.hoisted(() => ({
  getCoinInfo: vi.fn(),
  getPrice: vi.fn(),
}));

vi.mock("framer-motion", () => ({
  motion: {
    div: ({
      children,
      layout: _layout,
      ...props
    }: React.HTMLAttributes<HTMLDivElement> & { layout?: unknown }) => (
      <div {...props}>{children}</div>
    ),
  },
}));

vi.mock("@left-curve/foundation", async (importOriginal) => ({
  ...(await importOriginal<typeof import("@left-curve/foundation")>()),
  useApp: () => ({
    settings: {
      formatNumberOptions: {
        language: "en-US",
        mask: 1,
      },
    },
  }),
}));

vi.mock("@left-curve/store", () => ({
  useConfig: () => ({
    coins: {
      getCoinInfo: assetCardMocks.getCoinInfo,
    },
  }),
  usePrices: () => ({
    getPrice: assetCardMocks.getPrice,
  }),
}));

describe("AssetCard", () => {
  beforeEach(() => {
    assetCardMocks.getPrice.mockImplementation((amount: string) => {
      if (amount === "12.345") return "24.69";
      if (amount === "2.5") return "100";
      return "0";
    });
    assetCardMocks.getCoinInfo.mockImplementation((denom: string) => {
      if (denom === "bridge/usdc") {
        return {
          decimals: 6,
          denom,
          logoURI: "/images/coins/usdc.svg",
          name: "USD Coin",
          symbol: "USDC",
          type: "coin",
        };
      }

      if (denom === "dex/pool/btc-usdc") {
        return {
          base: {
            logoURI: "/images/coins/btc.svg",
            symbol: "BTC",
          },
          decimals: 6,
          denom,
          name: "BTC / USDC Pool",
          quote: {
            logoURI: "/images/coins/usdc.svg",
            symbol: "USDC",
          },
          symbol: "BTC-USDC LP",
          type: "lp",
        };
      }

      throw new Error(`Unexpected denom: ${denom}`);
    });
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("renders spot coins from backend denoms and prices the human amount", () => {
    const { container } = render(<AssetCard coin={{ amount: "12345000", denom: "bridge/usdc" }} />);

    expect(assetCardMocks.getCoinInfo).toHaveBeenCalledWith("bridge/usdc");
    expect(assetCardMocks.getPrice).toHaveBeenCalledWith("12.345", "bridge/usdc");
    expect(screen.getByRole("img", { name: "bridge/usdc" })).toHaveAttribute(
      "src",
      "/images/coins/usdc.svg",
    );
    expect(screen.getByText("USDC")).toBeInTheDocument();
    expect(screen.getByText("USD Coin")).toBeInTheDocument();
    expect(container).toHaveTextContent("$24.69");
    expect(container).toHaveTextContent("12.345");
  });

  it("renders LP coins with both pair assets while preserving price conversion", () => {
    const { container } = render(
      <AssetCard coin={{ amount: "2500000", denom: "dex/pool/btc-usdc" }} />,
    );

    expect(assetCardMocks.getCoinInfo).toHaveBeenCalledWith("dex/pool/btc-usdc");
    expect(assetCardMocks.getPrice).toHaveBeenCalledWith("2.5", "dex/pool/btc-usdc");
    expect(screen.getByRole("img", { name: "BTC" })).toHaveAttribute(
      "src",
      "/images/coins/btc.svg",
    );
    expect(screen.getByRole("img", { name: "USDC" })).toHaveAttribute(
      "src",
      "/images/coins/usdc.svg",
    );
    expect(screen.getByText("BTC-USDC LP")).toBeInTheDocument();
    expect(screen.getByText("BTC / USDC Pool")).toBeInTheDocument();
    expect(container).toHaveTextContent("$100.00");
    expect(container).toHaveTextContent("2.5");
  });

  it("renders perps margin and vault balances from the shared margin asset metadata", () => {
    const { container } = render(
      <>
        <AssetCard.Perp amount="42.5" />
        <AssetCard.Vault shares="123.456" usdValue="1500" />
      </>,
    );

    expect(screen.getByRole("img", { name: "USD" })).toHaveAttribute(
      "src",
      "/images/coins/usd.svg",
    );
    expect(screen.getByText("US Dollar")).toBeInTheDocument();
    expect(screen.getByRole("img", { name: "DLP" })).toHaveAttribute(
      "src",
      "/images/coins/usd.svg",
    );
    expect(screen.getByText(m["vaultLiquidity.title"]())).toBeInTheDocument();
    expect(container).toHaveTextContent("42.5");
    expect(container).toHaveTextContent("$1,500.00");
    expect(container).toHaveTextContent("123.46");
  });
});
