import { createFileRoute } from "@tanstack/react-router";

import { z } from "zod";
import { m } from "@left-curve/foundation/paraglide/messages.js";

export const Route = createFileRoute("/(app)/_app/transfer")({
  head: () => ({
    meta: [{ title: `Dango | ${m["sendAndReceive.title"]()}` }],
  }),
  validateSearch: z.object({
    action: z.enum(["send", "receive"]).catch("send"),
  }),
});
