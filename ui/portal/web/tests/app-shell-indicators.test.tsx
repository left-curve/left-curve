import { act, cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { resetAppletsKitMocks, setAppletsKitUseAppFactory } from "./mocks/applets-kit";

import { TxIndicator } from "../src/components/foundation/TxIndicator";

const appShellMocks = vi.hoisted(() => ({
  submitTxListener: undefined as
    | ((
        event:
          | { status: "pending" }
          | { description?: string; status: "error"; title: string }
          | { status: "success" },
      ) => void)
    | undefined,
  subscribe: vi.fn(),
  toastError: vi.fn(),
  unsubscribe: vi.fn(),
}));

describe("app shell indicators", () => {
  beforeEach(() => {
    resetAppletsKitMocks();
    appShellMocks.submitTxListener = undefined;
    appShellMocks.subscribe.mockImplementation((_key, { listener }) => {
      appShellMocks.submitTxListener = listener;
      return appShellMocks.unsubscribe;
    });
    setAppletsKitUseAppFactory(() => ({
      settings: {
        formatNumberOptions: {},
      },
      subscriptions: {
        subscribe: appShellMocks.subscribe,
      },
      toast: {
        error: appShellMocks.toastError,
      },
    }));
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
    vi.unstubAllGlobals();
    vi.useRealTimers();
    window.history.pushState({}, "", "/");
  });

  it("tracks transaction pending, success, and error states from submitTx subscriptions", () => {
    vi.useFakeTimers();
    const { container, unmount } = render(<TxIndicator icon={<span>Wallet</span>} />);

    expect(appShellMocks.subscribe).toHaveBeenCalledWith(
      "submitTx",
      expect.objectContaining({
        listener: expect.any(Function),
      }),
    );
    expect(screen.getByText("Wallet")).toBeInTheDocument();

    act(() => {
      appShellMocks.submitTxListener?.({ status: "pending" });
    });

    expect(container.querySelector(".animate-spinner-ease-spin")).toBeInTheDocument();

    act(() => {
      appShellMocks.submitTxListener?.({ status: "success" });
    });

    expect(container.querySelector(".text-primitives-green-light-300")).toBeInTheDocument();

    act(() => {
      vi.advanceTimersByTime(1500);
    });

    expect(screen.getByText("Wallet")).toBeInTheDocument();

    act(() => {
      appShellMocks.submitTxListener?.({ status: "pending" });
    });
    act(() => {
      appShellMocks.submitTxListener?.({
        description: "balance below required amount",
        status: "error",
        title: "Transaction failed",
      });
    });

    expect(container.querySelector(".text-primitives-red-light-300")).toBeInTheDocument();
    expect(appShellMocks.toastError).toHaveBeenCalledWith({
      description: "balance below required amount",
      title: "Transaction failed",
    });

    act(() => {
      vi.advanceTimersByTime(1500);
    });

    expect(screen.getByText("Wallet")).toBeInTheDocument();

    unmount();

    expect(appShellMocks.unsubscribe).toHaveBeenCalledOnce();
  });
});
