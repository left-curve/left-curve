import { createFileRoute, redirect } from "@tanstack/react-router";

import { z } from "zod";
import { m } from "~/paraglide/messages";
import { coinsBySymbol } from "~/store";

export const Route = createFileRoute("/(app)/_app/trade/$pairSymbols")({
  head: () => ({
    meta: [{ title: `Dango | ${m["applets.0.title"]()}` }],
  }),
  beforeLoad: async ({ context, params }) => {
    const { client } = context;
    const { pairSymbols } = params;
    const [baseSymbol, quoteSymbol] = pairSymbols.split("-");
    const baseDenom = coinsBySymbol[baseSymbol]?.denom;
    const quoteDenom = coinsBySymbol[quoteSymbol]?.denom;

    const pair = await client?.getPair({ baseDenom, quoteDenom }).catch(() => null);
    if (!pair) throw redirect({ to: "/trade/$pairSymbols", params: { pairSymbols: "BTC-USDC" } });
  },
  validateSearch: z.object({
    action: z.enum(["buy", "sell"]).default("buy"),
  }),
});
