import { RouterProvider, createRouter } from "@tanstack/react-router";
import { useAccount, useConfig, usePublicClient } from "@left-curve/store";
import { useTheme } from "@left-curve/applets-kit";
import { queryClient } from "~/queryClient";

import { Route as rootRoute } from "./routes/__root";
import { tradeRoute, tradeIndexRoute } from "./routes/trade";
import { earnRoute } from "./routes/earn";
import { moveRoute } from "./routes/move";
import {
  explorerRoute,
  explorerIndexRoute,
  explorerBlockRoute,
  explorerTxRoute,
} from "./routes/explorer";
import {
  accountRoute,
  accountIndexRoute,
  accountOverviewRoute,
  accountPortfolioRoute,
  accountPreferencesRoute,
  accountSecurityRoute,
  accountSessionRoute,
  accountReferralRoute,
  accountRewardsRoute,
} from "./routes/account";
import { componentsRoute } from "./routes/components";

import type { RouterContext } from "~/app.router";

const routeTree = rootRoute.addChildren([
  tradeIndexRoute,
  tradeRoute,
  earnRoute,
  moveRoute,
  explorerRoute.addChildren([explorerIndexRoute, explorerBlockRoute, explorerTxRoute]),
  accountRoute.addChildren([
    accountIndexRoute,
    accountOverviewRoute,
    accountPortfolioRoute,
    accountPreferencesRoute,
    accountSecurityRoute,
    accountSessionRoute,
    accountReferralRoute,
    accountRewardsRoute,
  ]),
  componentsRoute,
]);

export const novaRouter = createRouter({
  routeTree,
  defaultPreload: "intent",
  defaultStaleTime: 5000,
  scrollRestoration: true,
  context: {} as RouterContext,
});

export function NovaRouter() {
  const account = useAccount();
  const config = useConfig();
  const client = usePublicClient();
  const theme = useTheme();

  return (
    <RouterProvider router={novaRouter} context={{ account, config, client, theme, queryClient }} />
  );
}
