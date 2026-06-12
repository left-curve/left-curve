import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import { resetAppletsKitMocks, setAppletsKitUseApp } from "./mocks/applets-kit";
import { StatusBadge } from "../src/components/foundation/StatusBadge";

type BlockSubscriptionListener = (event: {
  blockHeight: number;
  createdAt: string;
  hash: string;
}) => void;

const statusMocks = vi.hoisted(() => ({
  blockListener: undefined as BlockSubscriptionListener | undefined,
  blockUnsubscribe: vi.fn(),
  navigate: vi.fn(),
  subscriptions: {
    subscribe: vi.fn(),
  },
  useServiceStatus: vi.fn(),
}));

vi.mock("@left-curve/foundation", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@left-curve/foundation")>();

  return {
    ...actual,
    useApp: () => ({
      subscriptions: statusMocks.subscriptions,
    }),
  };
});

vi.mock("@left-curve/store", () => ({
  useServiceStatus: statusMocks.useServiceStatus,
}));

function setServiceStatus({
  chainStatus = "success",
  globalStatus = "success",
  isChainPaused = false,
  isReady = true,
  transportMode = "ws",
  wsStatus = "success",
}: {
  chainStatus?: "error" | "success" | "warning";
  globalStatus?: "error" | "success" | "warning";
  isChainPaused?: boolean | undefined;
  isReady?: boolean;
  transportMode?: "http-polling" | "reconnecting" | "ws";
  wsStatus?: "error" | "success" | "warning";
} = {}) {
  statusMocks.useServiceStatus.mockReturnValue({
    chainStatus,
    globalStatus,
    isChainPaused,
    isReady,
    transportMode,
    wsStatus,
  });
}

describe("StatusBadge", () => {
  beforeEach(() => {
    resetAppletsKitMocks();
    setAppletsKitUseApp({
      navigate: statusMocks.navigate,
    });
    statusMocks.blockListener = undefined;
    statusMocks.blockUnsubscribe.mockReset();
    statusMocks.subscriptions.subscribe.mockImplementation(
      (_key: "block", { listener }: { listener: BlockSubscriptionListener }) => {
        statusMocks.blockListener = listener;
        return statusMocks.blockUnsubscribe;
      },
    );
    setServiceStatus();
    class ResizeObserverMock {
      disconnect = vi.fn();
      observe = vi.fn();
      unobserve = vi.fn();
    }

    Object.defineProperty(globalThis, "ResizeObserver", {
      configurable: true,
      value: ResizeObserverMock,
    });
    Object.defineProperty(window, "dango", {
      configurable: true,
      value: {
        urls: {
          upUrl: "https://status.example/up",
        },
      },
    });
    window.history.pushState({}, "", "/");
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("does not render or navigate until service status checks are ready", () => {
    setServiceStatus({
      isChainPaused: undefined,
      isReady: false,
    });

    render(<StatusBadge />);

    expect(
      screen.queryByText(m["statusBadge.statusText"]({ status: "success" })),
    ).not.toBeInTheDocument();
    expect(statusMocks.navigate).not.toHaveBeenCalled();
    expect(statusMocks.useServiceStatus).toHaveBeenCalledWith({
      upUrl: "https://status.example/up",
    });
  });

  it("navigates to maintenance when the chain is paused outside maintenance", async () => {
    setServiceStatus({
      chainStatus: "error",
      globalStatus: "error",
      isChainPaused: true,
    });

    render(<StatusBadge />);

    await waitFor(() => {
      expect(statusMocks.navigate).toHaveBeenCalledWith("/maintenance");
    });
    expect(screen.getByText(m["statusBadge.statusText"]({ status: "error" }))).toBeInTheDocument();
  });

  it("navigates home when the chain recovers on the maintenance page", async () => {
    window.history.pushState({}, "", "/maintenance");
    setServiceStatus({
      isChainPaused: false,
    });

    render(<StatusBadge />);

    await waitFor(() => {
      expect(statusMocks.navigate).toHaveBeenCalledWith("/");
    });
  });

  it("renders websocket and chain status labels from service status", () => {
    setServiceStatus({
      globalStatus: "warning",
      transportMode: "http-polling",
      wsStatus: "warning",
    });

    render(<StatusBadge />);

    const warningLabel = screen.getByText(m["statusBadge.statusText"]({ status: "warning" }));
    const popoverTrigger = warningLabel.closest("button");
    expect(popoverTrigger).not.toBeNull();

    fireEvent.click(popoverTrigger as HTMLButtonElement);

    expect(screen.getByText(m["statusBadge.websocket"]())).toBeInTheDocument();
    expect(screen.getByText(m["statusBadge.httpPolling"]())).toBeInTheDocument();
    expect(screen.getByText(m["statusBadge.chain"]())).toBeInTheDocument();
    expect(
      screen.getByText(m["statusBadge.statusText"]({ status: "success" })),
    ).toBeInTheDocument();

    expect(statusMocks.subscriptions.subscribe).toHaveBeenCalledWith("block", {
      listener: expect.any(Function),
    });
    act(() => {
      statusMocks.blockListener?.({
        blockHeight: 123,
        createdAt: "2026-06-10T12:00:00.000Z",
        hash: "0x626c6f636b2d6861736800000000000000000000",
      });
    });

    expect(screen.getByText("#123")).toBeInTheDocument();
  });

  it("labels reconnecting websocket transport separately from chain health", () => {
    setServiceStatus({
      chainStatus: "success",
      globalStatus: "warning",
      transportMode: "reconnecting",
      wsStatus: "warning",
    });

    render(<StatusBadge />);

    const warningLabel = screen.getByText(m["statusBadge.statusText"]({ status: "warning" }));
    const popoverTrigger = warningLabel.closest("button");
    expect(popoverTrigger).not.toBeNull();

    fireEvent.click(popoverTrigger as HTMLButtonElement);

    expect(screen.getByText(m["statusBadge.reconnecting"]())).toBeInTheDocument();
    expect(screen.queryByText(m["statusBadge.httpPolling"]())).not.toBeInTheDocument();
    expect(
      screen.getByText(m["statusBadge.statusText"]({ status: "success" })),
    ).toBeInTheDocument();
  });

  it("opens the public status details page from the popover action", () => {
    const open = vi.fn();
    vi.stubGlobal("open", open);

    render(<StatusBadge />);

    const popoverTrigger = screen
      .getByText(m["statusBadge.statusText"]({ status: "success" }))
      .closest("button");
    expect(popoverTrigger).not.toBeNull();

    fireEvent.click(popoverTrigger as HTMLButtonElement);
    fireEvent.click(screen.getByRole("button", { name: new RegExp(m["statusBadge.details"]()) }));

    expect(open).toHaveBeenCalledWith(
      "https://status.dango.exchange/",
      "_blank",
      "noopener,noreferrer",
    );
  });
});
