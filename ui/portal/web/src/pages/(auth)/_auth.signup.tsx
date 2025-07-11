import { createFileRoute } from "@tanstack/react-router";

import { m } from "~/paraglide/messages";

export const Route = createFileRoute("/(auth)/_auth/signup")({
  head: () => ({
    meta: [{ title: `Dango | ${m["common.signup"]()}` }],
  }),
});
