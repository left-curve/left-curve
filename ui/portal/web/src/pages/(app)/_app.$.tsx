import { createFileRoute } from "@tanstack/react-router";

import { NotFound } from "~/components/foundation/NotFound";

export const Route = createFileRoute("/(app)/_app/$")({
  component: NotFound,
});
