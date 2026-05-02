import { createRoute, redirect } from "@tanstack/react-router";
import { z } from "zod";
import { Route as rootRoute } from "./__root";
import { TradeScreen } from "../trade";

export const tradeIndexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/trade",
  beforeLoad: ({ context }) => {
    const isDevnet = context.config.chain.name === "Devnet";
    throw redirect({
      to: "/trade/$pairSymbols",
      params: { pairSymbols: isDevnet ? "ETH-USD" : "BTC-USD" },
      search: { type: "perps" },
    });
  },
});

export const tradeRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/trade/$pairSymbols",
  component: TradeScreen,
  validateSearch: z.object({
    order_type: z.enum(["limit", "market"]).default("market"),
    action: z.enum(["buy", "sell"]).default("buy"),
    type: z.enum(["spot", "perps"]).default("perps"),
  }),
});
