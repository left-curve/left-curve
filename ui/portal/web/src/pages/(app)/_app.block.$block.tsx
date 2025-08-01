import { createFileRoute } from "@tanstack/react-router";

import { m } from "~/paraglide/messages";

export const Route = createFileRoute("/(app)/_app/block/$block")({
  head: () => ({
    meta: [{ title: `Dango | ${m["explorer.block.title"]()}` }],
  }),
});
