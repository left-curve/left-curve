import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import { LootResult } from "../src/components/points/rewards/LootResult";
import { LootSummary } from "../src/components/points/rewards/LootSummary";
import { NFTsSection } from "../src/components/points/rewards/NFTsSection";
import { huntedDisplay, nftDisplay } from "../src/components/points/rewards/loot";

const rewardNftMocks = vi.hoisted(() => ({
  open: vi.fn(),
  useAccount: vi.fn(),
}));

vi.mock("@left-curve/store", () => ({
  useAccount: rewardNftMocks.useAccount,
}));

function setAccount({ isConnected = true }: { isConnected?: boolean } = {}) {
  rewardNftMocks.useAccount.mockReturnValue({
    isConnected,
  });
}

function expectTweet(text: string) {
  expect(rewardNftMocks.open).toHaveBeenCalledWith(
    `https://twitter.com/intent/tweet?text=${encodeURIComponent(text)}`,
    "_blank",
  );
}

const nftRarityLabels = [
  () => m["points.rewards.nfts.rarities.common"](),
  () => m["points.rewards.nfts.rarities.uncommon"](),
  () => m["points.rewards.nfts.rarities.rare"](),
  () => m["points.rewards.nfts.rarities.epic"](),
  () => m["points.rewards.nfts.rarities.legendary"](),
  () => m["points.rewards.nfts.rarities.mythic"](),
];

describe("reward NFT and loot UI", () => {
  beforeEach(() => {
    setAccount();
    Object.defineProperty(window, "open", {
      configurable: true,
      value: rewardNftMocks.open,
    });
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("renders NFT quantities for connected users from backend-fed reward items", () => {
    render(
      <NFTsSection
        nfts={[
          {
            frameSrc: "/custom/frame-common.png",
            imageSrc: "/custom/common.png",
            quantity: 3,
            rarity: "common",
          },
          {
            frameSrc: "/custom/frame-rare.png",
            imageSrc: "/custom/rare.png",
            quantity: 0,
            rarity: "rare",
          },
        ]}
      />,
    );

    expect(screen.getByText(m["points.rewards.nfts.title"]())).toBeInTheDocument();
    expect(
      screen.getByAltText(`${m["points.rewards.nfts.rarities.common"]()} NFT`),
    ).toHaveAttribute("src", "/custom/common.png");
    expect(screen.getByAltText(`${m["points.rewards.nfts.rarities.rare"]()} NFT`)).toHaveAttribute(
      "src",
      "/custom/rare.png",
    );
    expect(screen.getByText("x3")).toBeInTheDocument();
    expect(screen.getByText("x0")).toBeInTheDocument();
  });

  it("honors an explicit empty backend NFT collection instead of rendering default rewards", () => {
    render(<NFTsSection nfts={[]} />);

    expect(screen.getByText(m["points.rewards.nfts.title"]())).toBeInTheDocument();
    for (const label of nftRarityLabels) {
      expect(screen.queryByAltText(`${label()} NFT`)).not.toBeInTheDocument();
    }
    expect(screen.queryAllByText(/^x\d+$/)).toHaveLength(0);
  });

  it("locks NFT quantities for disconnected users while keeping reward art visible", () => {
    setAccount({
      isConnected: false,
    });

    render(
      <NFTsSection
        nfts={[
          {
            frameSrc: "/custom/frame-epic.png",
            imageSrc: "/custom/epic.png",
            quantity: 4,
            rarity: "epic",
          },
        ]}
      />,
    );

    expect(
      screen.getByAltText(`${m["points.rewards.nfts.rarities.epic"]()} NFT`),
    ).toBeInTheDocument();
    expect(screen.queryByText("x4")).not.toBeInTheDocument();
  });

  it("uses the next action and booster share copy while opening multiple loot results", () => {
    const onContinue = vi.fn();
    const onNext = vi.fn();

    render(
      <LootResult
        display={huntedDisplay("golden_shell", "2.5")}
        isOpenAllMode
        currentBoxIndex={0}
        totalBoxesToOpen={2}
        onContinue={onContinue}
        onNext={onNext}
      />,
    );

    expect(screen.getByText(m["points.chestOpening.boosterTitle"]())).toBeInTheDocument();
    expect(screen.getByAltText("2.5x Boost")).toBeInTheDocument();
    expect(screen.getByText("1/2")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: m["points.chestOpening.next"]() }));

    expect(onNext).toHaveBeenCalledOnce();
    expect(onContinue).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: m["points.chestOpening.shareToX"]() }));

    expectTweet(m["points.chestOpening.shareBoostText"]({ multiplier: "2.5" }));
  });

  it("finishes and shares a single NFT result", () => {
    const onContinue = vi.fn();

    render(<LootResult display={nftDisplay("epic")} onContinue={onContinue} />);

    expect(screen.getByText(m["points.chestOpening.nftCardTitle"]())).toBeInTheDocument();
    expect(screen.getByAltText("Epic")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: m["points.chestOpening.done"]() }));

    expect(onContinue).toHaveBeenCalledOnce();

    fireEvent.click(screen.getByRole("button", { name: m["points.chestOpening.shareToX"]() }));

    expectTweet(
      m["points.chestOpening.shareNftText"]({
        article: "an",
        label: "Epic",
      }),
    );
  });

  it("summarizes bulk loot counts and shares the total opened chest count", () => {
    const onClose = vi.fn();

    render(
      <LootSummary
        buckets={[
          {
            count: 2,
            display: nftDisplay("rare"),
          },
          {
            count: 1,
            display: huntedDisplay("pearl_dango", "3"),
          },
        ]}
        onClose={onClose}
      />,
    );

    expect(screen.getByText(m["points.chestOpening.summaryTitle"]())).toBeInTheDocument();
    expect(screen.getByAltText("Rare")).toBeInTheDocument();
    expect(screen.getByAltText("3x Boost")).toBeInTheDocument();
    expect(screen.getByText("x2")).toBeInTheDocument();
    expect(screen.getByText("x1")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: m["points.chestOpening.shareToX"]() }));

    expectTweet(m["points.chestOpening.shareBulkText"]({ count: 3 }));

    fireEvent.click(screen.getByRole("button", { name: m["points.chestOpening.done"]() }));

    expect(onClose).toHaveBeenCalledOnce();

    fireEvent.click(screen.getByRole("button", { name: m["points.chestOpening.closeLabel"]() }));

    expect(onClose).toHaveBeenCalledTimes(2);
  });

  it("keeps zero-count loot summary buckets visible without inflating share totals", () => {
    const onClose = vi.fn();

    render(
      <LootSummary
        buckets={[
          {
            count: 0,
            display: nftDisplay("common"),
          },
          {
            count: 2,
            display: huntedDisplay("silver_shell", "1.5"),
          },
        ]}
        onClose={onClose}
      />,
    );

    expect(screen.getByAltText("Common")).toBeInTheDocument();
    expect(screen.getByAltText("1.5x Boost")).toBeInTheDocument();
    expect(screen.getByText("x0")).toBeInTheDocument();
    expect(screen.getByText("x2")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: m["points.chestOpening.shareToX"]() }));

    expectTweet(m["points.chestOpening.shareBulkText"]({ count: 2 }));
  });

  it("handles an explicitly empty bulk loot summary without inventing reward entries", () => {
    const onClose = vi.fn();

    render(<LootSummary buckets={[]} onClose={onClose} />);

    expect(screen.getByText(m["points.chestOpening.summaryTitle"]())).toBeInTheDocument();
    expect(screen.queryAllByText(/^x\d+$/)).toHaveLength(0);
    expect(screen.queryByRole("img")).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: m["points.chestOpening.shareToX"]() }));

    expectTweet(m["points.chestOpening.shareBulkText"]({ count: 0 }));

    fireEvent.click(screen.getByRole("button", { name: m["points.chestOpening.done"]() }));
    fireEvent.click(screen.getByRole("button", { name: m["points.chestOpening.closeLabel"]() }));

    expect(onClose).toHaveBeenCalledTimes(2);
  });
});
