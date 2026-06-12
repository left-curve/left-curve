import { createFileRoute, redirect } from "@tanstack/react-router";
import { MarketPair } from "@left-curve/foundation/market-pair";

import { z } from "zod";
import { m } from "@left-curve/foundation/paraglide/messages.js";

export const Route = createFileRoute("/(app)/_app/trade/$ticker")({
  head: () => ({
    meta: [{ title: `Dango | ${m["applets.trade.title"]()}` }],
  }),
  beforeLoad: async ({ context, params }) => {
    const { client } = context;
    const { ticker } = params;
    const fallback = {
      to: "/trade/$ticker",
      params: { ticker: MarketPair.default.ticker },
    } as const;

    const pair = MarketPair.tryFromTicker(ticker);
    if (!pair) throw redirect(fallback);

    if (pair.ticker !== ticker) {
      throw redirect({ ...fallback, params: { ticker: pair.ticker } });
    }

    if (pair === MarketPair.default) return;

    const pairParam = await client.getPerpsPairParam({ pairId: pair.id }).catch(() => null);
    if (!pairParam) throw redirect(fallback);
  },
  validateSearch: z.object({
    order_type: z.enum(["limit", "market"]).default("market"),
    action: z.enum(["buy", "sell"]).default("buy"),
  }),
});
