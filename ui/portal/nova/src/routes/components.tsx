import { createRoute } from "@tanstack/react-router";
import { Route as rootRoute } from "./__root";
import { ComponentShowcase } from "../components/ComponentShowcase";

export const componentsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/components",
  component: ComponentShowcase,
});
