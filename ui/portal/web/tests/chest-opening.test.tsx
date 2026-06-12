import { QueryClientProvider } from "@tanstack/react-query";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { HuntedBooster, HuntedBoxEntry } from "@left-curve/store";

import {
  ChestOpeningProvider,
  useChestOpening,
} from "../src/components/points/rewards/useChestOpening";
import { createTestQueryClient } from "./utils/query-client";

const chestMocks = vi.hoisted(() => ({
  openBoxes: vi.fn(),
}));

vi.mock("@left-curve/store", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@left-curve/store")>();

  return {
    ...actual,
    openBoxes: chestMocks.openBoxes,
  };
});

vi.mock("../src/components/points/rewards/ChestOpeningOverlay", () => ({
  ChestOpeningOverlay: ({
    currentBoxIndex,
    isBulkMode,
    isOpenAllMode,
    onClose,
    onNext,
    onSpin,
    slot,
    totalBoxesToOpen,
    variant,
  }: {
    currentBoxIndex: number;
    isBulkMode: boolean;
    isOpenAllMode: boolean;
    onClose: () => void;
    onNext?: () => void;
    onSpin?: () => void;
    slot:
      | { kind: "fungible"; loot: string }
      | {
          kind: "hunted";
          loot: string;
          epoch: number;
          multiplier: string;
        }
      | null;
    totalBoxesToOpen: number;
    variant: string;
  }) => {
    const slotLabel =
      slot?.kind === "hunted"
        ? `${slot.kind}:${slot.loot}:${slot.epoch}:${slot.multiplier}`
        : `${slot?.kind ?? "none"}:${slot?.loot ?? "none"}`;

    return (
      <section
        data-testid="chest-overlay"
        data-slot={slotLabel}
        data-variant={variant}
        data-bulk={String(isBulkMode)}
        data-open-all={String(isOpenAllMode)}
        data-index={String(currentBoxIndex)}
        data-total={String(totalBoxesToOpen)}
      >
        <button type="button" onClick={onSpin}>
          spin
        </button>
        <button type="button" onClick={onNext}>
          next
        </button>
        <button type="button" onClick={onClose}>
          close
        </button>
      </section>
    );
  },
}));

type BoxVariant = "bronze" | "silver" | "gold" | "crystal";

function ChestControls({ variant = "silver" }: { variant?: BoxVariant }) {
  const chest = useChestOpening();

  return (
    <div>
      <output data-testid="chest-state">
        {[
          chest.isOpen ? "open" : "closed",
          chest.currentVariant ?? "none",
          chest.isOpenAllMode ? "all" : "one",
          chest.isBulkMode ? "bulk" : "single",
          chest.currentBoxIndex,
          chest.totalBoxesToOpen,
        ].join("|")}
      </output>
      <button type="button" onClick={() => chest.openChest(variant)}>
        open one
      </button>
      <button type="button" onClick={() => chest.openAllChests(variant)}>
        open all
      </button>
      <button type="button" onClick={chest.closeChest}>
        close from context
      </button>
    </div>
  );
}

function renderChestProvider({
  userIndex = 7,
  variant = "silver",
  unopenedBoxes = {},
  huntedBoxes = [],
  huntedBoosters = [],
}: {
  userIndex?: number;
  variant?: BoxVariant;
  unopenedBoxes?: Record<string, Record<string, number>>;
  huntedBoxes?: HuntedBoxEntry[];
  huntedBoosters?: HuntedBooster[];
}) {
  const queryClient = createTestQueryClient();
  const invalidateQueries = vi.spyOn(queryClient, "invalidateQueries");

  render(
    <QueryClientProvider client={queryClient}>
      <ChestOpeningProvider
        userIndex={userIndex}
        unopenedBoxes={unopenedBoxes}
        huntedBoxes={huntedBoxes}
        huntedBoosters={huntedBoosters}
      >
        <ChestControls variant={variant} />
      </ChestOpeningProvider>
    </QueryClientProvider>,
  );

  return {
    invalidateQueries,
    queryClient,
  };
}

describe("ChestOpeningProvider", () => {
  beforeEach(() => {
    chestMocks.openBoxes.mockResolvedValue({ success: true });
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

  it("opens the highest-ranked hunted slot for a chest and uses the matching multiplier", () => {
    renderChestProvider({
      huntedBoxes: [
        { chest: "silver", epoch: 41, loot: "bronze_shell", opened: false },
        { chest: "silver", epoch: 42, loot: "pearl_dango", opened: false },
      ],
      huntedBoosters: [{ epoch: 42, loot: "pearl_dango", multiplier: "9.99", rank: 3 }],
    });

    fireEvent.click(screen.getByRole("button", { name: "open one" }));

    expect(screen.getByTestId("chest-state")).toHaveTextContent("open|silver|one|single|0|1");
    expect(screen.getByTestId("chest-overlay")).toHaveAttribute("data-variant", "silver");
    expect(screen.getByTestId("chest-overlay")).toHaveAttribute(
      "data-slot",
      "hunted:pearl_dango:42:9.99",
    );

    fireEvent.click(screen.getByRole("button", { name: "close" }));

    expect(screen.getByTestId("chest-state")).toHaveTextContent("closed|none|one|single|0|1");
    expect(chestMocks.openBoxes).not.toHaveBeenCalled();
  });

  it("submits all spun slots and invalidates box and booster resources", async () => {
    const { invalidateQueries } = renderChestProvider({
      unopenedBoxes: {
        silver: {
          common: 11,
        },
      },
      huntedBoxes: [{ chest: "silver", epoch: 7, loot: "golden_shell", opened: false }],
    });

    fireEvent.click(screen.getByRole("button", { name: "open all" }));

    expect(screen.getByTestId("chest-state")).toHaveTextContent("open|silver|all|bulk|0|12");
    expect(screen.getByTestId("chest-overlay")).toHaveAttribute(
      "data-slot",
      "hunted:golden_shell:7:2",
    );

    fireEvent.click(screen.getByRole("button", { name: "spin" }));
    fireEvent.click(screen.getByRole("button", { name: "close" }));

    await waitFor(() => {
      expect(chestMocks.openBoxes).toHaveBeenCalledWith(
        "https://points.example",
        7,
        {
          silver: {
            common: 11,
          },
        },
        [{ epoch: 7, loot: "golden_shell" }],
      );
    });
    expect(invalidateQueries).toHaveBeenCalledWith({ queryKey: ["boxes", 7] });
    expect(invalidateQueries).toHaveBeenCalledWith({ queryKey: ["boosters", 7] });
  });

  it("keeps hunted rewards ahead of fungible boxes when opening all chests", async () => {
    renderChestProvider({
      unopenedBoxes: {
        silver: {
          common: 2,
        },
      },
      huntedBoxes: [
        { chest: "silver", epoch: 11, loot: "bronze_shell", opened: false },
        { chest: "silver", epoch: 12, loot: "pearl_dango", opened: false },
        { chest: "gold", epoch: 13, loot: "golden_shell", opened: false },
      ],
      huntedBoosters: [{ epoch: 12, loot: "pearl_dango", multiplier: "3.5", rank: 3 }],
    });

    fireEvent.click(screen.getByRole("button", { name: "open all" }));

    expect(screen.getByTestId("chest-state")).toHaveTextContent("open|silver|all|single|0|4");
    expect(screen.getByTestId("chest-overlay")).toHaveAttribute(
      "data-slot",
      "hunted:pearl_dango:12:3.5",
    );

    fireEvent.click(screen.getByRole("button", { name: "next" }));

    expect(screen.getByTestId("chest-state")).toHaveTextContent("open|silver|all|single|1|4");
    expect(screen.getByTestId("chest-overlay")).toHaveAttribute(
      "data-slot",
      "hunted:bronze_shell:11:1.25",
    );

    fireEvent.click(screen.getByRole("button", { name: "next" }));

    expect(screen.getByTestId("chest-state")).toHaveTextContent("open|silver|all|single|2|4");
    expect(screen.getByTestId("chest-overlay")).toHaveAttribute("data-slot", "fungible:common");

    fireEvent.click(screen.getByRole("button", { name: "spin" }));
    fireEvent.click(screen.getByRole("button", { name: "close" }));

    await waitFor(() => {
      expect(chestMocks.openBoxes).toHaveBeenCalledWith(
        "https://points.example",
        7,
        {
          silver: {
            common: 2,
          },
        },
        [
          { epoch: 12, loot: "pearl_dango" },
          { epoch: 11, loot: "bronze_shell" },
        ],
      );
    });
  });

  it("submits hunted-only open-all rewards with an empty fungible boxes payload", async () => {
    const { invalidateQueries } = renderChestProvider({
      variant: "gold",
      huntedBoxes: [
        { chest: "gold", epoch: 22, loot: "silver_shell", opened: false },
        { chest: "gold", epoch: 23, loot: "golden_shell", opened: false },
        { chest: "silver", epoch: 24, loot: "pearl_dango", opened: false },
      ],
      huntedBoosters: [{ epoch: 23, loot: "golden_shell", multiplier: "2.75", rank: 2 }],
    });

    fireEvent.click(screen.getByRole("button", { name: "open all" }));

    expect(screen.getByTestId("chest-state")).toHaveTextContent("open|gold|all|single|0|2");
    expect(screen.getByTestId("chest-overlay")).toHaveAttribute(
      "data-slot",
      "hunted:golden_shell:23:2.75",
    );

    fireEvent.click(screen.getByRole("button", { name: "spin" }));
    fireEvent.click(screen.getByRole("button", { name: "close" }));

    await waitFor(() => {
      expect(chestMocks.openBoxes).toHaveBeenCalledWith("https://points.example", 7, {}, [
        { epoch: 23, loot: "golden_shell" },
        { epoch: 22, loot: "silver_shell" },
      ]);
    });
    expect(invalidateQueries).toHaveBeenCalledWith({ queryKey: ["boxes", 7] });
    expect(invalidateQueries).toHaveBeenCalledWith({ queryKey: ["boosters", 7] });
  });

  it("submits one spun fungible chest with the selected user and variant payload", async () => {
    const { invalidateQueries } = renderChestProvider({
      unopenedBoxes: {
        silver: {
          rare: 1,
        },
      },
    });

    fireEvent.click(screen.getByRole("button", { name: "open one" }));

    expect(screen.getByTestId("chest-state")).toHaveTextContent("open|silver|one|single|0|1");
    expect(screen.getByTestId("chest-overlay")).toHaveAttribute("data-slot", "fungible:rare");

    fireEvent.click(screen.getByRole("button", { name: "spin" }));
    fireEvent.click(screen.getByRole("button", { name: "close" }));

    await waitFor(() => {
      expect(chestMocks.openBoxes).toHaveBeenCalledWith(
        "https://points.example",
        7,
        {
          silver: {
            rare: 1,
          },
        },
        [],
      );
    });
    expect(invalidateQueries).toHaveBeenCalledWith({ queryKey: ["boxes", 7] });
    expect(invalidateQueries).toHaveBeenCalledWith({ queryKey: ["boosters", 7] });
  });

  it("ignores explicit zero backend fungible counts while submitting positive rewards", async () => {
    renderChestProvider({
      unopenedBoxes: {
        silver: {
          common: 0,
          rare: 1,
        },
      },
    });

    fireEvent.click(screen.getByRole("button", { name: "open all" }));

    expect(screen.getByTestId("chest-state")).toHaveTextContent("open|silver|all|single|0|1");
    expect(screen.getByTestId("chest-overlay")).toHaveAttribute("data-slot", "fungible:rare");

    fireEvent.click(screen.getByRole("button", { name: "spin" }));
    fireEvent.click(screen.getByRole("button", { name: "close" }));

    await waitFor(() => {
      expect(chestMocks.openBoxes).toHaveBeenCalledWith(
        "https://points.example",
        7,
        {
          silver: {
            rare: 1,
          },
        },
        [],
      );
    });
  });

  it("does not open or submit when backend reports only zero fungible counts", () => {
    renderChestProvider({
      unopenedBoxes: {
        silver: {
          common: 0,
          rare: 0,
        },
      },
    });

    fireEvent.click(screen.getByRole("button", { name: "open one" }));

    expect(screen.getByTestId("chest-state")).toHaveTextContent("closed|none|one|single|0|1");
    expect(screen.queryByTestId("chest-overlay")).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "open all" }));
    fireEvent.click(screen.getByRole("button", { name: "close from context" }));

    expect(screen.getByTestId("chest-state")).toHaveTextContent("closed|none|one|single|0|1");
    expect(screen.queryByTestId("chest-overlay")).not.toBeInTheDocument();
    expect(chestMocks.openBoxes).not.toHaveBeenCalled();
  });

  it("aggregates multi-rarity fungible open-all results into the backend mutation payload", async () => {
    renderChestProvider({
      unopenedBoxes: {
        silver: {
          common: 2,
          rare: 1,
        },
      },
    });

    fireEvent.click(screen.getByRole("button", { name: "open all" }));

    expect(screen.getByTestId("chest-state")).toHaveTextContent("open|silver|all|single|0|3");
    expect(screen.getByTestId("chest-overlay").getAttribute("data-slot")).toMatch(
      /^fungible:(common|rare)$/,
    );

    fireEvent.click(screen.getByRole("button", { name: "spin" }));
    fireEvent.click(screen.getByRole("button", { name: "close" }));

    await waitFor(() => {
      expect(chestMocks.openBoxes).toHaveBeenCalledWith(
        "https://points.example",
        7,
        {
          silver: {
            common: 2,
            rare: 1,
          },
        },
        [],
      );
    });
  });

  it("does not invalidate reward resources when backend box opening fails", async () => {
    chestMocks.openBoxes.mockRejectedValue(new Error("open failed"));
    const { invalidateQueries } = renderChestProvider({
      unopenedBoxes: {
        silver: {
          rare: 1,
        },
      },
    });

    fireEvent.click(screen.getByRole("button", { name: "open one" }));
    fireEvent.click(screen.getByRole("button", { name: "spin" }));
    fireEvent.click(screen.getByRole("button", { name: "close" }));

    await waitFor(() => {
      expect(chestMocks.openBoxes).toHaveBeenCalledWith(
        "https://points.example",
        7,
        {
          silver: {
            rare: 1,
          },
        },
        [],
      );
    });

    expect(screen.getByTestId("chest-state")).toHaveTextContent("closed|none|one|single|0|1");
    expect(invalidateQueries).not.toHaveBeenCalled();
  });

  it("advances through open-all slots before closing the sequence", () => {
    renderChestProvider({
      unopenedBoxes: {
        silver: {
          common: 2,
        },
      },
    });

    fireEvent.click(screen.getByRole("button", { name: "open all" }));

    expect(screen.getByTestId("chest-state")).toHaveTextContent("open|silver|all|single|0|2");
    expect(screen.getByTestId("chest-overlay")).toHaveAttribute("data-slot", "fungible:common");

    fireEvent.click(screen.getByRole("button", { name: "next" }));

    expect(screen.getByTestId("chest-state")).toHaveTextContent("open|silver|all|single|1|2");
    expect(screen.getByTestId("chest-overlay")).toHaveAttribute("data-slot", "fungible:common");

    fireEvent.click(screen.getByRole("button", { name: "next" }));

    expect(screen.getByTestId("chest-state")).toHaveTextContent("closed|none|one|single|0|1");
  });
});
