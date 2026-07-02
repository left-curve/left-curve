import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { Modals } from "@left-curve/applets-kit";
import { m } from "@left-curve/foundation/paraglide/messages.js";

import { PointsProfileTable } from "../src/components/points/profile/PointsProfileTable";

const pointsProfileMocks = vi.hoisted(() => ({
  navigate: vi.fn(),
  showModal: vi.fn(),
  useAccount: vi.fn(),
  useEpochPoints: vi.fn(),
  useUserPoints: vi.fn(),
}));

vi.mock("@left-curve/foundation", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@left-curve/foundation")>();

  return {
    ...actual,
    useApp: () => ({
      settings: {
        formatNumberOptions: {},
      },
      showModal: pointsProfileMocks.showModal,
    }),
  };
});

vi.mock("@left-curve/store", () => ({
  useAccount: pointsProfileMocks.useAccount,
  useEpochPoints: pointsProfileMocks.useEpochPoints,
}));

vi.mock("@tanstack/react-router", () => ({
  useNavigate: () => pointsProfileMocks.navigate,
}));

vi.mock("../src/components/points/useUserPoints", () => ({
  useUserPoints: pointsProfileMocks.useUserPoints,
}));

function epochPointsFixture(
  epoch: number,
  points: {
    perps?: string;
    referral?: string;
    vault?: string;
  },
  startedAt = String(1_700_000_000 + epoch * 1_000),
  endedAt = String(Number(startedAt) + 100),
) {
  return [
    epoch,
    {
      ended_at: endedAt,
      started_at: startedAt,
      stats: {
        points: {
          perps: points.perps ?? "0",
          referral: points.referral ?? "0",
          vault: points.vault ?? "0",
        },
      },
    },
  ] as const;
}

function setEpochPoints({
  epochPoints = [],
  isLoading = false,
}: {
  epochPoints?: unknown[];
  isLoading?: boolean;
} = {}) {
  pointsProfileMocks.useEpochPoints.mockReturnValue({
    epochPoints,
    isLoading,
  });
}

function setUserPoints({
  compensation,
}: {
  compensation?: {
    unrealized: string;
    vault: string;
  };
} = {}) {
  pointsProfileMocks.useUserPoints.mockReturnValue({
    compensation,
  });
}

function bodyRows() {
  return screen.getAllByRole("row").slice(1);
}

function rowText(row: HTMLElement) {
  return row.textContent ?? "";
}

function expectedDateRangePieces(startedAt: string, endedAt: string) {
  const start = new Date(Number.parseFloat(startedAt) * 1000);
  const end = new Date(Number.parseFloat(endedAt) * 1000);
  const dateOpts: Intl.DateTimeFormatOptions = { month: "short", day: "numeric" };
  const timeOpts: Intl.DateTimeFormatOptions = {
    hour: "2-digit",
    hour12: false,
    minute: "2-digit",
  };

  return [
    start.toLocaleDateString("en-US", dateOpts),
    start.toLocaleTimeString("en-US", timeOpts),
    end.toLocaleDateString("en-US", dateOpts),
    end.toLocaleTimeString("en-US", timeOpts),
  ];
}

describe("PointsProfileTable", () => {
  beforeEach(() => {
    Object.defineProperty(window, "dango", {
      configurable: true,
      value: {
        urls: {
          pointsUrl: "https://points.example",
        },
      },
    });

    pointsProfileMocks.useAccount.mockReturnValue({
      userIndex: 7,
    });
    setEpochPoints();
    setUserPoints();
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("requests epoch points for the connected user and renders the loading state", () => {
    setEpochPoints({
      isLoading: true,
    });

    render(<PointsProfileTable />);

    expect(pointsProfileMocks.useEpochPoints).toHaveBeenCalledWith({
      pointsUrl: "https://points.example",
      userIndex: 7,
    });
    expect(screen.getByText(m["points.profile.loading"]())).toBeInTheDocument();
  });

  it("preserves backend user index zero when requesting epoch points", () => {
    pointsProfileMocks.useAccount.mockReturnValue({
      userIndex: 0,
    });
    setEpochPoints({
      isLoading: true,
    });

    render(<PointsProfileTable />);

    expect(pointsProfileMocks.useEpochPoints).toHaveBeenCalledWith({
      pointsUrl: "https://points.example",
      userIndex: 0,
    });
    expect(screen.getByText(m["points.profile.loading"]())).toBeInTheDocument();
  });

  it("renders compensation and positive epoch totals in date order and shares a row payload", () => {
    setUserPoints({
      compensation: {
        unrealized: "15",
        vault: "10",
      },
    });
    setEpochPoints({
      epochPoints: [
        epochPointsFixture(3, {
          perps: "3",
          referral: "4",
          vault: "2",
        }),
        epochPointsFixture(4, {
          perps: "10",
          referral: "3",
          vault: "7",
        }),
        epochPointsFixture(5, {
          perps: "0",
          referral: "0",
          vault: "0",
        }),
      ],
    });

    const bubblingClick = vi.fn();
    const bubblingMouseDown = vi.fn();
    const bubblingPointerDown = vi.fn();

    render(
      <div
        onClick={bubblingClick}
        onMouseDown={bubblingMouseDown}
        onPointerDown={bubblingPointerDown}
      >
        <PointsProfileTable />
      </div>,
    );

    const rows = bodyRows();
    expect(rows).toHaveLength(3);
    expect(rowText(rows[0])).toContain(m["points.profile.epochLabel"]({ number: "4" }));
    expect(rowText(rows[0])).toContain(m["points.profile.xPoints"]({ points: "20" }));
    expect(rowText(rows[1])).toContain(m["points.profile.epochLabel"]({ number: "3" }));
    expect(rowText(rows[1])).toContain(m["points.profile.xPoints"]({ points: "9" }));
    expect(rowText(rows[2])).toContain(m["points.profile.epochLabel"]({ number: "0" }));
    expect(rowText(rows[2])).toContain(m["points.profile.compensation"]());
    expect(rowText(rows[2])).toContain(m["points.profile.xPoints"]({ points: "25" }));

    const shareButton = within(rows[2]).getByRole("button", {
      name: m["points.profile.columns.share"](),
    });

    fireEvent.pointerDown(shareButton);
    fireEvent.mouseDown(shareButton);
    fireEvent.click(shareButton);

    expect(pointsProfileMocks.showModal).toHaveBeenCalledWith(Modals.PointsShare, {
      points: 25,
      weekNumber: 0,
    });
    expect(bubblingClick).not.toHaveBeenCalled();
    expect(bubblingMouseDown).not.toHaveBeenCalled();
    expect(bubblingPointerDown).not.toHaveBeenCalled();
  });

  it("renders epoch date ranges from backend second timestamps", () => {
    const startedAt = "1700010000.5";
    const endedAt = "1700017200.25";
    setEpochPoints({
      epochPoints: [
        epochPointsFixture(
          6,
          {
            referral: "2.25",
            vault: "1.5",
          },
          startedAt,
          endedAt,
        ),
      ],
    });

    render(<PointsProfileTable />);

    const [row] = bodyRows();
    expect(rowText(row)).toContain(m["points.profile.epochLabel"]({ number: "6" }));
    expect(rowText(row)).toContain(m["points.profile.xPoints"]({ points: "3.75" }));
    for (const piece of expectedDateRangePieces(startedAt, endedAt)) {
      expect(rowText(row)).toContain(piece);
    }
  });

  it("preserves backend epoch zero rows with positive point totals", () => {
    const startedAt = "1700000000";
    const endedAt = "1700003600";
    setEpochPoints({
      epochPoints: [
        epochPointsFixture(
          0,
          {
            perps: "1.5",
            referral: "2.5",
            vault: "0",
          },
          startedAt,
          endedAt,
        ),
      ],
    });

    render(<PointsProfileTable />);

    const [row] = bodyRows();
    expect(rowText(row)).toContain(m["points.profile.epochLabel"]({ number: "0" }));
    expect(rowText(row)).toContain(m["points.profile.xPoints"]({ points: "4" }));
    expect(rowText(row)).not.toContain(m["points.profile.compensation"]());
    for (const piece of expectedDateRangePieces(startedAt, endedAt)) {
      expect(rowText(row)).toContain(piece);
    }
  });

  it("sorts by point totals, resets to the first page, and paginates history", () => {
    setEpochPoints({
      epochPoints: Array.from({ length: 12 }, (_, index) => {
        const epoch = index + 1;
        return epochPointsFixture(epoch, {
          perps: String(13 - epoch),
        });
      }),
    });

    render(<PointsProfileTable />);

    expect(rowText(bodyRows()[0])).toContain(m["points.profile.epochLabel"]({ number: "12" }));

    fireEvent.click(screen.getByRole("button", { name: m["points.profile.columns.points"]() }));

    expect(rowText(bodyRows()[0])).toContain(m["points.profile.epochLabel"]({ number: "1" }));

    fireEvent.click(screen.getByRole("button", { name: "2" }));

    expect(rowText(bodyRows()[0])).toContain(m["points.profile.epochLabel"]({ number: "11" }));

    fireEvent.click(screen.getByRole("button", { name: m["points.profile.columns.points"]() }));

    expect(rowText(bodyRows()[0])).toContain(m["points.profile.epochLabel"]({ number: "12" }));
  });

  it("shows the empty state and routes users to trade when no point history exists", () => {
    setUserPoints({
      compensation: {
        unrealized: "0",
        vault: "0",
      },
    });
    setEpochPoints({
      epochPoints: [
        epochPointsFixture(8, {
          perps: "0",
          referral: "0",
          vault: "0",
        }),
      ],
    });

    render(<PointsProfileTable />);

    expect(screen.getByText(m["points.profile.noHistory"]())).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: m["points.profile.getStarted"]() }));

    expect(pointsProfileMocks.navigate).toHaveBeenCalledWith({
      to: "/trade",
    });
  });
});
