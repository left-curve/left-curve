import { createFileRoute, redirect } from "@tanstack/react-router";
import { getDefaultTradePairSymbols } from "~/components/dex/helpers/tradePairSymbols";

export const Route = createFileRoute("/(app)/_app/trade/")({
  beforeLoad: async ({ context }) => {
    const { config } = context;

    throw redirect({
      to: "/trade/$pairSymbols",
      params: { pairSymbols: getDefaultTradePairSymbols(config.chain.name) },
    });
  },
});
