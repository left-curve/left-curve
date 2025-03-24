import { createRootRouteWithContext } from "@tanstack/react-router";
import type { RouterContext } from "~/app.router";

export const Route = createRootRouteWithContext<RouterContext>()({
  errorComponent: () => <div>Something went wrong</div>,
});
