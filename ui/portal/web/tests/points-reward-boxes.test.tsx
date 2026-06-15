import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import { BoxesSection } from "../src/components/points/rewards/BoxesSection";

const rewardBoxesMocks = vi.hoisted(() => ({
  openAllChests: vi.fn(),
  openChest: vi.fn(),
  useAccount: vi.fn(),
}));

vi.mock("@left-curve/store", () => ({
  useAccount: rewardBoxesMocks.useAccount,
}));

vi.mock("../src/components/points/rewards/useChestOpening", () => ({
  useChestOpening: () => ({
    openAllChests: rewardBoxesMocks.openAllChests,
    openChest: rewardBoxesMocks.openChest,
  }),
}));

const boxLabels = {
  bronze: () => m["points.rewards.boxes.tiers.bronze"](),
  crystal: () => m["points.rewards.boxes.tiers.crystal"](),
  gold: () => m["points.rewards.boxes.tiers.gold"](),
  silver: () => m["points.rewards.boxes.tiers.silver"](),
};

function actionButtons() {
  return screen.getAllByRole("button").filter((button) => {
    const name = button.textContent ?? "";
    return (
      name === m["points.rewards.boxes.open"]() || name === m["points.rewards.boxes.openAll"]()
    );
  });
}

describe("BoxesSection", () => {
  beforeEach(() => {
    rewardBoxesMocks.useAccount.mockReturnValue({
      isConnected: true,
    });
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("renders every box tier with backend-fed quantities and enables only valid actions", () => {
    render(
      <BoxesSection
        unopenedBoxes={{
          gold: 2,
          silver: 1,
        }}
      />,
    );

    expect(screen.getByText(m["points.rewards.boxes.title"]())).toBeInTheDocument();
    for (const label of Object.values(boxLabels)) {
      expect(screen.getByAltText(`${label()} chest`)).toBeInTheDocument();
      expect(screen.getByText(label())).toBeInTheDocument();
    }
    expect(screen.getByText("x1")).toBeInTheDocument();
    expect(screen.getByText("x2")).toBeInTheDocument();

    const buttons = actionButtons();
    expect(buttons).toHaveLength(8);

    expect(buttons[0]).toBeDisabled();
    expect(buttons[1]).toBeDisabled();
    expect(buttons[2]).toBeEnabled();
    expect(buttons[3]).toBeDisabled();
    expect(buttons[4]).toBeEnabled();
    expect(buttons[5]).toBeEnabled();
    expect(buttons[6]).toBeDisabled();
    expect(buttons[7]).toBeDisabled();

    fireEvent.click(buttons[2]);
    fireEvent.click(buttons[5]);

    expect(rewardBoxesMocks.openChest).toHaveBeenCalledWith("silver");
    expect(rewardBoxesMocks.openAllChests).toHaveBeenCalledWith("gold");
  });

  it("treats explicit backend zero box quantities as locked tiers", () => {
    render(
      <BoxesSection
        unopenedBoxes={{
          bronze: 0,
          crystal: 0,
          gold: 0,
          silver: 0,
        }}
      />,
    );

    expect(screen.queryByText("x0")).not.toBeInTheDocument();

    for (const button of actionButtons()) {
      expect(button).toBeDisabled();
      fireEvent.click(button);
    }

    expect(rewardBoxesMocks.openChest).not.toHaveBeenCalled();
    expect(rewardBoxesMocks.openAllChests).not.toHaveBeenCalled();
  });

  it("keeps zero quantity tiers locked while positive tiers stay actionable", () => {
    render(
      <BoxesSection
        unopenedBoxes={{
          bronze: 0,
          crystal: 1,
          gold: 0,
          silver: 2,
        }}
      />,
    );

    expect(screen.queryByText("x0")).not.toBeInTheDocument();
    expect(screen.getByText("x1")).toBeInTheDocument();
    expect(screen.getByText("x2")).toBeInTheDocument();

    const buttons = actionButtons();
    expect(buttons).toHaveLength(8);

    expect(buttons[0]).toBeDisabled();
    expect(buttons[1]).toBeDisabled();
    expect(buttons[2]).toBeEnabled();
    expect(buttons[3]).toBeEnabled();
    expect(buttons[4]).toBeDisabled();
    expect(buttons[5]).toBeDisabled();
    expect(buttons[6]).toBeEnabled();
    expect(buttons[7]).toBeDisabled();

    for (const button of buttons) {
      fireEvent.click(button);
    }

    expect(rewardBoxesMocks.openChest).toHaveBeenCalledTimes(2);
    expect(rewardBoxesMocks.openChest).toHaveBeenNthCalledWith(1, "silver");
    expect(rewardBoxesMocks.openChest).toHaveBeenNthCalledWith(2, "crystal");
    expect(rewardBoxesMocks.openAllChests).toHaveBeenCalledTimes(1);
    expect(rewardBoxesMocks.openAllChests).toHaveBeenCalledWith("silver");
  });

  it("locks all boxes for disconnected users even when backend quantities exist", () => {
    rewardBoxesMocks.useAccount.mockReturnValue({
      isConnected: false,
    });

    render(
      <BoxesSection
        unopenedBoxes={{
          bronze: 3,
          crystal: 4,
          gold: 2,
          silver: 1,
        }}
      />,
    );

    expect(screen.queryByText("x1")).not.toBeInTheDocument();
    expect(screen.queryByText("x2")).not.toBeInTheDocument();
    expect(screen.queryByText("x3")).not.toBeInTheDocument();
    expect(screen.queryByText("x4")).not.toBeInTheDocument();

    for (const button of actionButtons()) {
      expect(button).toBeDisabled();
      fireEvent.click(button);
    }

    expect(rewardBoxesMocks.openChest).not.toHaveBeenCalled();
    expect(rewardBoxesMocks.openAllChests).not.toHaveBeenCalled();
  });
});
