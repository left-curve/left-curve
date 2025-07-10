import { createFileRoute } from "@tanstack/react-router";

import { m } from "~/paraglide/messages";

export const Route = createFileRoute("/(app)/_app/settings")({
  head: () => ({
    meta: [{ title: `Dango | ${m["settings.title"]()}` }],
  }),
});
