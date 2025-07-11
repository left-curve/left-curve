import { createFileRoute } from "@tanstack/react-router";

import { z } from "zod";
import { m } from "~/paraglide/messages";

export const Route = createFileRoute("/(auth)/_auth/signin")({
  head: () => ({
    meta: [{ title: `Dango | ${m["common.signin"]()}` }],
  }),
  validateSearch: z.object({
    socketId: z.string().optional(),
  }),
});
