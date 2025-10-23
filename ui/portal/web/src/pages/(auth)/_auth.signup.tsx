import { createFileRoute } from "@tanstack/react-router";

import { m } from "@left-curve/foundation/paraglide/messages.js";

export const Route = createFileRoute("/(auth)/_auth/signup")({
  head: () => ({
    meta: [{ title: `Dango | ${m["common.signup"]()}` }],
  }),
});
