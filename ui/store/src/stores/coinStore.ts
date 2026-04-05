import { create } from "zustand";

import type { Denom } from "@left-curve/dango/types";
import type { AnyCoin, NativeCoin } from "../types/coin.js";

export type CoinStore = {
  byDenom: Record<Denom, NativeCoin>;
  bySymbol: Record<string, NativeCoin>;
  setCoins: (coins: Record<Denom, NativeCoin>) => void;
  getCoinInfo: (denom: Denom) => AnyCoin;
};

export const CoinStore = create<CoinStore>((set, get) => ({
  byDenom: {},
  bySymbol: {},
  setCoins: (coins: Record<Denom, NativeCoin>) => {
    const bySymbol = Object.values(coins).reduce((acc, coin) => {
      acc[coin.symbol] = coin;
      return acc;
    }, Object.create({}));
    set((s) => ({ ...s, byDenom: coins, bySymbol }));
  },
  getCoinInfo: (denom: Denom) => {
    const { byDenom } = get();
    const coin = byDenom[denom];
    if (coin) return coin;
    if (!coin && !denom.includes("dex")) {
      return {
        type: "native",
        symbol: denom.toUpperCase(),
        name: denom,
        denom,
        decimals: 0,
      };
    }
    const [_, __, baseDenom, quoteDenom] = denom.split("/");
    const coinsArray = Object.values(byDenom);
    const baseCoin = coinsArray.find((x) => x.denom.includes(baseDenom))!;
    const quoteCoin = coinsArray.find((x) => x.denom.includes(quoteDenom))!;

    return {
      type: "lp",
      symbol: `${baseCoin.symbol}-${quoteCoin.symbol} LP`,
      name: `${baseCoin.symbol}-${quoteCoin.symbol} Liquidity Shares`,
      denom,
      decimals: 0,
      base: baseCoin,
      quote: quoteCoin,
    };
  },
}));
