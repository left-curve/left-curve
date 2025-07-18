import { createFileRoute } from "@tanstack/react-router";
import { z } from "zod";
import { coinsBySymbol } from "~/store";

import { m } from "~/paraglide/messages";

const BASE_DENOM = "USDC";
const DEFAULT_QUOTE = "BTC";

export const Route = createFileRoute("/(app)/_app/swap")({
  head: () => ({
    meta: [{ title: `Dango | ${m["applets.1.title"]()}` }],
  }),
  validateSearch: z
    .object({
      from: z.string(),
      to: z.string(),
    })
    .superRefine(({ from, to }, ctx) => {
      const fromCoin = coinsBySymbol[from];
      const toCoin = coinsBySymbol[to];

      if (
        !fromCoin ||
        !toCoin ||
        !(to === BASE_DENOM || from === BASE_DENOM) ||
        (to === BASE_DENOM && from === BASE_DENOM)
      ) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          message: "Invalid pair",
        });
      }
    })
    .catch({ from: BASE_DENOM, to: DEFAULT_QUOTE }),
});
