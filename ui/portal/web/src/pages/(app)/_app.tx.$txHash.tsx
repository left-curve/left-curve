import { createFileRoute } from "@tanstack/react-router";

import { m } from "~/paraglide/messages";

export const Route = createFileRoute("/(app)/_app/tx/$txHash")({
  head: () => ({
    meta: [{ title: `Dango | ${m["explorer.txs.title"]()}` }],
  }),
});
