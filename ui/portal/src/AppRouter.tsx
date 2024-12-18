import { RouterProvider, createRouter } from "@tanstack/react-router";

import { useAccount, useConfig, usePublicClient } from "@left-curve/react";

import { MainRouteWithChildren } from "./pages/layout";

const router = createRouter({
  routeTree: MainRouteWithChildren,
  defaultPreload: "intent",
});

export const AppRouter: React.FC = () => {
  const account = useAccount();
  const config = useConfig();
  const client = usePublicClient();

  return <RouterProvider router={router} context={{ account, config, client }} />;
};
