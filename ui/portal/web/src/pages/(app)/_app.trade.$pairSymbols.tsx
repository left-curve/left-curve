import { createFileRoute, redirect } from "@tanstack/react-router";

import { z } from "zod";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import {
  getDefaultTradePairSymbols,
  getPerpsPairId,
  parseTradePairSymbols,
} from "~/components/dex/helpers/tradePairSymbols";

export const Route = createFileRoute("/(app)/_app/trade/$pairSymbols")({
  head: () => ({
    meta: [{ title: `Dango | ${m["applets.trade.title"]()}` }],
  }),
  beforeLoad: async ({ context, params }) => {
    const { client, config } = context;
    const { pairSymbols } = params;
    const defaultPairSymbols = getDefaultTradePairSymbols(config.chain.name);
    const fallback = {
      to: "/trade/$pairSymbols",
      params: { pairSymbols: defaultPairSymbols },
    } as const;

    const parsed = parseTradePairSymbols(pairSymbols);
    if (!parsed) throw redirect(fallback);

    const normalizedPairSymbols = `${parsed.baseSymbol}-${parsed.quoteSymbol}`;
    if (normalizedPairSymbols !== pairSymbols) {
      throw redirect({ ...fallback, params: { pairSymbols: normalizedPairSymbols } });
    }

    const pair = await client.getPerpsPairParam({ pairId: getPerpsPairId(parsed) }).catch(() => null);
    if (!pair) throw redirect(fallback);
  },
  validateSearch: z.object({
    order_type: z.enum(["limit", "market"]).default("market"),
    action: z.enum(["buy", "sell"]).default("buy"),
  }),
});
