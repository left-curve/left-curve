import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import { resetAppletsKitMocks } from "./mocks/applets-kit";

import { PointsProgressBar } from "../src/components/points/rewards/PointsProgressBar";

const progressBarMocks = vi.hoisted(() => ({
  scrollTo: vi.fn(),
  useAccount: vi.fn(),
}));

class MockIntersectionObserver {
  disconnect = vi.fn();
  observe = vi.fn();
  unobserve = vi.fn();
}

vi.mock("@left-curve/store", () => ({
  useAccount: progressBarMocks.useAccount,
}));

function setAccount({ isConnected = true }: { isConnected?: boolean } = {}) {
  progressBarMocks.useAccount.mockReturnValue({
    isConnected,
  });
}

describe("PointsProgressBar", () => {
  beforeEach(() => {
    resetAppletsKitMocks();
    Object.defineProperty(globalThis, "IntersectionObserver", {
      configurable: true,
      value: MockIntersectionObserver,
    });
    Object.defineProperty(HTMLElement.prototype, "clientWidth", {
      configurable: true,
      get: () => 400,
    });
    Object.defineProperty(HTMLElement.prototype, "scrollWidth", {
      configurable: true,
      get: () => 4300,
    });
    Object.defineProperty(HTMLElement.prototype, "scrollTo", {
      configurable: true,
      value: progressBarMocks.scrollTo,
    });

    progressBarMocks.scrollTo.mockImplementation(function scrollTo(
      this: HTMLElement,
      options: ScrollToOptions,
    ) {
      this.scrollLeft = Number(options.left ?? 0);
    });
    setAccount();
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("renders a connected user's current milestone window and next reward target", async () => {
    render(<PointsProgressBar currentVolume={95_000} />);

    expect(screen.queryByAltText("Start")).not.toBeInTheDocument();
    expect(screen.getByText("$50K")).toBeInTheDocument();
    expect(screen.getByText("$75K")).toBeInTheDocument();
    expect(screen.getByText("$100K")).toBeInTheDocument();
    expect(screen.getByText("$250K")).toBeInTheDocument();
    expect(screen.getByText("$500K")).toBeInTheDocument();

    expect(
      screen.getByText(
        m["points.rewards.boxes.volumeUntilNext"]({
          amount: "$5,000.00",
          tier: m["points.rewards.boxes.tiers.silver"](),
        }),
      ),
    ).toBeInTheDocument();

    await waitFor(() => {
      expect(progressBarMocks.scrollTo).toHaveBeenCalledWith(
        expect.objectContaining({
          behavior: "auto",
          left: expect.any(Number),
        }),
      );
    });
    expect(Number(progressBarMocks.scrollTo.mock.calls[0][0].left)).toBeGreaterThan(0);
  });

  it("keeps the start flag visible before the first milestone and targets a bronze box", () => {
    render(<PointsProgressBar currentVolume={0} />);

    expect(screen.getByAltText("Start")).toBeInTheDocument();
    expect(screen.getByText("$0")).toBeInTheDocument();
    expect(screen.getByText("$25K")).toBeInTheDocument();
    expect(screen.getByText("$100K")).toBeInTheDocument();

    expect(
      screen.getByText(
        m["points.rewards.boxes.volumeUntilNext"]({
          amount: "$25,000",
          tier: m["points.rewards.boxes.tiers.bronze"](),
        }),
      ),
    ).toBeInTheDocument();
    expect(progressBarMocks.scrollTo).toHaveBeenCalledWith(
      expect.objectContaining({
        left: 0,
      }),
    );
  });

  it("targets the next reward tier when backend volume is exactly on a milestone", () => {
    render(<PointsProgressBar currentVolume={100_000} />);

    expect(screen.queryByAltText("Start")).not.toBeInTheDocument();
    expect(screen.getByText("$75K")).toBeInTheDocument();
    expect(screen.getByText("$100K")).toBeInTheDocument();
    expect(screen.getByText("$125K")).toBeInTheDocument();

    expect(
      screen.getByText(
        m["points.rewards.boxes.volumeUntilNext"]({
          amount: "$25,000",
          tier: m["points.rewards.boxes.tiers.bronze"](),
        }),
      ),
    ).toBeInTheDocument();
  });

  it("shows disconnected messaging while preserving the same milestone labels", () => {
    setAccount({
      isConnected: false,
    });

    render(<PointsProgressBar currentVolume={125_000} />);

    expect(screen.getByText(m["points.rewards.boxes.notLoggedIn"]())).toBeInTheDocument();
    expect(screen.getByText("$100K")).toBeInTheDocument();
    expect(screen.getByText("$125K")).toBeInTheDocument();
    expect(screen.getByText("$250K")).toBeInTheDocument();
    expect(screen.queryByText(/\$25,000/)).not.toBeInTheDocument();
  });

  it("formats million-scale backend volume milestones and targets the next cyclic tier", () => {
    render(<PointsProgressBar currentVolume={1_225_000} />);

    expect(screen.getByText("$1.2M")).toBeInTheDocument();
    expect(screen.getByText("$1.225M")).toBeInTheDocument();
    expect(screen.getByText("$1.25M")).toBeInTheDocument();
    expect(
      screen.getByText(
        m["points.rewards.boxes.volumeUntilNext"]({
          amount: "$25,000",
          tier: m["points.rewards.boxes.tiers.gold"](),
        }),
      ),
    ).toBeInTheDocument();
  });
});
