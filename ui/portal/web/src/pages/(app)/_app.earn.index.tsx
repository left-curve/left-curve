import { createFileRoute } from "@tanstack/react-router";

import { m } from "~/paraglide/messages";

export const Route = createFileRoute("/(app)/_app/earn/")({
  head: () => ({
    meta: [{ title: `Dango | ${m["earn.earn"]()}` }],
  }),
});
