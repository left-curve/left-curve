import { createFileRoute } from "@tanstack/react-router";

import { m } from "@left-curve/foundation/paraglide/messages.js";

export const Route = createFileRoute("/(app)/_app/account/create")({
  head: () => ({
    meta: [{ title: `Dango | ${m["signup.createAccount"]()}` }],
  }),
});
