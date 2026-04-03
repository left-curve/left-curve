import { createContext } from "@left-curve/applets-kit";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import type React from "react";
import { type PropsWithChildren, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";

import { type BoxReward, openBox } from "@left-curve/store";
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
  unopenedBoxes?: Record<string, BoxReward[]>;
}>;

export const ChestOpeningProvider: React.FC<ChestOpeningProviderProps> = ({
  children,
  userIndex,
  unopenedBoxes = {},
}) => {
  const queryClient = useQueryClient();
  const pointsUrl = window.dango.urls.pointsUrl;
  const [currentVariant, setCurrentVariant] = useState<BoxVariant | null>(null);
  const [currentBox, setCurrentBox] = useState<BoxReward | null>(null);
  const [currentFrame, setCurrentFrame] = useState(0);
  const [animationComplete, setAnimationComplete] = useState(false);
  const animationRef = useRef<number | null>(null);
  const lastFrameTimeRef = useRef<number>(0);

  // Open All mode state
  const [isOpenAllMode, setIsOpenAllMode] = useState(false);
  const [currentBoxIndex, setCurrentBoxIndex] = useState(0);
  const [totalBoxesToOpen, setTotalBoxesToOpen] = useState(1);
  const [boxesToOpen, setBoxesToOpen] = useState<BoxReward[]>([]);

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
    if (!isOpenAllMode || boxesToOpen.length === 0) return counts;
    for (const box of boxesToOpen) {
      const rarity = box.loot?.toLowerCase() as NFTRarity | undefined;
      if (rarity && rarity in counts) {
        counts[rarity]++;
      }
    }
    return counts;
  }, [isOpenAllMode, boxesToOpen]);

  const openBoxMutation = useMutation({
    mutationFn: ({ boxUserIndex, boxId }: { boxUserIndex: number; boxId: string }) =>
      openBox(pointsUrl, boxUserIndex, boxId),
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
      const boxes = unopenedBoxes[variant];
      if (!boxes || boxes.length === 0) return;

      const box = boxes[0];
      setCurrentBox(box);
      setCurrentFrame(0);
      setAnimationComplete(false);
      lastFrameTimeRef.current = 0;
      setCurrentVariant(variant);
      setIsOpenAllMode(false);
      setCurrentBoxIndex(0);
      setTotalBoxesToOpen(1);
      setBoxesToOpen([]);
    },
    [unopenedBoxes],
  );

  const openAllChests = useCallback(
    (variant: BoxVariant) => {
      const boxes = unopenedBoxes[variant];
      if (!boxes || boxes.length === 0) return;

      setBoxesToOpen([...boxes]);
      setTotalBoxesToOpen(boxes.length);
      setCurrentBoxIndex(0);
      setIsOpenAllMode(true);

      const box = boxes[0];
      setCurrentBox(box);
      setCurrentFrame(0);
      setAnimationComplete(false);
      lastFrameTimeRef.current = 0;
      setCurrentVariant(variant);
    },
    [unopenedBoxes],
  );

  const openNextBox = useCallback(() => {
    if (!isOpenAllMode || !currentVariant) return;

    // Mark current box as opened
    if (currentBox && userIndex) {
      openBoxMutation.mutate({ boxUserIndex: userIndex, boxId: currentBox.box_id });
    }

    const nextIndex = currentBoxIndex + 1;
    if (nextIndex >= boxesToOpen.length) {
      // No more boxes, close
      closeChestInternal();
      return;
    }

    // Open next box
    setCurrentBoxIndex(nextIndex);
    const nextBox = boxesToOpen[nextIndex];
    setCurrentBox(nextBox);
    setCurrentFrame(0);
    setAnimationComplete(false);
    lastFrameTimeRef.current = 0;
    if (animationRef.current) {
      cancelAnimationFrame(animationRef.current);
    }
  }, [isOpenAllMode, currentVariant, currentBox, userIndex, currentBoxIndex, boxesToOpen, openBoxMutation]);

  const closeChestInternal = useCallback(() => {
    setCurrentVariant(null);
    setCurrentBox(null);
    setCurrentFrame(0);
    setAnimationComplete(false);
    lastFrameTimeRef.current = 0;
    setIsOpenAllMode(false);
    setCurrentBoxIndex(0);
    setTotalBoxesToOpen(1);
    setBoxesToOpen([]);
    if (animationRef.current) {
      cancelAnimationFrame(animationRef.current);
    }
  }, []);

  const closeChest = useCallback(() => {
    if (userIndex) {
      if (isBulkMode && boxesToOpen.length > 0) {
        // In bulk mode, open all boxes at once
        for (const box of boxesToOpen) {
          openBoxMutation.mutate({ boxUserIndex: userIndex, boxId: box.box_id });
        }
      } else if (currentBox) {
        // Single box mode
        openBoxMutation.mutate({ boxUserIndex: userIndex, boxId: currentBox.box_id });
      }
    }
    closeChestInternal();
  }, [currentBox, userIndex, openBoxMutation, closeChestInternal, isBulkMode, boxesToOpen]);

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
            loot={currentBox?.loot ?? null}
            onClose={closeChest}
            currentFrame={currentFrame}
            animationFrames={animationFrames}
            onAnimationComplete={onAnimationComplete}
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
