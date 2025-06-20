import { createFileRoute, redirect } from "@tanstack/react-router";

export const Route = createFileRoute("/(app)/_app/trade/")({
  beforeLoad: async () => {
    throw redirect({
      to: "/trade/$pairSymbols",
      params: { pairSymbols: "BTC-USDC" },
    });
  },
});
