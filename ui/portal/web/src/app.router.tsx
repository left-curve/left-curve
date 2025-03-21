import { RouterProvider, createRouter } from "@tanstack/react-router";

import { useAccount, useConfig, usePublicClient } from "@left-curve/store";

import { Spinner } from "@left-curve/applets-kit";

import type {
  UseAccountReturnType,
  UseConfigReturnType,
  UsePublicClientReturnType,
} from "@left-curve/store";

import { routeTree } from "./app.pages";

const router = createRouter({
  routeTree,
  defaultPreload: "intent",
  defaultStaleTime: 5000,
  scrollRestoration: true,
  defaultPendingComponent: () => (
    <div className="flex-1 w-full flex justify-center items-center">
      <Spinner size="lg" color="pink" />
    </div>
  ),
});

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}

export interface RouterContext {
  client?: UsePublicClientReturnType;
  account?: UseAccountReturnType;
  config?: UseConfigReturnType;
}

export const AppRouter: React.FC = () => {
  const account = useAccount();
  const config = useConfig();
  const client = usePublicClient();

  return <RouterProvider router={router} context={{ account, config, client }} />;
};
