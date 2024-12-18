import { Outlet, createRoute } from "@tanstack/react-router";
import { AppLayout } from "~/components/AppLayout";
import { MainRoute } from "../layout";
import { AccountsRoute } from "./accounts";

export const AppRoute = createRoute({
  id: "app-layout",
  getParentRoute: () => MainRoute,
  component: () => {
    return (
      <>
        <AppLayout>
          <Outlet />
        </AppLayout>
      </>
    );
  },
});

export const AppRouteWithChildren = AppRoute.addChildren([
  createRoute({
    getParentRoute: () => AppRoute,
    path: "/",
  }),

  createRoute({
    getParentRoute: () => AppRoute,
    path: "/accounts",
  }).update(AccountsRoute),

  createRoute({
    getParentRoute: () => AppRoute,
    path: "/transfer",
  }).lazy(() => import("./transfer").then((d) => d.TransferRoute)),

  createRoute({
    getParentRoute: () => AppRoute,
    path: "/swap",
  }).lazy(() => import("./swap").then((d) => d.SwapRoute)),

  createRoute({
    getParentRoute: () => AppRoute,
    path: "/block-explorer",
  }).lazy(() => import("./block-explorer").then((d) => d.BlockExplorerRoute)),

  createRoute({
    getParentRoute: () => AppRoute,
    path: "/amm",
  }).lazy(() => import("./amm").then((d) => d.AmmRoute)),

  createRoute({
    getParentRoute: () => AppRoute,
    path: "/account-creation",
  }).lazy(() => import("./account-creation").then((d) => d.AccountCreationRoute)),
]);
