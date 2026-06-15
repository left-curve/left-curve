import { afterEach, describe, expect, it } from "vitest";

import { CoinStore } from "../../../store/src/stores/coinStore";

import type { NativeCoin } from "../../../store/src/types/coin";

const btcCoin = {
  decimals: 8,
  denom: "bridge/btc",
  logoURI: "/images/coins/bitcoin.svg",
  name: "Bitcoin",
  symbol: "BTC",
  type: "native",
} satisfies NativeCoin;

const usdcCoin = {
  decimals: 6,
  denom: "bridge/usdc",
  logoURI: "/images/coins/usdc.svg",
  name: "USD Coin",
  symbol: "USDC",
  type: "native",
} satisfies NativeCoin;

const ethCoin = {
  decimals: 18,
  denom: "bridge/eth",
  logoURI: "/images/coins/eth.svg",
  name: "Ether",
  symbol: "ETH",
  type: "native",
} satisfies NativeCoin;

describe("coin store", () => {
  afterEach(() => {
    CoinStore.setState({
      byDenom: {},
      bySymbol: {},
    });
  });

  it("indexes configured native coins by denom and symbol", () => {
    CoinStore.getState().setCoins({
      [btcCoin.denom]: btcCoin,
      [usdcCoin.denom]: usdcCoin,
    });

    expect(CoinStore.getState().byDenom).toEqual({
      "bridge/btc": btcCoin,
      "bridge/usdc": usdcCoin,
    });
    expect(CoinStore.getState().bySymbol).toEqual({
      BTC: btcCoin,
      USDC: usdcCoin,
    });
    expect(CoinStore.getState().getCoinInfo("bridge/usdc")).toBe(usdcCoin);
  });

  it("replaces stale symbol indexes when runtime config coins change", () => {
    CoinStore.getState().setCoins({
      [btcCoin.denom]: btcCoin,
      [usdcCoin.denom]: usdcCoin,
    });

    CoinStore.getState().setCoins({
      [ethCoin.denom]: ethCoin,
    });

    expect(CoinStore.getState().byDenom).toEqual({
      "bridge/eth": ethCoin,
    });
    expect(CoinStore.getState().bySymbol).toEqual({
      ETH: ethCoin,
    });
    expect(CoinStore.getState().bySymbol).not.toHaveProperty("BTC");
    expect(CoinStore.getState().bySymbol).not.toHaveProperty("USDC");
  });

  it("returns deterministic native fallback metadata for unknown non-LP denoms", () => {
    expect(CoinStore.getState().getCoinInfo("airdrop/bonus")).toEqual({
      decimals: 0,
      denom: "airdrop/bonus",
      name: "airdrop/bonus",
      symbol: "AIRDROP/BONUS",
      type: "native",
    });
  });

  it("derives LP metadata from configured base and quote coins", () => {
    CoinStore.getState().setCoins({
      [btcCoin.denom]: btcCoin,
      [usdcCoin.denom]: usdcCoin,
    });

    expect(CoinStore.getState().getCoinInfo("dex/pool/btc/usdc")).toEqual({
      base: btcCoin,
      decimals: 0,
      denom: "dex/pool/btc/usdc",
      name: "BTC-USDC Liquidity Shares",
      quote: usdcCoin,
      symbol: "BTC-USDC LP",
      type: "lp",
    });
  });
});
