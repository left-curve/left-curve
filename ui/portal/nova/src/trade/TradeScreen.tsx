import { useRef } from "react";
import { View } from "react-native";
import {
  useConfig,
  TradePairStore,
  useAllPerpsPairStats,
  useAllPairStats,
  usePerpsPairState,
  usePerpsState,
  useOraclePrices,
  useLivePerpsTradesState,
} from "@left-curve/store";
import { useParams, useSearch } from "@tanstack/react-router";
import { MarketBar } from "./MarketBar";
import { Chart } from "./Chart";
import { OrderBook } from "./OrderBook";
import { OrderForm } from "./OrderForm";
import { TradeHistory } from "./TradeHistory";

function TradeSubscriptions() {
  const mode = TradePairStore((s) => s.mode);

  useLivePerpsTradesState({ subscribe: mode === "perps" });
  usePerpsPairState({ subscribe: mode === "perps" });
  useOraclePrices({ subscribe: true });
  useAllPairStats();
  useAllPerpsPairStats();
  usePerpsState();

  return null;
}

export function TradeScreen() {
  const { coins } = useConfig();
  const { pairSymbols } = useParams({ from: "/trade/$pairSymbols" });
  const { type } = useSearch({ from: "/trade/$pairSymbols" });
  const lastPair = useRef("");

  const [baseSymbol, quoteSymbol] = pairSymbols.split("-");
  const baseDenom = coins.bySymbol[baseSymbol]?.denom;
  const quoteDenom = type === "perps" ? "usd" : coins.bySymbol[quoteSymbol]?.denom;

  if (!baseDenom || !quoteDenom) return null;

  const pairKey = `${baseDenom}:${quoteDenom}:${type}`;
  if (lastPair.current !== pairKey) {
    lastPair.current = pairKey;
    TradePairStore.getState().setPair({ baseDenom, quoteDenom }, type);
  }

  return (
    <View
      className="p-2"
      style={{
        display: "grid" as never,
        gridTemplateColumns: "1fr 300px 280px",
        gridTemplateRows: "auto 1fr 320px",
        gap: 6,
        height: "calc(100vh - 48px)",
      }}
    >
      <TradeSubscriptions />
      <View style={{ gridColumn: "1 / -1" }}>
        <MarketBar />
      </View>

      <View style={{ gridColumn: "1 / 2", gridRow: "2 / 3" }}>
        <Chart />
      </View>

      <View style={{ gridColumn: "2 / 3", gridRow: "2 / 3", minHeight: 0 }}>
        <OrderBook />
      </View>

      <View style={{ gridColumn: "3 / 4", gridRow: "2 / 4" }}>
        <OrderForm />
      </View>

      <View style={{ gridColumn: "1 / 3", gridRow: "3 / 4", minHeight: 0 }}>
        <TradeHistory />
      </View>
    </View>
  );
}
