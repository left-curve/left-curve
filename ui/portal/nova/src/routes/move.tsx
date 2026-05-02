import { createRoute } from "@tanstack/react-router";
import { Route as rootRoute } from "./__root";
import { MoveScreen } from "../move";

export const moveRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/move",
  component: MoveScreen,
});
