import { createFileRoute, redirect } from "@tanstack/react-router";

import { z } from "zod";
import { m } from "~/paraglide/messages";

export const Route = createFileRoute("/(app)/_app/trade/$pairSymbols")({
  head: () => ({
    meta: [{ title: `Dango | ${m["applets.0.title"]()}` }],
  }),
  beforeLoad: async ({ context, params }) => {
    const { client, config } = context;
    const { coins } = config;
    const { pairSymbols } = params;
    const [baseSymbol, quoteSymbol] = pairSymbols.split("-");
    const baseDenom = coins.bySymbol[baseSymbol]?.denom;
    const quoteDenom = coins.bySymbol[quoteSymbol]?.denom;

    const pair = await client?.getPair({ baseDenom, quoteDenom }).catch(() => null);
    if (!pair)
      throw redirect({
        to: "/trade/$pairSymbols",
        params: { pairSymbols: "BTC-USDC" },
      });
  },
  validateSearch: z.object({
    order_type: z.enum(["limit", "market"]).default("market"),
    action: z.enum(["buy", "sell"]).default("buy"),
  }),
});
