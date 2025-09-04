import { RouterProvider, createRouter, useNavigate } from "@tanstack/react-router";

import { useAccount, useConfig, usePublicClient } from "@left-curve/store";

import { AppProvider, Spinner, useTheme } from "@left-curve/applets-kit";

import { Toast } from "@left-curve/applets-kit";
import { RootModal } from "./components/modals/RootModal";

import { routeTree } from "./app.pages";

import type {
  UseAccountReturnType,
  UseConfigReturnType,
  UsePublicClientReturnType,
} from "@left-curve/store";
import { createToaster } from "./app.toaster";

const [Toaster, toast] = createToaster((props) => <Toast {...props} />);

export const router = createRouter({
  routeTree,
  defaultPreload: "intent",
  defaultStaleTime: 5000,
  scrollRestoration: true,
  context: {} as RouterContext,
  defaultPendingComponent: () => (
    <div className="flex-1 w-full flex justify-center items-center h-screen">
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
  client: UsePublicClientReturnType;
  account: UseAccountReturnType;
  config: UseConfigReturnType;
}

export const AppRouter: React.FC = () => {
  const account = useAccount();
  const config = useConfig();
  const client = usePublicClient();

  return (
    <RouterProvider
      router={router}
      context={{ account, config, client }}
      InnerWrap={({ children }) => {
        const navigate = useNavigate();
        const _theme = useTheme();
        return (
          <AppProvider toast={toast} navigate={(to, options) => navigate({ to, ...options })}>
            {children}
            <RootModal />
            <Toaster position="bottom-center" containerStyle={{ zIndex: 99999999 }} />
          </AppProvider>
        );
      }}
    />
  );
};
