import { act, cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type React from "react";

import { resetAppletsKitMocks, setAppletsKitUseAppFactory } from "./mocks/applets-kit";

import { Modals } from "@left-curve/applets-kit";

import { Route } from "../src/pages/__root";

const rootRouteMocks = vi.hoisted(() => ({
  account: {
    address: "0x616c696365000000000000000000000000000000",
  },
  connector: {
    id: "wallet",
    name: "Browser Wallet",
    type: "wallet",
  } as { id: string; name: string; type: string } | undefined,
  ensureQueryData: vi.fn(),
  getAppConfigQueryOptions: vi.fn(),
  isConnected: true,
  mipdListener: undefined as ((isMipdLoaded: boolean) => void) | undefined,
  modal: {
    modal: null as string | null,
  },
  queryOptions: {
    queryKey: ["app-config"],
  },
  session: undefined as { sessionInfo: { expireAt: string } } | undefined,
  setContext: vi.fn(),
  setUser: vi.fn(),
  settings: {
    useSessionKey: true,
  },
  showModal: vi.fn(),
  startActivities: vi.fn(),
  stopActivities: vi.fn(),
  subscribe: vi.fn(),
  username: "alice" as string | undefined,
}));

vi.mock("@sentry/react", () => ({
  setContext: rootRouteMocks.setContext,
  setUser: rootRouteMocks.setUser,
}));

vi.mock("@tanstack/react-router", () => ({
  createRootRouteWithContext: () => (options: unknown) => ({ options }),
  HeadContent: () => <title>head content</title>,
  Outlet: () => <main data-testid="root-outlet" />,
}));

vi.mock("@left-curve/store", () => ({
  getAppConfigQueryOptions: rootRouteMocks.getAppConfigQueryOptions,
  useAccount: () => ({
    account: rootRouteMocks.account,
    connector: rootRouteMocks.connector,
    isConnected: rootRouteMocks.isConnected,
    username: rootRouteMocks.username,
  }),
  useActivities: () => ({
    startActivities: rootRouteMocks.startActivities,
  }),
  useSessionKey: () => ({
    session: rootRouteMocks.session,
  }),
}));

vi.mock("~/components/foundation/ErrorPage", () => ({
  ErrorPage: ({ error }: { error: Error }) => <div>{error.message}</div>,
}));

type RootRoute = {
  options: {
    beforeLoad: (args: {
      context: {
        config: {
          state: {
            isMipdLoaded: boolean;
          };
          subscribe: (
            selector: (state: { isMipdLoaded: boolean }) => boolean,
            listener: (isMipdLoaded: boolean) => void,
          ) => void;
        };
        queryClient: {
          ensureQueryData: ReturnType<typeof vi.fn>;
        };
      };
    }) => Promise<void>;
    component: React.ComponentType;
  };
};

function rootRoute() {
  return Route as unknown as RootRoute;
}

function renderRoot() {
  const Component = rootRoute().options.component;
  return render(<Component />);
}

describe("root route", () => {
  beforeEach(() => {
    resetAppletsKitMocks();
    rootRouteMocks.account = {
      address: "0x616c696365000000000000000000000000000000",
    };
    rootRouteMocks.connector = {
      id: "wallet",
      name: "Browser Wallet",
      type: "wallet",
    };
    rootRouteMocks.ensureQueryData.mockResolvedValue(undefined);
    rootRouteMocks.getAppConfigQueryOptions.mockReturnValue(rootRouteMocks.queryOptions);
    rootRouteMocks.isConnected = true;
    rootRouteMocks.mipdListener = undefined;
    rootRouteMocks.modal = {
      modal: null,
    };
    rootRouteMocks.session = undefined;
    rootRouteMocks.settings = {
      useSessionKey: true,
    };
    rootRouteMocks.startActivities.mockReturnValue(rootRouteMocks.stopActivities);
    rootRouteMocks.subscribe.mockImplementation((_selector, listener) => {
      rootRouteMocks.mipdListener = listener;
    });
    rootRouteMocks.username = "alice";
    setAppletsKitUseAppFactory(() => ({
      modal: rootRouteMocks.modal,
      settings: rootRouteMocks.settings,
      showModal: rootRouteMocks.showModal,
    }));
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
    vi.useRealTimers();
  });

  it("waits for wallet-provider discovery before ensuring app config data", async () => {
    const config = {
      state: {
        isMipdLoaded: false,
      },
      subscribe: rootRouteMocks.subscribe,
    };

    const beforeLoadPromise = rootRoute().options.beforeLoad({
      context: {
        config,
        queryClient: {
          ensureQueryData: rootRouteMocks.ensureQueryData,
        },
      },
    });

    await Promise.resolve();

    expect(rootRouteMocks.subscribe).toHaveBeenCalledWith(
      expect.any(Function),
      expect.any(Function),
    );
    expect(rootRouteMocks.ensureQueryData).not.toHaveBeenCalled();

    rootRouteMocks.mipdListener?.(true);
    await beforeLoadPromise;

    expect(rootRouteMocks.getAppConfigQueryOptions).toHaveBeenCalledWith(config, {});
    expect(rootRouteMocks.ensureQueryData).toHaveBeenCalledWith(rootRouteMocks.queryOptions);
  });

  it("continues root loading when the app config query fails", async () => {
    rootRouteMocks.ensureQueryData.mockRejectedValue(new Error("config unavailable"));
    const config = {
      state: {
        isMipdLoaded: true,
      },
      subscribe: rootRouteMocks.subscribe,
    };

    await expect(
      rootRoute().options.beforeLoad({
        context: {
          config,
          queryClient: {
            ensureQueryData: rootRouteMocks.ensureQueryData,
          },
        },
      }),
    ).resolves.toBeUndefined();

    expect(rootRouteMocks.subscribe).not.toHaveBeenCalled();
    expect(rootRouteMocks.ensureQueryData).toHaveBeenCalledWith(rootRouteMocks.queryOptions);
  });

  it("tracks the current user, connector context, and activity lifecycle", async () => {
    const Component = rootRoute().options.component;
    const { rerender, unmount } = renderRoot();

    expect(await screen.findByTestId("root-outlet")).toBeInTheDocument();
    expect(rootRouteMocks.setUser).toHaveBeenCalledWith({ username: "alice" });
    expect(rootRouteMocks.setContext).toHaveBeenCalledWith("connector", {
      id: "wallet",
      name: "Browser Wallet",
      type: "wallet",
    });
    expect(rootRouteMocks.startActivities).toHaveBeenCalledOnce();

    rootRouteMocks.account = {
      address: "0x626f620000000000000000000000000000000000",
    };
    rerender(<Component />);

    await waitFor(() => {
      expect(rootRouteMocks.startActivities).toHaveBeenCalledTimes(2);
    });
    expect(rootRouteMocks.stopActivities).toHaveBeenCalledOnce();

    unmount();

    expect(rootRouteMocks.stopActivities).toHaveBeenCalledTimes(2);
  });

  it("clears Sentry user context when no username is available", () => {
    rootRouteMocks.username = undefined;

    renderRoot();

    expect(rootRouteMocks.setUser).toHaveBeenCalledWith(null);
    expect(rootRouteMocks.setContext).not.toHaveBeenCalled();
  });

  it("opens renew-session when a connected wallet has no usable session key", () => {
    vi.useFakeTimers();
    renderRoot();

    act(() => {
      vi.advanceTimersByTime(1000);
    });

    expect(rootRouteMocks.showModal).toHaveBeenCalledWith(Modals.RenewSession);
  });

  it("opens renew-session when the connected wallet session key is expired", () => {
    vi.useFakeTimers();
    rootRouteMocks.session = {
      sessionInfo: {
        expireAt: String(Math.floor(Date.now() / 1000) - 1),
      },
    };

    renderRoot();

    act(() => {
      vi.advanceTimersByTime(1000);
    });

    expect(rootRouteMocks.showModal).toHaveBeenCalledWith(Modals.RenewSession);
  });

  it("does not reopen renew-session while that modal is already active", () => {
    vi.useFakeTimers();
    rootRouteMocks.modal = {
      modal: Modals.RenewSession,
    };

    renderRoot();

    act(() => {
      vi.advanceTimersByTime(1000);
    });

    expect(rootRouteMocks.showModal).not.toHaveBeenCalled();
  });

  it("does not open renew-session for valid sessions or protected connector types", () => {
    vi.useFakeTimers();
    rootRouteMocks.session = {
      sessionInfo: {
        expireAt: String(Math.floor(Date.now() / 1000) + 60),
      },
    };

    const { unmount } = renderRoot();

    act(() => {
      vi.advanceTimersByTime(1000);
    });

    expect(rootRouteMocks.showModal).not.toHaveBeenCalled();

    unmount();
    cleanup();
    vi.clearAllMocks();
    rootRouteMocks.session = undefined;
    rootRouteMocks.connector = {
      id: "session",
      name: "Session",
      type: "session",
    };

    renderRoot();

    act(() => {
      vi.advanceTimersByTime(1000);
    });

    expect(rootRouteMocks.showModal).not.toHaveBeenCalled();

    unmount();
    cleanup();
    vi.clearAllMocks();
    rootRouteMocks.connector = {
      id: "debug",
      name: "Debug",
      type: "debug",
    };

    renderRoot();

    act(() => {
      vi.advanceTimersByTime(1000);
    });

    expect(rootRouteMocks.showModal).not.toHaveBeenCalled();
  });

  it("does not open renew-session when session keys are disabled in settings", () => {
    vi.useFakeTimers();
    rootRouteMocks.settings = {
      useSessionKey: false,
    };

    renderRoot();

    act(() => {
      vi.advanceTimersByTime(1000);
    });

    expect(rootRouteMocks.showModal).not.toHaveBeenCalled();
  });

  it("does not open renew-session while the account is disconnected", () => {
    vi.useFakeTimers();
    rootRouteMocks.isConnected = false;

    renderRoot();

    act(() => {
      vi.advanceTimersByTime(1000);
    });

    expect(rootRouteMocks.showModal).not.toHaveBeenCalled();
  });

  it("does not open renew-session when no connector is available to sign it", () => {
    vi.useFakeTimers();
    rootRouteMocks.connector = undefined;

    renderRoot();

    act(() => {
      vi.advanceTimersByTime(1000);
    });

    expect(rootRouteMocks.showModal).not.toHaveBeenCalled();
  });
});
