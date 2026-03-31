import { createFileRoute, redirect } from "@tanstack/react-router";

export const Route = createFileRoute("/(app)/_app/trade/")({
  beforeLoad: async ({ context }) => {
    const { config } = context;
    const isDevnet = config.chain.id === "dev-9";

    throw redirect({
      to: "/trade/$pairSymbols",
      params: { pairSymbols: isDevnet ? "ETH-USD" : "BTC-USD" },
      search: { type: "perps" },
    });
  },
});
