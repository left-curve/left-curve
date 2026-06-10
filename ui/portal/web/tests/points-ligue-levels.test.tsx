import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import { LigueLevels } from "../src/components/points/profile";
import { UserPointsProvider, type UserLeague } from "../src/components/points/useUserPoints";

const ligueLevelsMocks = vi.hoisted(() => ({
  useAccount: vi.fn(),
  usePoints: vi.fn(),
}));

vi.mock("@left-curve/store", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@left-curve/store")>();

  return {
    ...actual,
    useAccount: ligueLevelsMocks.useAccount,
    usePoints: ligueLevelsMocks.usePoints,
  };
});

function setAccount({
  isConnected = true,
  userIndex = 42,
}: Partial<{
  isConnected: boolean;
  userIndex: number | undefined;
}> = {}) {
  ligueLevelsMocks.useAccount.mockReturnValue({
    isConnected,
    userIndex,
  });
}

function setPoints({
  percentile = 84,
}: Partial<{
  percentile: number;
}> = {}) {
  ligueLevelsMocks.usePoints.mockReturnValue({
    compensation: undefined,
    isLoading: false,
    lpPoints: 10,
    percentile,
    pnl: 0,
    points: 25,
    rank: 7,
    referralPoints: 5,
    tradingPoints: 10,
    volume: 1000,
  });
}

function renderLigueLevels(currentLevel?: UserLeague) {
  return render(
    <UserPointsProvider>
      <LigueLevels currentLevel={currentLevel} />
    </UserPointsProvider>,
  );
}

function badge(level: UserLeague) {
  return screen.getByAltText(`${level} badge`);
}

function shineImage(container: HTMLElement) {
  return container.querySelector('img[src="/images/points/league-shine.png"]');
}

describe("LigueLevels", () => {
  beforeEach(() => {
    Object.defineProperty(window, "dango", {
      configurable: true,
      value: {
        urls: {
          pointsUrl: "https://points.example",
        },
      },
    });

    setAccount();
    setPoints();
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("derives the connected user's current league from backend percentile data", () => {
    const { container } = renderLigueLevels();

    expect(ligueLevelsMocks.usePoints).toHaveBeenCalledWith({
      pointsUrl: "https://points.example",
      userIndex: 42,
    });
    expect(screen.getByText(m["points.leagues.title"]())).toBeInTheDocument();
    expect(screen.getByText(m["points.leagues.subtitle"]())).toBeInTheDocument();
    expect(shineImage(container)).toBeInTheDocument();
    expect(badge("gold")).not.toHaveClass("grayscale");
    expect(badge("platinum")).not.toHaveClass("grayscale");
    expect(badge("diamond")).toHaveClass("grayscale");
  });

  it("treats every league as locked when the account is disconnected", () => {
    setAccount({
      isConnected: false,
      userIndex: undefined,
    });
    setPoints({
      percentile: 99,
    });

    const { container } = renderLigueLevels();

    expect(shineImage(container)).not.toBeInTheDocument();
    for (const level of [
      "wood",
      "iron",
      "gold",
      "platinum",
      "diamond",
      "master",
      "grandmaster",
    ] as UserLeague[]) {
      expect(badge(level)).toHaveClass("grayscale");
    }
  });

  it("lets callers override the displayed current league for preview states", () => {
    setPoints({
      percentile: 10,
    });

    const { container } = renderLigueLevels("master");

    expect(shineImage(container)).toBeInTheDocument();
    expect(badge("wood")).not.toHaveClass("grayscale");
    expect(badge("diamond")).not.toHaveClass("grayscale");
    expect(badge("master")).not.toHaveClass("grayscale");
    expect(badge("grandmaster")).toHaveClass("grayscale");
  });
});
