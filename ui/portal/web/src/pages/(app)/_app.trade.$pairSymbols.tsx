import { createFileRoute, redirect } from "@tanstack/react-router";

import { z } from "zod";
import { m } from "@left-curve/foundation/paraglide/messages.js";

export const Route = createFileRoute("/(app)/_app/trade/$pairSymbols")({
  head: () => ({
    meta: [{ title: `Dango | ${m["applets.trade.title"]()}` }],
  }),
  beforeLoad: async ({ context, params, search }) => {
    const { client, config } = context;
    const { coins } = config;
    const { type } = search;

    const { pairSymbols } = params;
    const [baseSymbol, quoteSymbol] = pairSymbols.split("-");
    const baseDenom = coins.bySymbol[baseSymbol]?.denom;

    if (type === "spot") {
      const quoteDenom = coins.bySymbol[quoteSymbol]?.denom;
      const pair = await client.getPair({ baseDenom, quoteDenom }).catch(() => null);
      if (!pair)
        throw redirect({
          to: "/trade/$pairSymbols",
          params: { pairSymbols: "ETH-USDC" },
        });
    }

    if (type === "perps") {
      const pairId = `perp/${baseSymbol.toLowerCase()}${quoteSymbol.toLowerCase()}`;
      const pair = await client.getPerpsPairParam({ pairId }).catch(() => null);
      if (!pair)
        throw redirect({
          to: "/trade/$pairSymbols",
          params: { pairSymbols: "ETH-USD" },
          search: { type: "perps" },
        });
    }
  },
  validateSearch: z.object({
    order_type: z.enum(["limit", "market"]).default("market"),
    action: z.enum(["buy", "sell"]).default("buy"),
    type: z.enum(["spot", "perps"]).default("spot"),
  }),
});
