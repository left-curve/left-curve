import { createFileRoute, redirect } from "@tanstack/react-router";
import { coinsBySymbol } from "~/store";

import { z } from "zod";
import { m } from "~/paraglide/messages";

export const Route = createFileRoute("/(app)/_app/earn/pool/$pairSymbols")({
  head: () => ({
    meta: [{ title: `Dango | ${m["poolLiquidity.pool"]()}` }],
  }),
  beforeLoad: async ({ context, params }) => {
    const { client } = context;
    const { pairSymbols } = params;

    const [baseSymbol, quoteSymbol] = pairSymbols.split("-");
    const baseDenom = coinsBySymbol[baseSymbol]?.denom;
    const quoteDenom = coinsBySymbol[quoteSymbol]?.denom;

    const pairParams = await client?.getPair({ baseDenom, quoteDenom }).catch(() => null);
    if (pairParams) return { pair: { baseDenom, quoteDenom, params: pairParams } };

    throw redirect({ to: "/earn/pool/$pairSymbols", params: { pairSymbols: "BTC-USDC" } });
  },
  validateSearch: z.object({
    action: z.enum(["deposit", "withdraw"]).catch("deposit"),
  }),
});
