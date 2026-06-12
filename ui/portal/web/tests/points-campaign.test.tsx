import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import type React from "react";

import { PointsCampaign } from "../src/components/points/PointsCampaign";

type PointsCampaignTab = "profile" | "rewards" | "leaderboard";

const pointsCampaignMocks = vi.hoisted(() => ({
  chestProviderProps: [] as Array<{
    huntedBoosters: unknown;
    huntedBoxes: unknown;
    unopenedBoxes: unknown;
    userIndex: number | undefined;
  }>,
  open: vi.fn(),
  useAccount: vi.fn(),
  useBoosters: vi.fn(),
  useBoxes: vi.fn(),
  useCurrentEpoch: vi.fn(),
  usePoints: vi.fn(),
}));

class MockResizeObserver {
  disconnect = vi.fn();
  observe = vi.fn();
  unobserve = vi.fn();
}

vi.mock("../src/components/foundation/MobileTitle", () => ({
  MobileTitle: ({ title }: { title: string }) => <div data-testid="mobile-title">{title}</div>,
}));

vi.mock("../src/components/foundation/PageGlow", () => ({
  PageGlow: () => <div data-testid="page-glow" />,
}));

vi.mock("../src/components/points/PointsHeader", () => ({
  PointsHeader: () => <div data-testid="points-header" />,
}));

vi.mock("../src/components/points/leaderboard", () => ({
  LeaderboardTable: () => <section data-testid="leaderboard-table" />,
  RecentHuntDropsTable: () => <section data-testid="recent-hunt-drops-table" />,
}));

vi.mock("../src/components/points/profile", () => ({
  LigueLevels: () => <section data-testid="ligue-levels" />,
  PointsProfileTable: () => <section data-testid="points-profile-table" />,
}));

vi.mock("../src/components/points/rewards", () => ({
  BoostersSection: ({
    currentEpoch,
    currentEpochEndsAt,
    huntedBoosters,
  }: {
    currentEpoch: number | undefined;
    currentEpochEndsAt: Date | undefined;
    huntedBoosters: unknown;
  }) => (
    <section
      data-current-epoch={currentEpoch}
      data-hunted-boosters={JSON.stringify(huntedBoosters)}
      data-testid="boosters-section"
    >
      {currentEpochEndsAt?.toISOString()}
    </section>
  ),
  BoxesSection: ({ unopenedBoxes }: { unopenedBoxes: unknown }) => (
    <section data-testid="boxes-section">{JSON.stringify(unopenedBoxes)}</section>
  ),
  ChestOpeningProvider: ({
    children,
    huntedBoosters,
    huntedBoxes,
    unopenedBoxes,
    userIndex,
  }: React.PropsWithChildren<{
    huntedBoosters: unknown;
    huntedBoxes: unknown;
    unopenedBoxes: unknown;
    userIndex: number | undefined;
  }>) => {
    pointsCampaignMocks.chestProviderProps.push({
      huntedBoosters,
      huntedBoxes,
      unopenedBoxes,
      userIndex,
    });

    return <div data-testid="chest-opening-provider">{children}</div>;
  },
  NFTsSection: ({ nfts }: { nfts: unknown }) => (
    <section data-testid="nfts-section">{JSON.stringify(nfts)}</section>
  ),
  PointsProgressBar: ({ currentVolume }: { currentVolume: number }) => (
    <section data-testid="points-progress-bar">{currentVolume}</section>
  ),
}));

vi.mock("@left-curve/store", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@left-curve/store")>();

  return {
    ...actual,
    useAccount: pointsCampaignMocks.useAccount,
    useBoosters: pointsCampaignMocks.useBoosters,
    useBoxes: pointsCampaignMocks.useBoxes,
    useCurrentEpoch: pointsCampaignMocks.useCurrentEpoch,
    usePoints: pointsCampaignMocks.usePoints,
  };
});

function setAccount(
  options: Partial<{
    isConnected: boolean;
    userIndex: number | undefined;
  }> = {},
) {
  const { isConnected = true } = options;
  const userIndex = "userIndex" in options ? options.userIndex : isConnected ? 42 : undefined;

  pointsCampaignMocks.useAccount.mockReturnValue({
    isConnected,
    userIndex,
  });
}

function setPoints({ volume = 123_456 }: Partial<{ volume: number }> = {}) {
  pointsCampaignMocks.usePoints.mockReturnValue({
    compensation: undefined,
    isLoading: false,
    lpPoints: 10,
    percentile: 84,
    pnl: 0,
    points: 25,
    rank: 7,
    referralPoints: 5,
    tradingPoints: 10,
    volume,
  });
}

function setBoxes() {
  pointsCampaignMocks.useBoxes.mockReturnValue({
    huntedBoxes: [
      {
        chest: "gold",
        epoch: 8,
        loot: "golden_shell",
        opened: false,
      },
    ],
    nfts: {
      gold: {
        rare: 2,
      },
    },
    unopenedBoxes: {
      gold: {
        rare: 2,
      },
    },
    unopenedCounts: {
      gold: 2,
    },
  });
}

function setBoosters() {
  pointsCampaignMocks.useBoosters.mockReturnValue({
    huntedBoosters: [
      {
        epoch: 8,
        loot: "golden_shell",
        multiplier: "2",
      },
    ],
  });
}

function setCurrentEpoch() {
  pointsCampaignMocks.useCurrentEpoch.mockReturnValue({
    currentEpoch: 9,
    endDate: new Date("2026-06-09T12:00:00.000Z"),
  });
}

function renderCampaign({
  activeTab = "profile",
  onTabChange = vi.fn(),
}: Partial<{
  activeTab: PointsCampaignTab;
  onTabChange: (tab: PointsCampaignTab) => void;
}> = {}) {
  render(
    <PointsCampaign activeTab={activeTab} onTabChange={onTabChange}>
      <PointsCampaign.Header />
      <PointsCampaign.Tabs />
    </PointsCampaign>,
  );

  return { onTabChange };
}

function hiddenTabWrapper(testId: string) {
  return screen.getByTestId(testId).closest(".hidden");
}

describe("PointsCampaign", () => {
  beforeEach(() => {
    vi.stubGlobal("ResizeObserver", MockResizeObserver);
    vi.stubGlobal("open", pointsCampaignMocks.open);

    Object.defineProperty(window, "dango", {
      configurable: true,
      value: {
        urls: {
          pointsUrl: "https://points.example",
        },
      },
    });

    pointsCampaignMocks.chestProviderProps = [];
    setAccount();
    setPoints();
    setBoxes();
    setBoosters();
    setCurrentEpoch();
  });

  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
    vi.clearAllMocks();
  });

  it("wires account-scoped points and loot data into the campaign shell", () => {
    renderCampaign();

    expect(screen.getByTestId("mobile-title")).toHaveTextContent(m["points.mobileTitle"]());
    expect(pointsCampaignMocks.usePoints).toHaveBeenCalledWith({
      pointsUrl: "https://points.example",
      userIndex: 42,
    });
    expect(pointsCampaignMocks.useBoxes).toHaveBeenCalledWith({
      pointsUrl: "https://points.example",
      userIndex: 42,
    });
    expect(pointsCampaignMocks.useBoosters).toHaveBeenCalledWith({
      pointsUrl: "https://points.example",
      userIndex: 42,
    });
    expect(pointsCampaignMocks.useCurrentEpoch).toHaveBeenCalledWith({
      pointsUrl: "https://points.example",
    });
    expect(pointsCampaignMocks.chestProviderProps).toEqual([
      {
        huntedBoosters: [
          {
            epoch: 8,
            loot: "golden_shell",
            multiplier: "2",
          },
        ],
        huntedBoxes: [
          {
            chest: "gold",
            epoch: 8,
            loot: "golden_shell",
            opened: false,
          },
        ],
        unopenedBoxes: {
          gold: {
            rare: 2,
          },
        },
        userIndex: 42,
      },
    ]);
    expect(screen.getByTestId("points-progress-bar")).toHaveTextContent("123456");
    expect(screen.getByTestId("boxes-section")).toHaveTextContent('{"gold":2}');
    expect(screen.getByTestId("boosters-section")).toHaveAttribute("data-current-epoch", "9");
  });

  it("preserves backend user index zero across points campaign hooks", () => {
    setAccount({
      userIndex: 0,
    });

    renderCampaign({ activeTab: "rewards" });

    expect(pointsCampaignMocks.usePoints).toHaveBeenCalledWith({
      pointsUrl: "https://points.example",
      userIndex: 0,
    });
    expect(pointsCampaignMocks.useBoxes).toHaveBeenCalledWith({
      pointsUrl: "https://points.example",
      userIndex: 0,
    });
    expect(pointsCampaignMocks.useBoosters).toHaveBeenCalledWith({
      pointsUrl: "https://points.example",
      userIndex: 0,
    });
    expect(pointsCampaignMocks.chestProviderProps).toEqual([
      expect.objectContaining({
        userIndex: 0,
      }),
    ]);
  });

  it("routes reward-tab hook data into inventory, boosters, and chest opening sections", () => {
    const huntedBoosters = [
      {
        epoch: 11,
        loot: "pearl_dango",
        multiplier: "2.75",
      },
    ];
    const huntedBoxes = [
      {
        chest: "crystal",
        epoch: 11,
        loot: "pearl_dango",
        opened: false,
      },
    ];
    const nfts = [
      {
        quantity: 3,
        rarity: "legendary",
      },
    ];
    const unopenedBoxes = {
      crystal: {
        mythic: 1,
      },
      silver: {
        common: 2,
      },
    };
    const unopenedCounts = {
      crystal: 1,
      silver: 2,
    };

    setPoints({ volume: 987_654 });
    pointsCampaignMocks.useBoxes.mockReturnValue({
      huntedBoxes,
      nfts,
      unopenedBoxes,
      unopenedCounts,
    });
    pointsCampaignMocks.useBoosters.mockReturnValue({
      huntedBoosters,
    });
    pointsCampaignMocks.useCurrentEpoch.mockReturnValue({
      currentEpoch: 11,
      endDate: new Date("2026-06-10T18:45:00.000Z"),
    });

    renderCampaign({ activeTab: "rewards" });

    expect(hiddenTabWrapper("points-progress-bar")).not.toBeInTheDocument();
    expect(screen.getByTestId("points-progress-bar")).toHaveTextContent("987654");
    expect(screen.getByTestId("boxes-section")).toHaveTextContent(JSON.stringify(unopenedCounts));
    expect(screen.getByTestId("nfts-section")).toHaveTextContent(JSON.stringify(nfts));
    expect(screen.getByTestId("boosters-section")).toHaveAttribute("data-current-epoch", "11");
    expect(screen.getByTestId("boosters-section")).toHaveTextContent("2026-06-10T18:45:00.000Z");
    expect(screen.getByTestId("boosters-section")).toHaveAttribute(
      "data-hunted-boosters",
      JSON.stringify(huntedBoosters),
    );
    expect(pointsCampaignMocks.chestProviderProps).toEqual([
      {
        huntedBoosters,
        huntedBoxes,
        unopenedBoxes,
        userIndex: 42,
      },
    ]);
  });

  it("opens the points rules from the header", () => {
    renderCampaign();

    fireEvent.click(screen.getByRole("button", { name: m["points.header.readRules"]() }));

    expect(pointsCampaignMocks.open).toHaveBeenCalledWith(
      "https://dango-4.gitbook.io/dango-docs/points",
    );
  });

  it("delegates tab changes and keeps visibility controlled by the active tab prop", () => {
    const { onTabChange } = renderCampaign({
      activeTab: "rewards",
    });

    expect(hiddenTabWrapper("ligue-levels")).toBeInTheDocument();
    expect(hiddenTabWrapper("points-progress-bar")).not.toBeInTheDocument();
    expect(hiddenTabWrapper("leaderboard-table")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: m["points.tabs.profile"]() }));
    fireEvent.click(screen.getByRole("button", { name: m["points.tabs.leaderboard"]() }));

    expect(onTabChange).toHaveBeenNthCalledWith(1, "profile");
    expect(onTabChange).toHaveBeenNthCalledWith(2, "leaderboard");
  });

  it("does not render point history for disconnected accounts", () => {
    setAccount({
      isConnected: false,
      userIndex: undefined,
    });

    renderCampaign();

    expect(pointsCampaignMocks.usePoints).toHaveBeenCalledWith({
      pointsUrl: "https://points.example",
      userIndex: undefined,
    });
    expect(screen.getByTestId("ligue-levels")).toBeInTheDocument();
    expect(screen.queryByTestId("points-profile-table")).not.toBeInTheDocument();
  });
});
