import { createFileRoute, redirect } from "@tanstack/react-router";
import { coinsBySymbol } from "~/store";

export const Route = createFileRoute("/(app)/_app/trade/$pairSymbols")({
  beforeLoad: async ({ context, params }) => {
    const { client } = context;
    const { pairSymbols } = params;
    const [baseSymbol, quoteSymbol] = pairSymbols.split("-");
    const baseDenom = coinsBySymbol[baseSymbol]?.denom;
    const quoteDenom = coinsBySymbol[quoteSymbol]?.denom;

    const pair = await client?.getPair({ baseDenom, quoteDenom }).catch(() => null);
    if (!pair) throw redirect({ to: "/trade/$pairSymbols", params: { pairSymbols: "BTC-USDC" } });
  },
});
