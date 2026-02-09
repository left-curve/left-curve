import { createContext } from "@left-curve/applets-kit";
import type React from "react";
import { type PropsWithChildren, useCallback, useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { ChestOpeningOverlay } from "./ChestOpeningOverlay";

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
    (_, i) => `/images/points/boxes-animation/${variant}/frame_${String(i + 1).padStart(4, "0")}.webp`,
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

const NFT_IMAGES = [
  "/images/points/nft/common.png",
  "/images/points/nft/uncommon.png",
  "/images/points/nft/epic.png",
  "/images/points/nft/mythic.png",
  "/images/points/nft/legendary.png",
  "/images/points/nft/rare.png",
];

const prefetchedImages: HTMLImageElement[] = [];

const prefetchImages = () => {
  if (prefetchedImages.length > 0) return;

  const images = [
    ...Object.values(CHEST_ASSETS),
    FLASH_IMAGE,
    FLASH_IMAGE2,
    ...NFT_IMAGES,
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
  closeChest: () => void;
  isOpen: boolean;
  currentVariant: BoxVariant | null;
};

const [ChestOpeningContextProvider, useChestOpeningContext] = createContext<ChestOpeningContextValue>({
  name: "ChestOpeningContext",
});

export const ChestOpeningProvider: React.FC<PropsWithChildren> = ({ children }) => {
  const [currentVariant, setCurrentVariant] = useState<BoxVariant | null>(null);
  const [currentFrame, setCurrentFrame] = useState(0);
  const [animationComplete, setAnimationComplete] = useState(false);
  const animationRef = useRef<number | null>(null);
  const lastFrameTimeRef = useRef<number>(0);

  const isOpen = currentVariant !== null;
  const animationFrames = currentVariant ? (ANIMATION_FRAMES[currentVariant] ?? null) : null;

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

  const openChest = useCallback((variant: BoxVariant) => {
    setCurrentFrame(0);
    setAnimationComplete(false);
    lastFrameTimeRef.current = 0;
    setCurrentVariant(variant);
  }, []);

  const closeChest = useCallback(() => {
    setCurrentVariant(null);
    setCurrentFrame(0);
    setAnimationComplete(false);
    lastFrameTimeRef.current = 0;
    if (animationRef.current) {
      cancelAnimationFrame(animationRef.current);
    }
  }, []);

  const onAnimationComplete = useCallback(() => {
    setAnimationComplete(true);
  }, []);

  return (
    <ChestOpeningContextProvider
      value={{
        openChest,
        closeChest,
        isOpen,
        currentVariant,
      }}
    >
      {children}
      {isOpen &&
        createPortal(
          <ChestOpeningOverlay
            variant={currentVariant!}
            onClose={closeChest}
            currentFrame={currentFrame}
            animationFrames={animationFrames}
            onAnimationComplete={onAnimationComplete}
          />,
          document.body,
        )}
    </ChestOpeningContextProvider>
  );
};

export const useChestOpening = useChestOpeningContext;
