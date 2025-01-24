import {
  Outlet,
  RouterProvider,
  createRootRouteWithContext,
  createRoute,
  createRouter,
  redirect,
  useNavigate,
} from "@tanstack/react-router";

import { useAccount, useConfig, usePublicClient } from "@left-curve/store-react";

import { AccountsRoute } from "./pages/accounts";
import { AuthRoute } from "./pages/auth";

import { Spinner } from "@left-curve/applets-kit";
import { AppLayout } from "./components/AppLayout";

import type {
  UseAccountReturnType,
  UseConfigReturnType,
  UsePublicClientReturnType,
} from "@left-curve/store-react";
import { useEffect } from "react";

export const AppRoute = createRoute({
  id: "app-layout",
  getParentRoute: () => RootRouter,
  beforeLoad: async ({ context }) => {
    const { account } = context;
    if (!account?.isConnected) throw redirect({ to: "/auth/login" });
  },
  component: () => {
    const { account, isConnected } = useAccount();
    const navigate = useNavigate();

    useEffect(() => {
      if (!isConnected) {
        navigate({ to: "/auth/login" });
      }
    }, [account]);

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
  AccountsRoute,
  createRoute({
    getParentRoute: () => AppRoute,
    path: "/",
  }),

  createRoute({
    getParentRoute: () => AppRoute,
    path: "/transfer",
  }).lazy(() => import("./pages/transfer").then((d) => d.TransferRoute)),

  createRoute({
    getParentRoute: () => AppRoute,
    path: "/swap",
  }).lazy(() => import("./pages/swap").then((d) => d.SwapRoute)),

  createRoute({
    getParentRoute: () => AppRoute,
    path: "/block-explorer",
  }).lazy(() => import("./pages/block-explorer").then((d) => d.BlockExplorerRoute)),

  createRoute({
    getParentRoute: () => AppRoute,
    path: "/amm",
  }).lazy(() => import("./pages/amm").then((d) => d.AmmRoute)),

  createRoute({
    getParentRoute: () => AppRoute,
    path: "/account-creation",
  }).lazy(() => import("./pages/account-creation").then((d) => d.AccountCreationRoute)),

  createRoute({
    getParentRoute: () => AppRoute,
    path: "$",
    component: () => {
      return (
        <div className="w-full flex flex-1 justify-center items-center p-4">
          <h3 className="text-center max-w-4xl typography-display-xs md:typography-display-xl">
            Sorry, we couldn't find the page you were looking for.
          </h3>
        </div>
      );
    },
  }),
]);

export interface RouterContext {
  client?: UsePublicClientReturnType;
  account?: UseAccountReturnType;
  config?: UseConfigReturnType;
}

export const RootRouter = createRootRouteWithContext<RouterContext>()({
  beforeLoad: async ({ context }) => {
    const { config } = context;
    if (!config?.state.isMipdLoaded) {
      await new Promise((resolve) => {
        config?.subscribe(
          (x) => x.isMipdLoaded,
          (isMipdLoaded) => isMipdLoaded && resolve(null),
        );
      });
    }
  },
});

const router = createRouter({
  routeTree: RootRouter.addChildren([AppRoute, AuthRoute]),
  defaultPreload: "intent",
  defaultPendingComponent: () => (
    <div className="flex-1 w-full flex justify-center items-center">
      <Spinner size="lg" color="pink" />
    </div>
  ),
});

export const AppRouter: React.FC = () => {
  const account = useAccount();
  const config = useConfig();
  const client = usePublicClient();

  return <RouterProvider router={router} context={{ account, config, client }} />;
};
