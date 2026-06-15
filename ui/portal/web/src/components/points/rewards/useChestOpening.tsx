import { createContext } from "@left-curve/applets-kit";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import type React from "react";
import { type PropsWithChildren, useCallback, useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";

import {
  type HuntedBooster,
  type HuntedBoxEntry,
  type HuntedLoot,
  openBoxes,
} from "@left-curve/store";
import { ChestOpeningOverlay } from "./ChestOpeningOverlay";
import {
  type LootDisplay,
  type NftRarity,
  ALL_NFT_DISPLAYS,
  HUNTED_ORDER,
  huntedDisplay,
  nftDisplay,
} from "./loot";

export type { LootDisplay };
export type { LootBucket } from "./LootSummary";

const BULK_THRESHOLD = 10;

type BoxVariant = "bronze" | "silver" | "gold" | "crystal";

export type OpenableSlot =
  | { kind: "fungible"; loot: string }
  | { kind: "hunted"; loot: HuntedLoot; epoch: number; multiplier: string };

const HUNTED_RANK: Record<HuntedLoot, number> = {
  bronze_shell: 0,
  silver_shell: 1,
  golden_shell: 2,
  pearl_dango: 3,
};

const HUNTED_FALLBACK_MULTIPLIER: Record<HuntedLoot, string> = {
  bronze_shell: "1.25",
  silver_shell: "1.5",
  golden_shell: "2",
  pearl_dango: "2.5",
};

const CHEST_ASSETS: Record<BoxVariant, string> = {
  bronze: "/images/points/boxes/bronze.png",
  silver: "/images/points/boxes/silver.png",
  gold: "/images/points/boxes/gold.png",
  crystal: "/images/points/boxes/crystal.png",
};

const generateFrames = (variant: string) =>
  Array.from(
    { length: 50 },
    (_, i) =>
      `/images/points/boxes-animation/${variant}/frame_${String(i + 1).padStart(4, "0")}.webp`,
  );

const ANIMATION_FRAMES: Partial<Record<BoxVariant, string[]>> = {
  bronze: generateFrames("bronze"),
  silver: generateFrames("silver"),
  gold: generateFrames("gold"),
  crystal: generateFrames("crystal"),
};

const ANIMATION_FPS = 30;
const FRAME_DURATION = 1000 / ANIMATION_FPS;

const FLASH_IMAGE = "/images/points/flash.png";
const FLASH_IMAGE2 = "/images/points/flash2.png";

const prefetchedImages: HTMLImageElement[] = [];

const prefetchImages = () => {
  if (prefetchedImages.length > 0) return;

  const images = [
    ...Object.values(CHEST_ASSETS),
    FLASH_IMAGE,
    FLASH_IMAGE2,
    ...ALL_NFT_DISPLAYS.map((d) => d.frameSrc),
    ...HUNTED_ORDER.map((loot) => huntedDisplay(loot, "1").frameSrc),
    ...Object.values(ANIMATION_FRAMES).flat(),
  ];

  images.forEach((src) => {
    const img = new Image();
    if (src.startsWith("http")) {
      img.crossOrigin = "anonymous";
    }
    img.src = src;
    prefetchedImages.push(img);
  });
};

function pickRandomLoot(remaining: Record<string, number>): string | null {
  const entries = Object.entries(remaining).filter(([_, n]) => n > 0);
  if (entries.length === 0) return null;
  const total = entries.reduce((sum, [_, n]) => sum + n, 0);
  let roll = Math.random() * total;
  for (const [loot, count] of entries) {
    roll -= count;
    if (roll <= 0) return loot;
  }
  return entries.at(-1)![0];
}

function generateFungibleSequence(remaining: Record<string, number>, count: number): string[] {
  const pool = { ...remaining };
  const sequence: string[] = [];
  for (let i = 0; i < count; i++) {
    const loot = pickRandomLoot(pool);
    if (!loot) break;
    sequence.push(loot);
    pool[loot]--;
  }
  return sequence;
}

function multiplierFor(loot: HuntedLoot, epoch: number, huntedBoosters: HuntedBooster[]): string {
  const match = huntedBoosters.find((b) => b.loot === loot && b.epoch === epoch);
  return match ? match.multiplier.toString() : HUNTED_FALLBACK_MULTIPLIER[loot];
}

function huntedSlotsForChest(
  variant: BoxVariant,
  huntedBoxes: HuntedBoxEntry[],
  huntedBoosters: HuntedBooster[],
): OpenableSlot[] {
  return huntedBoxes
    .filter((entry) => entry.chest === variant)
    .slice()
    .sort((a, b) => HUNTED_RANK[b.loot] - HUNTED_RANK[a.loot])
    .map((entry) => ({
      kind: "hunted" as const,
      loot: entry.loot,
      epoch: entry.epoch,
      multiplier: multiplierFor(entry.loot, entry.epoch, huntedBoosters),
    }));
}

export function displayForSlot(slot: OpenableSlot): LootDisplay {
  if (slot.kind === "hunted") return huntedDisplay(slot.loot, slot.multiplier);
  return nftDisplay(slot.loot as NftRarity);
}

function splitMutationPayload(
  variant: BoxVariant,
  slots: OpenableSlot[],
): {
  boxes: Record<string, Record<string, number>>;
  hunted: Array<{ epoch: number; loot: HuntedLoot }>;
} {
  const variantCounts: Record<string, number> = {};
  const hunted: Array<{ epoch: number; loot: HuntedLoot }> = [];
  for (const slot of slots) {
    if (slot.kind === "fungible") {
      variantCounts[slot.loot] = (variantCounts[slot.loot] ?? 0) + 1;
    } else {
      hunted.push({ epoch: slot.epoch, loot: slot.loot });
    }
  }
  const boxes = Object.keys(variantCounts).length > 0 ? { [variant]: variantCounts } : {};
  return { boxes, hunted };
}

type ChestOpeningContextValue = {
  openChest: (variant: BoxVariant) => void;
  openAllChests: (variant: BoxVariant) => void;
  closeChest: () => void;
  isOpen: boolean;
  currentVariant: BoxVariant | null;
  isOpenAllMode: boolean;
  isBulkMode: boolean;
  currentBoxIndex: number;
  totalBoxesToOpen: number;
};

const [ChestOpeningContextProvider, useChestOpeningContext] =
  createContext<ChestOpeningContextValue>({
    name: "ChestOpeningContext",
  });

type ChestOpeningProviderProps = PropsWithChildren<{
  userIndex?: number;
  unopenedBoxes?: Record<string, Record<string, number>>;
  huntedBoxes?: HuntedBoxEntry[];
  huntedBoosters?: HuntedBooster[];
}>;

export const ChestOpeningProvider: React.FC<ChestOpeningProviderProps> = ({
  children,
  userIndex,
  unopenedBoxes = {},
  huntedBoxes = [],
  huntedBoosters = [],
}) => {
  const queryClient = useQueryClient();
  const pointsUrl = window.dango.urls.pointsUrl;
  const [currentVariant, setCurrentVariant] = useState<BoxVariant | null>(null);
  const [selectedSlot, setSelectedSlot] = useState<OpenableSlot | null>(null);
  const [currentFrame, setCurrentFrame] = useState(0);
  const [animationComplete, setAnimationComplete] = useState(false);
  const animationRef = useRef<number | null>(null);
  const lastFrameTimeRef = useRef<number>(0);

  const [isOpenAllMode, setIsOpenAllMode] = useState(false);
  const [currentBoxIndex, setCurrentBoxIndex] = useState(0);
  const [totalBoxesToOpen, setTotalBoxesToOpen] = useState(1);
  const [slotSequence, setSlotSequence] = useState<OpenableSlot[]>([]);
  const [hasSpun, setHasSpun] = useState(false);

  const isOpen = currentVariant !== null;
  const animationFrames = currentVariant ? (ANIMATION_FRAMES[currentVariant] ?? null) : null;
  const isBulkMode = isOpenAllMode && totalBoxesToOpen > BULK_THRESHOLD;

  const openBoxesMutation = useMutation({
    mutationFn: ({
      boxUserIndex,
      boxes,
      hunted,
    }: {
      boxUserIndex: number;
      boxes: Record<string, Record<string, number>>;
      hunted: Array<{ epoch: number; loot: HuntedLoot }>;
    }) => openBoxes(pointsUrl, boxUserIndex, boxes, hunted),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["boxes", userIndex] });
      queryClient.invalidateQueries({ queryKey: ["boosters", userIndex] });
    },
  });

  useEffect(() => {
    prefetchImages();
  }, []);

  useEffect(() => {
    if (!isOpen || !animationFrames || animationComplete) return;

    const totalFrames = animationFrames.length;

    const animate = (timestamp: number) => {
      if (!lastFrameTimeRef.current) {
        lastFrameTimeRef.current = timestamp;
      }

      const elapsed = timestamp - lastFrameTimeRef.current;

      if (elapsed >= FRAME_DURATION) {
        setCurrentFrame((prev) => {
          const nextFrame = prev + 1;
          if (nextFrame >= totalFrames) {
            setAnimationComplete(true);
            return prev;
          }
          return nextFrame;
        });
        lastFrameTimeRef.current = timestamp;
      }

      animationRef.current = requestAnimationFrame(animate);
    };

    const startDelay = setTimeout(() => {
      animationRef.current = requestAnimationFrame(animate);
    }, 350);

    return () => {
      clearTimeout(startDelay);
      if (animationRef.current) {
        cancelAnimationFrame(animationRef.current);
      }
    };
  }, [isOpen, animationFrames, animationComplete]);

  const openChest = useCallback(
    (variant: BoxVariant) => {
      const hunted = huntedSlotsForChest(variant, huntedBoxes, huntedBoosters);
      const fungibleRemaining = unopenedBoxes[variant];

      let slot: OpenableSlot | null = null;
      if (hunted.length > 0) {
        slot = hunted[0];
      } else if (fungibleRemaining) {
        const loot = pickRandomLoot(fungibleRemaining);
        if (loot) slot = { kind: "fungible", loot };
      }

      if (!slot) return;

      setSelectedSlot(slot);
      setSlotSequence([slot]);
      setCurrentFrame(0);
      setAnimationComplete(false);
      lastFrameTimeRef.current = 0;
      setCurrentVariant(variant);
      setIsOpenAllMode(false);
      setCurrentBoxIndex(0);
      setTotalBoxesToOpen(1);
      setHasSpun(false);
    },
    [unopenedBoxes, huntedBoxes, huntedBoosters],
  );

  const openAllChests = useCallback(
    (variant: BoxVariant) => {
      const hunted = huntedSlotsForChest(variant, huntedBoxes, huntedBoosters);
      const fungibleRemaining = unopenedBoxes[variant] ?? {};
      const fungibleTotal = Object.values(fungibleRemaining).reduce((sum, n) => sum + n, 0);

      if (hunted.length === 0 && fungibleTotal === 0) return;

      const fungibleSlots: OpenableSlot[] = generateFungibleSequence(
        fungibleRemaining,
        fungibleTotal,
      ).map((loot) => ({ kind: "fungible", loot }));

      const sequence = [...hunted, ...fungibleSlots];

      setSlotSequence(sequence);
      setTotalBoxesToOpen(sequence.length);
      setCurrentBoxIndex(0);
      setIsOpenAllMode(true);
      setSelectedSlot(sequence[0]);
      setCurrentFrame(0);
      setAnimationComplete(false);
      lastFrameTimeRef.current = 0;
      setCurrentVariant(variant);
      setHasSpun(false);
    },
    [unopenedBoxes, huntedBoxes, huntedBoosters],
  );

  const closeChestInternal = useCallback(() => {
    setCurrentVariant(null);
    setSelectedSlot(null);
    setCurrentFrame(0);
    setAnimationComplete(false);
    lastFrameTimeRef.current = 0;
    setIsOpenAllMode(false);
    setCurrentBoxIndex(0);
    setTotalBoxesToOpen(1);
    setSlotSequence([]);
    setHasSpun(false);
    if (animationRef.current) {
      cancelAnimationFrame(animationRef.current);
    }
  }, []);

  const openNextBox = useCallback(() => {
    if (!isOpenAllMode || !currentVariant) return;

    const nextIndex = currentBoxIndex + 1;
    if (nextIndex >= slotSequence.length) {
      closeChestInternal();
      return;
    }

    setCurrentBoxIndex(nextIndex);
    setSelectedSlot(slotSequence[nextIndex]);
    setCurrentFrame(0);
    setAnimationComplete(false);
    lastFrameTimeRef.current = 0;
    if (animationRef.current) {
      cancelAnimationFrame(animationRef.current);
    }
  }, [isOpenAllMode, currentVariant, currentBoxIndex, slotSequence, closeChestInternal]);

  const closeChest = useCallback(() => {
    if (hasSpun && userIndex && currentVariant && slotSequence.length > 0) {
      const { boxes, hunted } = splitMutationPayload(currentVariant, slotSequence);
      openBoxesMutation.mutate({ boxUserIndex: userIndex, boxes, hunted });
    }
    closeChestInternal();
  }, [hasSpun, userIndex, currentVariant, slotSequence, openBoxesMutation, closeChestInternal]);

  const markSpun = useCallback(() => {
    setHasSpun(true);
  }, []);

  const onAnimationComplete = useCallback(() => {
    setAnimationComplete(true);
  }, []);

  return (
    <ChestOpeningContextProvider
      value={{
        openChest,
        openAllChests,
        closeChest,
        isOpen,
        currentVariant,
        isOpenAllMode,
        isBulkMode,
        currentBoxIndex,
        totalBoxesToOpen,
      }}
    >
      {children}
      {isOpen &&
        createPortal(
          <ChestOpeningOverlay
            variant={currentVariant!}
            slot={selectedSlot}
            slotSequence={slotSequence}
            onClose={closeChest}
            currentFrame={currentFrame}
            animationFrames={animationFrames}
            onAnimationComplete={onAnimationComplete}
            onSpin={markSpun}
            isOpenAllMode={isOpenAllMode}
            isBulkMode={isBulkMode}
            currentBoxIndex={currentBoxIndex}
            totalBoxesToOpen={totalBoxesToOpen}
            onNext={openNextBox}
          />,
          document.body,
        )}
    </ChestOpeningContextProvider>
  );
};

export const useChestOpening = useChestOpeningContext;
