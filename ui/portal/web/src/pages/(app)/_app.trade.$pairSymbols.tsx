import { createFileRoute, redirect } from "@tanstack/react-router";

import { z } from "zod";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import {
  getDefaultTradePairSymbols,
  getPerpsPairIdFromSymbols,
  normalizeTradePairSymbols,
} from "~/components/dex/helpers/tradePairSymbols";

export const Route = createFileRoute("/(app)/_app/trade/$pairSymbols")({
  head: () => ({
    meta: [{ title: `Dango | ${m["applets.trade.title"]()}` }],
  }),
  beforeLoad: async ({ context, params }) => {
    const { client, config } = context;
    const { pairSymbols } = params;
    const defaultPairSymbols = getDefaultTradePairSymbols(config.chain.name);
    const normalizedPairSymbols = normalizeTradePairSymbols(pairSymbols);

    if (!normalizedPairSymbols) {
      throw redirect({
        to: "/trade/$pairSymbols",
        params: { pairSymbols: defaultPairSymbols },
      });
    }

    if (normalizedPairSymbols !== pairSymbols) {
      throw redirect({
        to: "/trade/$pairSymbols",
        params: { pairSymbols: normalizedPairSymbols },
      });
    }

    const pairId = getPerpsPairIdFromSymbols(normalizedPairSymbols);
    if (!pairId) {
      throw redirect({
        to: "/trade/$pairSymbols",
        params: { pairSymbols: defaultPairSymbols },
      });
    }

    const pair = await client.getPerpsPairParam({ pairId }).catch(() => null);
    if (!pair)
      throw redirect({
        to: "/trade/$pairSymbols",
        params: { pairSymbols: defaultPairSymbols },
      });
  },
  validateSearch: z.object({
    order_type: z.enum(["limit", "market"]).default("market"),
    action: z.enum(["buy", "sell"]).default("buy"),
  }),
});
