import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import {
  resetAppletsKitMocks,
  setAppletsKitUseAppFactory,
  setAppletsKitUseMediaQueryFactory,
  setAppletsKitUsePortalTargetFactory,
} from "./mocks/applets-kit";

import { QuestBanner, QuestBannerRender } from "../src/components/foundation/QuestBanner";
import { TxIndicator } from "../src/components/foundation/TxIndicator";

const appShellMocks = vi.hoisted(() => ({
  isConnected: true,
  isLg: true,
  isQuestBannerVisible: true,
  questData: undefined as
    | {
        eth_address?: string;
        trading_volumes?: string;
        tx_count?: number;
      }
    | undefined,
  questIsLoading: false,
  setQuestBannerVisibility: vi.fn(),
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
  username: "alice" as string | undefined,
  useQuery: vi.fn(),
}));

vi.mock("@tanstack/react-query", () => ({
  useQuery: appShellMocks.useQuery,
}));

vi.mock("@left-curve/store", () => ({
  useAccount: () => ({
    isConnected: appShellMocks.isConnected,
    username: appShellMocks.username,
  }),
}));

describe("app shell indicators", () => {
  beforeEach(() => {
    resetAppletsKitMocks();
    appShellMocks.isConnected = true;
    appShellMocks.isLg = true;
    appShellMocks.isQuestBannerVisible = true;
    appShellMocks.questData = undefined;
    appShellMocks.questIsLoading = false;
    appShellMocks.submitTxListener = undefined;
    appShellMocks.username = "alice";
    appShellMocks.subscribe.mockImplementation((_key, { listener }) => {
      appShellMocks.submitTxListener = listener;
      return appShellMocks.unsubscribe;
    });
    appShellMocks.useQuery.mockImplementation((options) => {
      appShellMocks.useQuery.options = options;
      return {
        data: appShellMocks.questData,
        isLoading: appShellMocks.questIsLoading,
      };
    });
    setAppletsKitUseAppFactory(() => ({
      isQuestBannerVisible: appShellMocks.isQuestBannerVisible,
      setQuestBannerVisibility: appShellMocks.setQuestBannerVisibility,
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
    setAppletsKitUseMediaQueryFactory(() => ({
      isLg: appShellMocks.isLg,
    }));
    setAppletsKitUsePortalTargetFactory((selector) => document.querySelector(selector));
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
    vi.unstubAllGlobals();
    vi.useRealTimers();
    window.history.pushState({}, "", "/");
  });

  it("fetches quest progress from the configured backend URL for the connected username", async () => {
    const questData = {
      eth_address: "0x6574680000000000000000000000000000000000",
      trading_volumes: "0",
      tx_count: 10,
    };
    const fetchMock = vi.fn().mockResolvedValue({
      json: vi.fn().mockResolvedValue(questData),
    });
    vi.stubGlobal("fetch", fetchMock);
    Object.defineProperty(window, "dango", {
      configurable: true,
      value: {
        urls: {
          questUrl: "https://quests.example",
        },
      },
    });

    render(<QuestBanner />);

    const { queryFn } = (
      appShellMocks.useQuery as unknown as {
        options: { queryFn: () => Promise<typeof questData> };
      }
    ).options;

    await expect(queryFn()).resolves.toEqual(questData);
    expect(fetchMock).toHaveBeenCalledWith("https://quests.example/alice");
  });

  it("does not enable quest progress fetches while disconnected or hidden", () => {
    appShellMocks.isConnected = false;
    appShellMocks.username = undefined;

    render(<QuestBanner />);

    expect(appShellMocks.useQuery).toHaveBeenLastCalledWith(
      expect.objectContaining({
        enabled: false,
        queryKey: ["quests", undefined],
      }),
    );

    cleanup();
    vi.clearAllMocks();
    appShellMocks.isConnected = true;
    appShellMocks.username = "alice";
    appShellMocks.isQuestBannerVisible = false;

    render(<QuestBanner />);

    expect(appShellMocks.useQuery).toHaveBeenLastCalledWith(
      expect.objectContaining({
        enabled: false,
        queryKey: ["quests", "alice"],
      }),
    );
  });

  it("renders incomplete quest progress, enables the username query, and can be dismissed", () => {
    appShellMocks.questData = {
      trading_volumes: "1000000",
      tx_count: 4,
    };

    const { container } = render(<QuestBanner />);

    expect(appShellMocks.useQuery).toHaveBeenCalledWith(
      expect.objectContaining({
        enabled: true,
        queryKey: ["quests", "alice"],
      }),
    );
    expect(screen.getByRole("link", { name: m["quests.galxeQuest.title"]() })).toHaveAttribute(
      "href",
      "https://app.galxe.com/quest/dango/GCNAXt8Tqv",
    );
    expect(
      screen.getByText(m["quests.galxeQuest.quest.connectEthereumWallet"]()),
    ).toBeInTheDocument();
    expect(
      screen.getByText(m["quests.galxeQuest.quest.completeTxsInEthereum"]({ number: 4 })),
    ).toBeInTheDocument();
    expect(screen.queryByRole("link", { name: "Claim NFT" })).not.toBeInTheDocument();

    const closeIcon = container.querySelector("svg.cursor-pointer");
    expect(closeIcon).toBeInTheDocument();

    fireEvent.click(closeIcon as SVGElement);

    expect(appShellMocks.setQuestBannerVisibility).toHaveBeenCalledWith(false);
  });

  it("renders the claim link when every quest is complete", () => {
    const ethAddress = "0x6574680000000000000000000000000000000000";
    appShellMocks.questData = {
      eth_address: ethAddress,
      trading_volumes: "0",
      tx_count: 10,
    };

    render(<QuestBanner />);

    expect(
      screen.getByText(`${m["quests.galxeQuest.quest.connectEthereumWallet"]()} (${ethAddress})`),
    ).toBeInTheDocument();
    expect(
      screen.getByText(m["quests.galxeQuest.quest.completeTxsInEthereum"]({ number: 10 })),
    ).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Claim NFT" })).toHaveAttribute(
      "href",
      "https://app.galxe.com/quest/dango/GCNAXt8Tqv",
    );
  });

  it("portals the quest banner into the desktop or mobile shell target", async () => {
    window.history.pushState({}, "", "/trade");
    appShellMocks.questData = {
      eth_address: "0x6574680000000000000000000000000000000000",
      trading_volumes: "0",
      tx_count: 10,
    };
    const desktopTarget = document.createElement("div");
    desktopTarget.id = "quest-banner";
    const mobileTarget = document.createElement("div");
    mobileTarget.id = "quest-banner-mobile";
    document.body.append(desktopTarget, mobileTarget);

    const { rerender } = render(<QuestBannerRender />);

    expect(desktopTarget).toHaveTextContent(m["quests.galxeQuest.title"]());
    expect(mobileTarget).toBeEmptyDOMElement();

    appShellMocks.isLg = false;
    rerender(<QuestBannerRender />);

    await waitFor(() => {
      expect(mobileTarget).toHaveTextContent(m["quests.galxeQuest.title"]());
    });
    expect(desktopTarget).toBeEmptyDOMElement();
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
