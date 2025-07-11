import { createFileRoute } from "@tanstack/react-router";
import { m } from "~/paraglide/messages";

export const Route = createFileRoute("/(app)/_app/account/$address")({
  head: () => ({
    meta: [{ title: `Dango | ${m["explorer.accounts.title"]()}` }],
  }),
});
