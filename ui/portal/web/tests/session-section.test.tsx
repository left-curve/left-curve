import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import {
  resetAppletsKitMocks,
  setAppletsKitUseApp,
  setAppletsKitUseMediaQuery,
} from "./mocks/applets-kit";
import { Modals } from "@left-curve/applets-kit";
import { SessionSection } from "../src/components/settings/SessionSection";

type BlockSubscriptionListener = (event: {
  blockHeight: number;
  createdAt: string;
  hash: string;
}) => void;

const sessionSectionMocks = vi.hoisted(() => ({
  blockListeners: new Set<BlockSubscriptionListener>(),
  blockUnsubscribes: [] as ReturnType<typeof vi.fn>[],
  showModal: vi.fn(),
  subscriptions: {
    subscribe: vi.fn(),
  },
  useAccount: vi.fn(),
  useConfig: vi.fn(),
  useServiceStatus: vi.fn(),
  useSessionKey: vi.fn(),
}));

vi.mock("@tanstack/react-router", async () => {
  const React = await import("react");

  return {
    Link: React.forwardRef<HTMLAnchorElement, React.PropsWithChildren<{ to?: string }>>(
      ({ children, to, ...props }, ref) => (
        <a href={to} ref={ref} {...props}>
          {children}
        </a>
      ),
    ),
  };
});

vi.mock("@left-curve/foundation", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@left-curve/foundation")>();

  return {
    ...actual,
    useApp: () => ({ subscriptions: sessionSectionMocks.subscriptions }),
  };
});

vi.mock("@left-curve/store", () => ({
  useAccount: sessionSectionMocks.useAccount,
  useConfig: sessionSectionMocks.useConfig,
  useServiceStatus: sessionSectionMocks.useServiceStatus,
  useSessionKey: sessionSectionMocks.useSessionKey,
}));

vi.mock("../src/components/settings/SessionCountdown", () => ({
  SessionCountdown: () => <span data-testid="session-countdown">01:00</span>,
}));

describe("SessionSection", () => {
  beforeEach(() => {
    resetAppletsKitMocks();
    setAppletsKitUseApp({
      showModal: sessionSectionMocks.showModal,
    });
    setAppletsKitUseMediaQuery({
      isLg: true,
    });
    sessionSectionMocks.blockListeners.clear();
    sessionSectionMocks.blockUnsubscribes = [];
    sessionSectionMocks.subscriptions.subscribe.mockImplementation(
      (_key: "block", { listener }: { listener: BlockSubscriptionListener }) => {
        sessionSectionMocks.blockListeners.add(listener);
        const unsubscribe = vi.fn(() => {
          sessionSectionMocks.blockListeners.delete(listener);
        });
        sessionSectionMocks.blockUnsubscribes.push(unsubscribe);
        return unsubscribe;
      },
    );
    Object.defineProperty(window, "dango", {
      configurable: true,
      value: {
        urls: {
          upUrl: "https://status.example/up",
        },
      },
    });
    sessionSectionMocks.useAccount.mockReturnValue({
      isConnected: true,
      isUserActive: true,
      userIndex: 7,
      username: "user_7",
      userStatus: "active",
    });
    sessionSectionMocks.useConfig.mockReturnValue({
      chain: {
        name: "Dango Devnet",
        url: "https://rpc.example/graphql",
      },
    });
    sessionSectionMocks.useServiceStatus.mockReturnValue({
      chainStatus: "success",
      wsStatus: "warning",
    });
    sessionSectionMocks.useSessionKey.mockReturnValue({
      session: {
        sessionInfo: {
          expireAt: "1893456000",
        },
      },
    });
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("opens the edit-username modal only for an active default username", () => {
    render(<SessionSection.Username />);

    fireEvent.click(screen.getByText("user_7"));

    expect(sessionSectionMocks.showModal).toHaveBeenCalledWith(Modals.EditUsername);

    cleanup();
    vi.clearAllMocks();
    sessionSectionMocks.useAccount.mockReturnValue({
      isConnected: true,
      isUserActive: true,
      userIndex: 7,
      username: "alice",
      userStatus: "active",
    });

    render(<SessionSection.Username />);

    fireEvent.click(screen.getByText("alice"));

    expect(sessionSectionMocks.showModal).not.toHaveBeenCalled();
  });

  it("opens the edit-username modal for an active default username at backend user index zero", () => {
    sessionSectionMocks.useAccount.mockReturnValue({
      isConnected: true,
      isUserActive: true,
      userIndex: 0,
      username: "user_0",
      userStatus: "active",
    });

    render(<SessionSection.Username />);

    fireEvent.click(screen.getByText("user_0"));

    expect(sessionSectionMocks.showModal).toHaveBeenCalledWith(Modals.EditUsername);
  });

  it("renders inactive account guidance with a bridge link", () => {
    sessionSectionMocks.useAccount.mockReturnValue({
      isConnected: true,
      isUserActive: false,
      userIndex: 7,
      username: "user_7",
      userStatus: "inactive",
    });

    render(<SessionSection.UserStatus />);

    expect(screen.getByText(m["settings.session.userStatus.description"]())).toBeInTheDocument();
    expect(
      screen.getByRole("link", { name: m["settings.session.userStatus.button"]() }),
    ).toHaveAttribute("href", "/bridge");
    expect(
      screen.getByText(m["settings.session.accountStatus"]({ status: "inactive" })),
    ).toBeInTheDocument();
  });

  it("shows network metadata and live block details without the graphql suffix", () => {
    const rendered = render(<SessionSection.Network />);

    expect(screen.getByText("Dango Devnet")).toBeInTheDocument();
    expect(screen.getByText("https://rpc.example")).toBeInTheDocument();
    expect(sessionSectionMocks.subscriptions.subscribe).toHaveBeenCalledTimes(2);
    expect(sessionSectionMocks.subscriptions.subscribe).toHaveBeenNthCalledWith(1, "block", {
      listener: expect.any(Function),
    });
    expect(sessionSectionMocks.subscriptions.subscribe).toHaveBeenNthCalledWith(2, "block", {
      listener: expect.any(Function),
    });

    act(() => {
      for (const listener of sessionSectionMocks.blockListeners) {
        listener({
          blockHeight: 456,
          createdAt: "2026-06-10T12:00:00.000Z",
          hash: "0x73657373696f6e2d626c6f636b00000000000000",
        });
      }
    });

    expect(screen.getByText("#456")).toBeInTheDocument();
    expect(screen.getByText("2026-06-10T12:00:00.000Z")).toBeInTheDocument();

    rendered.unmount();

    expect(sessionSectionMocks.blockUnsubscribes).toHaveLength(2);
    expect(sessionSectionMocks.blockUnsubscribes[0]).toHaveBeenCalledOnce();
    expect(sessionSectionMocks.blockUnsubscribes[1]).toHaveBeenCalledOnce();
    expect(sessionSectionMocks.blockListeners.size).toBe(0);
  });

  it("opens QR connect only when a session is available", () => {
    render(<SessionSection.ConnectMobile />);

    fireEvent.click(screen.getByRole("button", { name: m["settings.connectToMobile"]() }));

    expect(sessionSectionMocks.showModal).toHaveBeenCalledWith(Modals.QRConnect);

    cleanup();
    vi.clearAllMocks();
    sessionSectionMocks.useSessionKey.mockReturnValue({ session: null });

    render(<SessionSection.ConnectMobile />);

    expect(screen.queryByText(m["settings.connectToMobile"]())).not.toBeInTheDocument();
  });

  it("renders websocket and chain statuses from the service-status hook", () => {
    render(<SessionSection.Status />);

    expect(sessionSectionMocks.useServiceStatus).toHaveBeenCalledWith({
      upUrl: "https://status.example/up",
    });
    expect(screen.getByText(m["statusBadge.websocket"]())).toBeInTheDocument();
    expect(screen.getByText(m["statusBadge.chain"]())).toBeInTheDocument();
    expect(
      screen.getByText(m["statusBadge.statusText"]({ status: "warning" })),
    ).toBeInTheDocument();
    expect(
      screen.getByText(m["statusBadge.statusText"]({ status: "success" })),
    ).toBeInTheDocument();
  });

  it("renders the session countdown only when a session key exists", () => {
    render(<SessionSection.RemainingTime />);

    expect(screen.getByTestId("session-countdown")).toBeInTheDocument();

    cleanup();
    sessionSectionMocks.useSessionKey.mockReturnValue({ session: null });

    render(<SessionSection.RemainingTime />);

    expect(screen.queryByTestId("session-countdown")).not.toBeInTheDocument();
  });
});
