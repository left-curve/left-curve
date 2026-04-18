import { createContext } from "@left-curve/applets-kit";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import type React from "react";
import { type PropsWithChildren, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";

import { openBoxes } from "@left-curve/store";
import { ChestOpeningOverlay } from "./ChestOpeningOverlay";

type NFTRarity = "common" | "uncommon" | "rare" | "epic" | "legendary" | "mythic";
export type LootCount = Record<NFTRarity, number>;

const BULK_THRESHOLD = 10;

type BoxVariant = "bronze" | "silver" | "gold" | "crystal";

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

const NFT_FRAMES = [
  "/images/points/nft/frame-common.png",
  "/images/points/nft/frame-uncommon.png",
  "/images/points/nft/frame-rare.png",
  "/images/points/nft/frame-epic.png",
  "/images/points/nft/frame-legendary.png",
  "/images/points/nft/frame-mythic.png",
];

const prefetchedImages: HTMLImageElement[] = [];

const prefetchImages = () => {
  if (prefetchedImages.length > 0) return;

  const images = [
    ...Object.values(CHEST_ASSETS),
    FLASH_IMAGE,
    FLASH_IMAGE2,
    ...NFT_FRAMES,
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

function generateLootSequence(
  remaining: Record<string, number>,
  count: number,
): string[] {
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

function aggregateLootSequence(
  variant: string,
  sequence: string[],
): Record<string, Record<string, number>> {
  const counts: Record<string, number> = {};
  for (const loot of sequence) {
    counts[loot] = (counts[loot] ?? 0) + 1;
  }
  return { [variant]: counts };
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
  lootCounts: LootCount;
};

const [ChestOpeningContextProvider, useChestOpeningContext] =
  createContext<ChestOpeningContextValue>({
    name: "ChestOpeningContext",
  });

type ChestOpeningProviderProps = PropsWithChildren<{
  userIndex?: number;
  unopenedBoxes?: Record<string, Record<string, number>>;
}>;

export const ChestOpeningProvider: React.FC<ChestOpeningProviderProps> = ({
  children,
  userIndex,
  unopenedBoxes = {},
}) => {
  const queryClient = useQueryClient();
  const pointsUrl = window.dango.urls.pointsUrl;
  const [currentVariant, setCurrentVariant] = useState<BoxVariant | null>(null);
  const [selectedLoot, setSelectedLoot] = useState<string | null>(null);
  const [currentFrame, setCurrentFrame] = useState(0);
  const [animationComplete, setAnimationComplete] = useState(false);
  const animationRef = useRef<number | null>(null);
  const lastFrameTimeRef = useRef<number>(0);

  // Open All mode state
  const [isOpenAllMode, setIsOpenAllMode] = useState(false);
  const [currentBoxIndex, setCurrentBoxIndex] = useState(0);
  const [totalBoxesToOpen, setTotalBoxesToOpen] = useState(1);
  const [lootSequence, setLootSequence] = useState<string[]>([]);
  const [pendingOpenPayload, setPendingOpenPayload] = useState<
    Record<string, Record<string, number>>
  >({});
  const [hasSpun, setHasSpun] = useState(false);

  const isOpen = currentVariant !== null;
  const animationFrames = currentVariant ? (ANIMATION_FRAMES[currentVariant] ?? null) : null;
  const isBulkMode = isOpenAllMode && totalBoxesToOpen > BULK_THRESHOLD;

  const lootCounts = useMemo<LootCount>(() => {
    const counts: LootCount = {
      common: 0,
      uncommon: 0,
      rare: 0,
      epic: 0,
      legendary: 0,
      mythic: 0,
    };
    if (!isOpenAllMode || !currentVariant) return counts;
    const variantPayload = pendingOpenPayload[currentVariant];
    if (!variantPayload) return counts;
    for (const [loot, count] of Object.entries(variantPayload)) {
      if (loot in counts) {
        counts[loot as NFTRarity] = count;
      }
    }
    return counts;
  }, [isOpenAllMode, currentVariant, pendingOpenPayload]);

  const openBoxesMutation = useMutation({
    mutationFn: ({
      boxUserIndex,
      boxes,
    }: { boxUserIndex: number; boxes: Record<string, Record<string, number>> }) =>
      openBoxes(pointsUrl, boxUserIndex, boxes),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["boxes", userIndex] });
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
      const remaining = unopenedBoxes[variant];
      if (!remaining) return;

      const loot = pickRandomLoot(remaining);
      if (!loot) return;

      setSelectedLoot(loot);
      setPendingOpenPayload({ [variant]: { [loot]: 1 } });
      setCurrentFrame(0);
      setAnimationComplete(false);
      lastFrameTimeRef.current = 0;
      setCurrentVariant(variant);
      setIsOpenAllMode(false);
      setCurrentBoxIndex(0);
      setTotalBoxesToOpen(1);
      setLootSequence([]);
      setHasSpun(false);
    },
    [unopenedBoxes],
  );

  const openAllChests = useCallback(
    (variant: BoxVariant) => {
      const remaining = unopenedBoxes[variant];
      if (!remaining) return;

      const total = Object.values(remaining).reduce((sum, n) => sum + n, 0);
      if (total === 0) return;

      const sequence = generateLootSequence(remaining, total);
      const payload = aggregateLootSequence(variant, sequence);

      setLootSequence(sequence);
      setPendingOpenPayload(payload);
      setTotalBoxesToOpen(total);
      setCurrentBoxIndex(0);
      setIsOpenAllMode(true);
      setSelectedLoot(sequence[0]);
      setCurrentFrame(0);
      setAnimationComplete(false);
      lastFrameTimeRef.current = 0;
      setCurrentVariant(variant);
      setHasSpun(false);
    },
    [unopenedBoxes],
  );

  const openNextBox = useCallback(() => {
    if (!isOpenAllMode || !currentVariant) return;

    const nextIndex = currentBoxIndex + 1;
    if (nextIndex >= lootSequence.length) {
      closeChestInternal();
      return;
    }

    setCurrentBoxIndex(nextIndex);
    setSelectedLoot(lootSequence[nextIndex]);
    setCurrentFrame(0);
    setAnimationComplete(false);
    lastFrameTimeRef.current = 0;
    if (animationRef.current) {
      cancelAnimationFrame(animationRef.current);
    }
  }, [isOpenAllMode, currentVariant, currentBoxIndex, lootSequence]);

  const closeChestInternal = useCallback(() => {
    setCurrentVariant(null);
    setSelectedLoot(null);
    setCurrentFrame(0);
    setAnimationComplete(false);
    lastFrameTimeRef.current = 0;
    setIsOpenAllMode(false);
    setCurrentBoxIndex(0);
    setTotalBoxesToOpen(1);
    setLootSequence([]);
    setPendingOpenPayload({});
    setHasSpun(false);
    if (animationRef.current) {
      cancelAnimationFrame(animationRef.current);
    }
  }, []);

  const closeChest = useCallback(() => {
    if (hasSpun && userIndex && Object.keys(pendingOpenPayload).length > 0) {
      openBoxesMutation.mutate({ boxUserIndex: userIndex, boxes: pendingOpenPayload });
    }
    closeChestInternal();
  }, [hasSpun, userIndex, pendingOpenPayload, openBoxesMutation, closeChestInternal]);

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
        lootCounts,
      }}
    >
      {children}
      {isOpen &&
        createPortal(
          <ChestOpeningOverlay
            variant={currentVariant!}
            loot={selectedLoot}
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
            lootCounts={lootCounts}
          />,
          document.body,
        )}
    </ChestOpeningContextProvider>
  );
};

export const useChestOpening = useChestOpeningContext;
