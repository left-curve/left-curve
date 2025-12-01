import { createFileRoute } from "@tanstack/react-router";

import { z } from "zod";
import { m } from "@left-curve/foundation/paraglide/messages.js";

export const Route = createFileRoute("/(app)/_app/bridge")({
  head: () => ({
    meta: [{ title: `Dango | ${m["bridge.title"]()}` }],
  }),
  validateSearch: z.object({
    action: z.enum(["deposit", "withdraw"]).catch("deposit"),
  }),
});
