import { createRoute } from "@tanstack/react-router";
import { Route as rootRoute } from "./__root";
import { EarnScreen } from "../earn";

export const earnRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/earn",
  component: EarnScreen,
});
