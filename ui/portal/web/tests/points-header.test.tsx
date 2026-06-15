import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";
import { Modals } from "@left-curve/applets-kit";

import { PointsHeader } from "../src/components/points/PointsHeader";

function escapedPattern(text: string) {
  return text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

const pointsHeaderMocks = vi.hoisted(() => {
  const subscribe = vi.fn();

  return {
    countdown: {
      days: "0",
      hours: "1",
      minutes: "2",
      seconds: "3",
    },
    blockListener: undefined as ((event: { blockHeight: number }) => void) | undefined,
    refetchCurrentEpoch: vi.fn(),
    showModal: vi.fn(),
    subscribe,
    subscriptions: {
      subscribe,
    },
    unsubscribe: vi.fn(),
    useAccount: vi.fn(),
    useCurrentEpoch: vi.fn(),
    usePredictPoints: vi.fn(),
    useUserPoints: vi.fn(),
  };
});

vi.mock("@left-curve/foundation", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@left-curve/foundation")>();

  return {
    ...actual,
    useApp: () => ({
      settings: {
        formatNumberOptions: {},
      },
      showModal: pointsHeaderMocks.showModal,
      subscriptions: pointsHeaderMocks.subscriptions,
    }),
    useCountdown: () => pointsHeaderMocks.countdown,
  };
});

vi.mock("@left-curve/store", () => ({
  useAccount: pointsHeaderMocks.useAccount,
  useCurrentEpoch: pointsHeaderMocks.useCurrentEpoch,
  usePredictPoints: pointsHeaderMocks.usePredictPoints,
}));

vi.mock("../src/components/points/useUserPoints", () => ({
  useUserPoints: pointsHeaderMocks.useUserPoints,
}));

function setAccount(options: { isConnected?: boolean; userIndex?: number } = {}) {
  const { isConnected = true } = options;
  const userIndex = "userIndex" in options ? options.userIndex : 7;

  pointsHeaderMocks.useAccount.mockReturnValue({
    isConnected,
    userIndex,
  });
}

function setUserPoints({
  lpPoints = 200,
  points = 1234,
  rank = 8,
  referralPoints = 50,
  tradingPoints = 100,
  volume = 55000,
}: Partial<{
  lpPoints: number;
  points: number;
  rank: number;
  referralPoints: number;
  tradingPoints: number;
  volume: number;
}> = {}) {
  pointsHeaderMocks.useUserPoints.mockReturnValue({
    lpPoints,
    points,
    rank,
    referralPoints,
    tradingPoints,
    volume,
  });
}

function setCurrentEpoch({
  currentEpoch = 9,
  endDate = new Date("2026-06-09T12:00:00Z"),
  isStarted = true,
  startsAt,
}: Partial<{
  currentEpoch: number | undefined;
  endDate: Date | undefined;
  isStarted: boolean;
  startsAt: { block: number } | { timestamp: string } | undefined;
}> = {}) {
  pointsHeaderMocks.useCurrentEpoch.mockReturnValue({
    currentEpoch,
    endDate,
    isStarted,
    refetch: pointsHeaderMocks.refetchCurrentEpoch,
    startsAt,
  });
}

function setPredictedPoints({
  perps = "40",
  referral = "5",
  vault = "30",
}: Partial<{
  perps: string;
  referral: string;
  vault: string;
}> = {}) {
  pointsHeaderMocks.usePredictPoints.mockReturnValue({
    predictedPoints: {
      stats: {
        points: {
          perps,
          referral,
          vault,
        },
      },
    },
  });
}

describe("PointsHeader", () => {
  beforeEach(() => {
    pointsHeaderMocks.countdown = {
      days: "0",
      hours: "1",
      minutes: "2",
      seconds: "3",
    };
    pointsHeaderMocks.blockListener = undefined;
    pointsHeaderMocks.subscribe.mockImplementation((_topic, { listener }) => {
      pointsHeaderMocks.blockListener = listener;
      return pointsHeaderMocks.unsubscribe;
    });
    setAccount();
    setUserPoints();
    setCurrentEpoch();
    setPredictedPoints();
    Object.defineProperty(window, "dango", {
      configurable: true,
      value: {
        urls: {
          pointsUrl: "https://points.example",
        },
      },
    });
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("renders connected point totals, predicted points, and opens the share modal", () => {
    render(<PointsHeader />);

    expect(pointsHeaderMocks.useCurrentEpoch).toHaveBeenCalledWith({
      pointsUrl: "https://points.example",
    });
    expect(pointsHeaderMocks.usePredictPoints).toHaveBeenCalledWith({
      enabled: true,
      pointsUrl: "https://points.example",
      userIndex: 7,
    });

    expect(screen.getByText(m["points.header.myPoints"]()).parentElement).toHaveTextContent(
      "1,234",
    );
    expect(screen.getByText(m["points.header.myVolume"]()).parentElement).toHaveTextContent(
      "$55,000",
    );
    expect(screen.getByText(m["points.header.myRank"]()).parentElement).toHaveTextContent("#8");
    expect(screen.getByText(`${m["points.header.currentEpoch"]()} 9`)).toBeInTheDocument();
    expect(screen.getByText(`${m["points.header.endsIn"]()} 1h 2m 3s`)).toBeInTheDocument();

    expect(screen.getByText(m["points.header.earnedLabel"]())).toBeInTheDocument();
    expect(screen.getByText(m["points.header.predictedLabel"]())).toBeInTheDocument();
    for (const value of ["100", "200", "50", "40", "30", "5"]) {
      expect(screen.getByText(value)).toBeInTheDocument();
    }

    fireEvent.click(screen.getByRole("button", { name: m["points.header.share"]() }));

    expect(pointsHeaderMocks.showModal).toHaveBeenCalledWith(Modals.PointsShare, {
      points: 1234,
      weekNumber: 9,
    });
  });

  it("renders connected zero point metrics as real values instead of placeholders", () => {
    setUserPoints({
      lpPoints: 0,
      points: 0,
      rank: 0,
      referralPoints: 0,
      tradingPoints: 0,
      volume: 0,
    });
    setPredictedPoints({
      perps: "0",
      referral: "0",
      vault: "0",
    });

    render(<PointsHeader />);

    expect(pointsHeaderMocks.usePredictPoints).toHaveBeenCalledWith({
      enabled: true,
      pointsUrl: "https://points.example",
      userIndex: 7,
    });
    expect(screen.getByText(m["points.header.myPoints"]()).parentElement).toHaveTextContent("0");
    expect(screen.getByText(m["points.header.myVolume"]()).parentElement).toHaveTextContent("$0");
    expect(screen.getByText(m["points.header.myRank"]()).parentElement).toHaveTextContent("#0");
    expect(screen.getByText(m["points.header.earnedLabel"]()).parentElement).toHaveTextContent(
      new RegExp(`0\\s*${escapedPattern(m["points.header.points"]())}`),
    );
    expect(screen.queryByText(m["points.header.predictedLabel"]())).not.toBeInTheDocument();
    expect(screen.queryByText("--")).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: m["points.header.share"]() }));

    expect(pointsHeaderMocks.showModal).toHaveBeenCalledWith(Modals.PointsShare, {
      points: 0,
      weekNumber: 9,
    });
  });

  it("disables sharing and prediction fetches for disconnected users before the campaign starts", () => {
    setAccount({
      isConnected: false,
      userIndex: undefined,
    });
    setCurrentEpoch({
      currentEpoch: undefined,
      endDate: undefined,
      isStarted: false,
      startsAt: {
        timestamp: "1790000000",
      },
    });
    pointsHeaderMocks.countdown = {
      days: "0",
      hours: "0",
      minutes: "0",
      seconds: "4",
    };
    pointsHeaderMocks.usePredictPoints.mockReturnValue({
      predictedPoints: undefined,
    });

    render(<PointsHeader />);

    expect(pointsHeaderMocks.usePredictPoints).toHaveBeenCalledWith({
      enabled: false,
      pointsUrl: "https://points.example",
      userIndex: undefined,
    });
    expect(screen.getAllByText("--")).toHaveLength(3);
    expect(screen.getByText(`${m["points.header.currentEpoch"]()} --`)).toBeInTheDocument();
    expect(screen.getByText(`${m["points.header.startsIn"]()} 4s`)).toBeInTheDocument();
    expect(screen.queryByText(m["points.header.predictedLabel"]())).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: m["points.header.share"]() })).toBeDisabled();

    fireEvent.click(screen.getByRole("button", { name: m["points.header.share"]() }));

    expect(pointsHeaderMocks.showModal).not.toHaveBeenCalled();
  });

  it("refetches the current epoch once when a started epoch countdown reaches zero", async () => {
    pointsHeaderMocks.countdown = {
      days: "0",
      hours: "0",
      minutes: "0",
      seconds: "0",
    };

    render(<PointsHeader />);

    await waitFor(() => {
      expect(pointsHeaderMocks.refetchCurrentEpoch).toHaveBeenCalledOnce();
    });
  });

  it("subscribes to block updates for block-based future campaign starts", () => {
    setCurrentEpoch({
      currentEpoch: undefined,
      endDate: undefined,
      isStarted: false,
      startsAt: {
        block: 1234,
      },
    });

    const { unmount } = render(<PointsHeader />);

    expect(pointsHeaderMocks.subscribe).toHaveBeenCalledWith(
      "block",
      expect.objectContaining({
        listener: expect.any(Function),
      }),
    );

    unmount();

    expect(pointsHeaderMocks.unsubscribe).toHaveBeenCalledOnce();
  });

  it("refetches a block-based future campaign once the live block countdown reaches zero", async () => {
    setCurrentEpoch({
      currentEpoch: undefined,
      endDate: undefined,
      isStarted: false,
      startsAt: {
        block: 1234,
      },
    });
    pointsHeaderMocks.countdown = {
      days: "0",
      hours: "0",
      minutes: "0",
      seconds: "5",
    };

    render(<PointsHeader />);

    expect(pointsHeaderMocks.blockListener).toEqual(expect.any(Function));

    act(() => {
      pointsHeaderMocks.blockListener?.({ blockHeight: 1230 });
    });

    expect(pointsHeaderMocks.refetchCurrentEpoch).not.toHaveBeenCalled();

    pointsHeaderMocks.countdown = {
      days: "0",
      hours: "0",
      minutes: "0",
      seconds: "0",
    };

    act(() => {
      pointsHeaderMocks.blockListener?.({ blockHeight: 1234 });
    });

    await waitFor(() => {
      expect(pointsHeaderMocks.refetchCurrentEpoch).toHaveBeenCalledOnce();
    });
  });
});
