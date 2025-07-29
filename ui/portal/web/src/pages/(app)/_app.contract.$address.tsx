import { createFileRoute } from "@tanstack/react-router";

import { m } from "~/paraglide/messages";

export const Route = createFileRoute("/(app)/_app/contract/$address")({
  head: () => ({
    meta: [{ title: `Dango | ${m["explorer.contracts.title"]()}` }],
  }),
});
