import { createFileRoute } from "@tanstack/react-router";
import { z } from "zod";

export const Route = createFileRoute("/(app)/_app/swap")({
  validateSearch: z.object({
    from: z.string().catch("USDC"),
    to: z.string().catch("BTC"),
  }),
});
