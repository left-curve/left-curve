import { createRootRouteWithContext, Navigate } from "@tanstack/react-router";
import { NovaShell } from "../layout/NovaShell";
import "../tokens/global.css";

import type { RouterContext } from "~/app.router";

function NotFound() {
  return <Navigate to="/trade" />;
}

export const Route = createRootRouteWithContext<RouterContext>()({
  component: () => <NovaShell />,
  notFoundComponent: NotFound,
});
