import { createFileRoute, redirect } from "@tanstack/react-router";
import { MarketPair } from "@left-curve/foundation/market-pair";

export const Route = createFileRoute("/(app)/_app/trade/")({
  beforeLoad: async () => {
    throw redirect({
      to: "/trade/$ticker",
      params: { ticker: MarketPair.default.ticker },
    });
  },
});
