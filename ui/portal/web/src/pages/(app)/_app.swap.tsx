import { createFileRoute, redirect } from "@tanstack/react-router";
import { z } from "zod";

import { m } from "~/paraglide/messages";

const BASE_DENOM = "USDC";
const DEFAULT_QUOTE = "BTC";

export const Route = createFileRoute("/(app)/_app/swap")({
  head: () => ({
    meta: [{ title: `Dango | ${m["applets.1.title"]()}` }],
  }),
  beforeLoad: async ({ context, search }) => {
    const { config } = context;
    const { coins } = config;
    const { from = BASE_DENOM, to = DEFAULT_QUOTE } = search;

    const fromCoin = coins.bySymbol[from];
    const toCoin = coins.bySymbol[to];
    if (
      !fromCoin ||
      !toCoin ||
      !(to === BASE_DENOM || from === BASE_DENOM) ||
      (to === BASE_DENOM && from === BASE_DENOM)
    ) {
      throw redirect({
        to: "/swap",
        search: { from: BASE_DENOM, to: DEFAULT_QUOTE },
      });
    }
  },
  validateSearch: z
    .object({
      from: z.string(),
      to: z.string(),
    })
    .catch({ from: BASE_DENOM, to: DEFAULT_QUOTE }),
});
