import { createFileRoute, redirect } from "@tanstack/react-router";

import { z } from "zod";
import { m } from "~/paraglide/messages";

export const Route = createFileRoute("/(app)/_app/earn/pool/$pairSymbols")({
  head: () => ({
    meta: [{ title: `Dango | ${m["poolLiquidity.pool"]()}` }],
  }),
  beforeLoad: async ({ context, params }) => {
    const { client, config } = context;
    const { pairSymbols } = params;
    const { coins } = config;

    const [baseSymbol, quoteSymbol] = pairSymbols.split("-");
    const baseDenom = coins.bySymbol[baseSymbol]?.denom;
    const quoteDenom = coins.bySymbol[quoteSymbol]?.denom;

    const pairParams = await client?.getPair({ baseDenom, quoteDenom }).catch(() => null);
    if (pairParams) return { pair: { baseDenom, quoteDenom, params: pairParams } };

    throw redirect({ to: "/earn/pool/$pairSymbols", params: { pairSymbols: "BTC-USDC" } });
  },
  validateSearch: z.object({
    action: z.enum(["deposit", "withdraw"]).catch("deposit"),
  }),
});
