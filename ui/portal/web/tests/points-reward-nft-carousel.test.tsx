import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { NFTCarousel } from "../src/components/points/rewards/NFTCarousel";

vi.mock("framer-motion", async () => {
  const React = await import("react");

  type MotionValue = {
    get: () => number;
    set: (value: number) => void;
  };

  return {
    motion: {
      div: ({
        animate: _animate,
        children,
        exit: _exit,
        initial: _initial,
        style: _style,
        transition: _transition,
        ...props
      }: React.HTMLAttributes<HTMLDivElement> & {
        animate?: unknown;
        exit?: unknown;
        initial?: unknown;
        style?: unknown;
        transition?: unknown;
      }) => <div {...props}>{children}</div>,
    },
    useMotionValue: (initial: number): MotionValue => {
      let current = initial;
      return {
        get: () => current,
        set: (value: number) => {
          current = value;
        },
      };
    },
    useTransform: (motionValue: MotionValue, transform: (value: number) => number) =>
      transform(motionValue.get()),
  };
});

const commonNft = {
  frameSrc: "/frames/common.png",
  id: "common",
  label: "Common",
  rarity: "common",
};

const rareNft = {
  frameSrc: "/frames/rare.png",
  id: "rare",
  label: "Rare",
  rarity: "rare",
};

const staleBackendNft = {
  frameSrc: "/frames/mythic.png",
  id: "mythic",
  label: "Mythic",
  rarity: "mythic",
};

describe("NFTCarousel", () => {
  let frameTime = 0;

  beforeEach(() => {
    frameTime = 0;
    Object.defineProperty(window, "innerWidth", {
      configurable: true,
      value: 1280,
    });
    vi.spyOn(performance, "now").mockImplementation(() => frameTime);
    vi.stubGlobal(
      "requestAnimationFrame",
      vi.fn((callback: FrameRequestCallback) => {
        frameTime += 1000;
        callback(frameTime);
        return frameTime;
      }),
    );
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
  });

  it("spins to the backend-selected target NFT and reports completion once", async () => {
    const onSpinComplete = vi.fn();

    const { rerender } = render(
      <NFTCarousel
        nfts={[commonNft, rareNft]}
        isSpinning
        targetNFT={rareNft}
        onSpinComplete={onSpinComplete}
      />,
    );

    expect(screen.getAllByAltText("Common")).toHaveLength(8);
    expect(screen.getAllByAltText("Rare")).toHaveLength(8);

    await waitFor(() => {
      expect(onSpinComplete).toHaveBeenCalledOnce();
    });

    rerender(
      <NFTCarousel
        nfts={[commonNft, rareNft]}
        isSpinning
        targetNFT={rareNft}
        onSpinComplete={onSpinComplete}
      />,
    );

    expect(onSpinComplete).toHaveBeenCalledOnce();
  });

  it("waits for a delayed backend-selected target before completing the spin", async () => {
    const onSpinComplete = vi.fn();

    const { rerender } = render(
      <NFTCarousel
        nfts={[commonNft, rareNft]}
        isSpinning
        targetNFT={null}
        onSpinComplete={onSpinComplete}
      />,
    );

    expect(screen.getAllByAltText("Common")).toHaveLength(8);
    expect(screen.getAllByAltText("Rare")).toHaveLength(8);
    expect(requestAnimationFrame).not.toHaveBeenCalled();
    expect(onSpinComplete).not.toHaveBeenCalled();

    rerender(
      <NFTCarousel
        nfts={[commonNft, rareNft]}
        isSpinning
        targetNFT={rareNft}
        onSpinComplete={onSpinComplete}
      />,
    );

    await waitFor(() => {
      expect(onSpinComplete).toHaveBeenCalledOnce();
    });
  });

  it("ignores backend-selected target NFTs that are not in the carousel item set", () => {
    const onSpinComplete = vi.fn();

    render(
      <NFTCarousel
        nfts={[commonNft, rareNft]}
        isSpinning
        targetNFT={staleBackendNft}
        onSpinComplete={onSpinComplete}
      />,
    );

    expect(screen.getAllByAltText("Common")).toHaveLength(8);
    expect(screen.getAllByAltText("Rare")).toHaveLength(8);
    expect(screen.queryByAltText("Mythic")).not.toBeInTheDocument();
    expect(requestAnimationFrame).not.toHaveBeenCalled();
    expect(onSpinComplete).not.toHaveBeenCalled();
  });
});
