import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/(app)/_app/user/$username")({
  head: () => ({
    meta: [{ title: "Dango | User Profile" }],
  }),
});
