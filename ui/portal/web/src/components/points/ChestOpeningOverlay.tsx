import { Button } from "@left-curve/applets-kit";
import { AnimatePresence, motion } from "framer-motion";
import type React from "react";
import { useCallback, useEffect, useState } from "react";
import { type NFTItem, NFTCarousel } from "./NFTCarousel";
import { NFTResult } from "./NFTResult";

type BoxVariant = "bronze" | "silver" | "gold" | "crystal";

type ChestOpeningOverlayProps = {
  variant: BoxVariant;
  onClose: () => void;
  currentFrame: number;
  animationFrames: string[] | null;
  onAnimationComplete: () => void;
};

const CHEST_ASSETS: Record<BoxVariant, string> = {
  bronze: "/images/points/boxes/bronze.png",
  silver: "/images/points/boxes/silver.png",
  gold: "/images/points/boxes/gold.png",
  crystal: "/images/points/boxes/crystal.png",
};

const FLASH_IMAGE = "/images/points/flash.png";
const FLASH_IMAGE2 = "/images/points/flash2.png";

const MOCK_NFTS: NFTItem[] = [
  {
    id: "common",
    rarity: "common",
    label: "Common",
    imageSrc: "https://www.figma.com/api/mcp/asset/c3b5358b-c2b3-4bc0-a1d6-30117c53b423",
  },
  {
    id: "uncommon",
    rarity: "uncommon",
    label: "Uncommon",
    imageSrc: "https://www.figma.com/api/mcp/asset/9680ed08-69ef-471f-83a5-813846101610",
  },
  {
    id: "epic",
    rarity: "epic",
    label: "Epic",
    imageSrc: "https://www.figma.com/api/mcp/asset/fe211f83-4040-4023-ae76-da8967f68d53",
  },
  {
    id: "golden",
    rarity: "golden",
    label: "Golden",
    imageSrc: "https://www.figma.com/api/mcp/asset/21afd773-9bb9-4cd4-a30c-dc9a356d0708",
  },
  {
    id: "legendary",
    rarity: "legendary",
    label: "Legendary",
    imageSrc: "https://www.figma.com/api/mcp/asset/e21cd2e1-7549-4b53-8916-3d1713bb5699",
  },
  {
    id: "rare",
    rarity: "rare",
    label: "Rare",
    imageSrc: "https://www.figma.com/api/mcp/asset/555ebcef-e6f2-490b-9c44-b72b36dad681",
  },
];

type Phase = "chest" | "carousel" | "spinning" | "result";

export const ChestOpeningOverlay: React.FC<ChestOpeningOverlayProps> = ({
  variant,
  onClose,
  currentFrame,
  animationFrames,
  onAnimationComplete,
}) => {
  const [phase, setPhase] = useState<Phase>("chest");
  const [isShaking, setIsShaking] = useState(false);
  const [winningNFT, setWinningNFT] = useState<NFTItem | null>(null);

  const chestImage = CHEST_ASSETS[variant];
  const hasAnimation = animationFrames !== null;
  const totalFrames = animationFrames?.length ?? 1;
  const animationProgress = hasAnimation ? currentFrame / totalFrames : 0;

  useEffect(() => {
    if (phase === "chest" && hasAnimation && animationFrames) {
      if (currentFrame >= animationFrames.length - 1) {
        setPhase("carousel");
        onAnimationComplete();
      }
    }
  }, [phase, hasAnimation, currentFrame, animationFrames, onAnimationComplete]);

  useEffect(() => {
    if (phase === "chest" && !hasAnimation) {
      const shakeTimer = setTimeout(() => setIsShaking(true), 800);
      const transitionTimer = setTimeout(() => setPhase("carousel"), 1800);
      return () => {
        clearTimeout(shakeTimer);
        clearTimeout(transitionTimer);
      };
    }
  }, [phase, hasAnimation]);

  const handleSpin = useCallback(() => {
    const randomIndex = Math.floor(Math.random() * MOCK_NFTS.length);
    setWinningNFT(MOCK_NFTS[randomIndex]);
    setPhase("spinning");
  }, []);

  const handleSpinComplete = useCallback(() => {
    setPhase("result");
  }, []);

  return (
    <motion.div
      className="fixed inset-0 z-[100] flex items-center justify-center overflow-hidden"
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      transition={{ duration: 0.2 }}
    >
      <div className="absolute inset-0 bg-[#1a1714]" />

      <AnimatePresence>
        {phase === "chest" && (
          <motion.div
            key="chest-phase"
            className="absolute inset-0 flex items-center justify-center"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.25, exit: { duration: 0.4, ease: "easeOut" } }}
          >
            <motion.div
              className="absolute w-[200vmax] h-[200vmax] pointer-events-none flex items-center justify-center"
              initial={{ scale: 0.2, opacity: 0.2 }}
              animate={{
                scale: hasAnimation ? 0.2 + animationProgress * 0.6 : isShaking ? 1.1 : 0.7,
                opacity: hasAnimation ? 0.2 + animationProgress * 0.7 : isShaking ? 0.9 : 0.4,
              }}
              exit={{ scale: 1.3, opacity: 0 }}
              transition={{
                scale: { duration: hasAnimation ? 0.1 : 0.8, ease: "easeOut" },
                opacity: { duration: hasAnimation ? 0.1 : 0.5, ease: "easeOut" },
                exit: { duration: 0.3 },
              }}
            >
              <motion.img
                src={FLASH_IMAGE}
                alt="flash"
                className="w-full h-full object-contain"
                style={{ marginTop: "5%" }}
                initial={{ rotate: 0 }}
                animate={{ rotate: hasAnimation ? animationProgress * 15 : isShaking ? 20 : 5 }}
                transition={{ duration: hasAnimation ? 0.1 : 1.5, ease: "linear" }}
              />
              <div className="absolute inset-0 bg-[radial-gradient(circle,transparent_0%,transparent_30%,#1a1714_70%)]" />
            </motion.div>

            <motion.img
              src={animationFrames ? animationFrames[currentFrame] : chestImage}
              alt={`${variant} chest`}
              className="relative z-10 w-[426px] h-[426px] lg:w-[646px] lg:h-[646px] object-contain"
              initial={{ scale: 0, opacity: 0 }}
              animate={{
                scale: 1,
                opacity: 1,
                rotate: !hasAnimation && isShaking ? [0, -4, 4, -4, 4, -2, 2, 0] : 0,
              }}
              exit={{ opacity: 0 }}
              transition={{
                scale: { duration: 0.35, ease: [0.34, 1.56, 0.64, 1] },
                opacity: { duration: 0.15 },
                rotate: { duration: 0.5, ease: "easeInOut" },
                exit: { duration: 0.4, ease: "easeOut" },
              }}
            />
          </motion.div>
        )}
      </AnimatePresence>

      <AnimatePresence>
        {(phase === "carousel" || phase === "spinning" || phase === "result") && (
          <motion.div
            key="carousel-phase"
            className="absolute inset-0 z-10 flex flex-col items-center justify-center gap-6 px-4"
            initial={{ opacity: 0, scale: 0.9 }}
            animate={{ opacity: 1, scale: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.3, ease: "easeOut" }}
          >
            <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[800px] h-[800px] rounded-full bg-[#EFDAA4]/50 blur-[250px] pointer-events-none -z-10" />

            <motion.p
              className="exposure-h2-italic text-white text-center"
              initial={{ opacity: 0, y: -20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: 0.1, duration: 0.3 }}
            >
              Spin to win
            </motion.p>

            <NFTCarousel
              nfts={MOCK_NFTS}
              isSpinning={phase === "spinning"}
              targetNFT={winningNFT}
              onSpinComplete={handleSpinComplete}
            />

            <div className="flex flex-col items-center gap-3 min-h-[100px]">
              <AnimatePresence mode="wait">
                {phase === "carousel" && (
                  <motion.div
                    key="spin-button"
                    initial={{ opacity: 0, y: 20, scale: 0.9 }}
                    animate={{ opacity: 1, y: 0, scale: 1 }}
                    exit={{ opacity: 0, scale: 0.8, y: -10 }}
                    transition={{ delay: 0.2, duration: 0.3, ease: "easeOut" }}
                  >
                    <Button as={motion.button} variant="primary" size="lg" onClick={handleSpin}>
                      Spin Now!
                    </Button>
                  </motion.div>
                )}
                {(phase === "spinning" || phase === "result") && (
                  <motion.p
                    key="spinning-text"
                    className="diatype-m-medium text-white/70 h-10 flex items-center"
                    initial={{ opacity: 0, y: 20, scale: 0.9 }}
                    animate={{ opacity: phase === "result" ? 0 : [0.5, 1, 0.5], y: 0, scale: 1 }}
                    transition={{
                      opacity: {
                        duration: 1,
                        repeat: phase === "spinning" ? Number.POSITIVE_INFINITY : 0,
                        delay: 0.3,
                      },
                      y: { duration: 0.3, ease: "easeOut" },
                      scale: { duration: 0.3, ease: "easeOut" },
                    }}
                  >
                    Spinning...
                  </motion.p>
                )}
              </AnimatePresence>

              <Button as={motion.button} variant="link" onClick={onClose}>
                ← Back
              </Button>
            </div>
          </motion.div>
        )}
      </AnimatePresence>

      <AnimatePresence>
        {phase === "result" && (
          <motion.div
            key="result-flash"
            className="absolute inset-0 flex items-center justify-center pointer-events-none z-20"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.5, ease: "easeOut" }}
          >
            <motion.div
              className="absolute w-[200vmax] h-[200vmax] pointer-events-none"
              initial={{ scale: 0.6, opacity: 0 }}
              animate={{ scale: 0.6, opacity: 1 }}
              transition={{ duration: 0.6, ease: "easeOut" }}
            >
              <motion.img
                src={FLASH_IMAGE2}
                alt=""
                className="w-full h-full object-cover"
                animate={{ rotate: 360 }}
                transition={{ duration: 40, ease: "linear", repeat: Number.POSITIVE_INFINITY }}
              />
            </motion.div>
            <div className="absolute inset-0 bg-[radial-gradient(circle,transparent_20%,transparent_50%,#1a1714_90%)]" />
          </motion.div>
        )}
      </AnimatePresence>

      <AnimatePresence>
        {phase === "result" && winningNFT && (
          <motion.div
            key="result-modal"
            className="relative z-30"
            initial={{ opacity: 0, scale: 0.8, y: 20 }}
            animate={{ opacity: 1, scale: 1, y: 0 }}
            exit={{ opacity: 0, scale: 0.8 }}
            transition={{ duration: 0.3, ease: "easeOut" }}
          >
            <NFTResult nft={winningNFT} onContinue={onClose} />
          </motion.div>
        )}
      </AnimatePresence>
    </motion.div>
  );
};
