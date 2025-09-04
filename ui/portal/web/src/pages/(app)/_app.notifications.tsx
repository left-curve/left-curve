import { createFileRoute } from "@tanstack/react-router";

import { m } from "@left-curve/foundation/paraglide/messages.js";

export const Route = createFileRoute("/(app)/_app/notifications")({
  head: () => ({
    meta: [{ title: `Dango | ${m["notifications.title"]()}` }],
  }),
});
