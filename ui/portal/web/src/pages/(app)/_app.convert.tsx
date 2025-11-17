import { createFileRoute, redirect } from "@tanstack/react-router";
import { z } from "zod";

import { m } from "@left-curve/foundation/paraglide/messages.js";

const BASE_DENOM = "USDC";
const DEFAULT_QUOTE = "BTC";

export const Route = createFileRoute("/(app)/_app/convert")({
  head: () => ({
    meta: [{ title: `Dango | ${m["applets.convert.title"]()}` }],
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
        to: "/convert",
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
