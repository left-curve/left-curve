import { type MotionValue, motion, useMotionValue, useTransform } from "framer-motion";
import type React from "react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

export type NFTItem = {
  id: string;
  rarity: string;
  label: string;
  imageSrc: string;
};

type NFTCarouselProps = {
  nfts: NFTItem[];
  isSpinning: boolean;
  targetNFT: NFTItem | null;
  onSpinComplete: () => void;
};

const RARITY_COLORS: Record<string, { bg: string; text: string }> = {
  common: { bg: "bg-[#A8C686]", text: "text-[#2D3A1F]" },
  uncommon: { bg: "bg-[#E8A0A0]", text: "text-[#4A2020]" },
  epic: { bg: "bg-[#7BA3C9]", text: "text-[#1F2D3A]" },
  golden: { bg: "bg-[#D4A84B]", text: "text-[#3A2D1F]" },
  legendary: { bg: "bg-[#C9A0E8]", text: "text-[#3A1F4A]" },
  rare: { bg: "bg-[#A0A0E8]", text: "text-[#1F1F4A]" },
};

const CARD_WIDTH_MOBILE = 232;
const CARD_WIDTH_DESKTOP = 320;
const CARD_GAP_MOBILE = 4;
const CARD_GAP_DESKTOP = 16;
const SPIN_DURATION = 4;
const MIN_ROTATIONS = 4;

const MAX_ROTATION = 8;
const ROTATION_FALLOFF = 3;
const ROTATION_PIVOT_DISTANCE = 800;
const VERTICAL_OFFSET_FACTOR = 12;

type NFTCardProps = {
  nft: NFTItem;
  index: number;
  motionX: MotionValue<number>;
  itemTotalWidth: number;
};

const NFTCard: React.FC<NFTCardProps> = ({ nft, index, motionX, itemTotalWidth }) => {
  const colors = RARITY_COLORS[nft.rarity] || RARITY_COLORS.common;

  const cardPosition = index * itemTotalWidth;

  const rotation = useTransform(motionX, (x) => {
    const distanceFromCenter = cardPosition + x;
    const cardsFromCenter = distanceFromCenter / itemTotalWidth;
    const clampedDistance = Math.max(
      -ROTATION_FALLOFF,
      Math.min(ROTATION_FALLOFF, cardsFromCenter),
    );
    return (clampedDistance / ROTATION_FALLOFF) * MAX_ROTATION;
  });

  const yOffset = useTransform(motionX, (x) => {
    const distanceFromCenter = cardPosition + x;
    const cardsFromCenter = distanceFromCenter / itemTotalWidth;
    const normalizedDistance = Math.min(Math.abs(cardsFromCenter), ROTATION_FALLOFF);
    return normalizedDistance * normalizedDistance * VERTICAL_OFFSET_FACTOR;
  });

  return (
    <motion.div
      className="flex-shrink-0 relative w-[232px] lg:w-[320px]"
      style={{
        rotate: rotation,
        y: yOffset,
        transformOrigin: `center calc(100% + ${ROTATION_PIVOT_DISTANCE}px)`,
      }}
    >
      <div
        className={`absolute right-0 top-0 z-20 px-3 py-1 rounded-tr-lg rounded-bl-lg ${colors.bg} ${colors.text} diatype-xs-bold shadow-md`}
      >
        {nft.label}
      </div>

      <div className="w-full h-[284px] lg:h-[391px] rounded-2xl overflow-hidden bg-[#2a2520] border border-[#3a352f] shadow-xl">
        <div className="w-full h-full flex items-center justify-center p-6 bg-gradient-to-b from-[#2a2520] to-[#1a1714]">
          <img
            src={nft.imageSrc}
            alt={nft.label}
            className="w-full h-full object-contain [filter:drop-shadow(0px_4px_30px_rgba(227,189,102,0.4))]"
          />
        </div>
      </div>
    </motion.div>
  );
};

const useIsDesktop = () => {
  const [isDesktop, setIsDesktop] = useState(() =>
    typeof window !== "undefined" ? window.innerWidth >= 1024 : false,
  );

  useEffect(() => {
    const checkIsDesktop = () => setIsDesktop(window.innerWidth >= 1024);
    window.addEventListener("resize", checkIsDesktop);
    return () => window.removeEventListener("resize", checkIsDesktop);
  }, []);

  return isDesktop;
};

export const NFTCarousel: React.FC<NFTCarouselProps> = ({
  nfts,
  isSpinning,
  targetNFT,
  onSpinComplete,
}) => {
  const hasSpunRef = useRef(false);
  const isDesktop = useIsDesktop();

  const cardWidth = isDesktop ? CARD_WIDTH_DESKTOP : CARD_WIDTH_MOBILE;
  const cardGap = isDesktop ? CARD_GAP_DESKTOP : CARD_GAP_MOBILE;
  const itemTotalWidth = cardWidth + cardGap;

  const repetitions = MIN_ROTATIONS + 4;
  const extendedNfts = useMemo(() => {
    const result: NFTItem[] = [];
    for (let i = 0; i < repetitions; i++) {
      result.push(...nfts);
    }
    return result;
  }, [nfts, repetitions]);

  const singleSetWidth = nfts.length * itemTotalWidth;

  const initialOffset = -(2 * singleSetWidth);

  const motionX = useMotionValue(initialOffset);

  const spin = useCallback(async () => {
    if (!targetNFT || hasSpunRef.current) return;
    hasSpunRef.current = true;

    const targetIndex = nfts.findIndex((nft) => nft.id === targetNFT.id);
    if (targetIndex === -1) return;

    const fullRotationsDistance = MIN_ROTATIONS * singleSetWidth;
    const targetPosition = targetIndex * itemTotalWidth;
    const finalOffset = initialOffset - fullRotationsDistance - targetPosition;

    const startTime = performance.now();
    const startX = motionX.get();
    const deltaX = finalOffset - startX;

    const cubicBezier = (t: number) => {
      const p1 = 0.1;
      const p2 = 0.7;
      const p3 = 0.3;
      const p4 = 1;
      const cx = 3 * p1;
      const bx = 3 * (p3 - p1) - cx;
      const ax = 1 - cx - bx;
      const cy = 3 * p2;
      const by = 3 * (p4 - p2) - cy;
      const ay = 1 - cy - by;
      const sampleCurveX = (t: number) => ((ax * t + bx) * t + cx) * t;
      const sampleCurveY = (t: number) => ((ay * t + by) * t + cy) * t;
      let x2 = t;
      for (let i = 0; i < 8; i++) {
        const x2minus = sampleCurveX(x2) - t;
        if (Math.abs(x2minus) < 0.001) break;
        const d2 = (3 * ax * x2 + 2 * bx) * x2 + cx;
        if (Math.abs(d2) < 0.000001) break;
        x2 = x2 - x2minus / d2;
      }
      return sampleCurveY(x2);
    };

    const animate = () => {
      const elapsed = performance.now() - startTime;
      const progress = Math.min(elapsed / (SPIN_DURATION * 1000), 1);
      const easedProgress = cubicBezier(progress);
      motionX.set(startX + deltaX * easedProgress);

      if (progress < 1) {
        requestAnimationFrame(animate);
      } else {
        onSpinComplete();
      }
    };

    requestAnimationFrame(animate);
  }, [targetNFT, nfts, singleSetWidth, itemTotalWidth, initialOffset, motionX, onSpinComplete]);

  useEffect(() => {
    if (isSpinning && targetNFT && !hasSpunRef.current) {
      spin();
    }
  }, [isSpinning, targetNFT, spin]);

  useEffect(() => {
    if (!isSpinning && !hasSpunRef.current) {
      motionX.set(initialOffset);
    }
  }, [isSpinning, motionX, initialOffset]);

  useEffect(() => {
    motionX.set(initialOffset);
  }, [motionX, initialOffset]);

  return (
    <div className="relative flex flex-col items-center w-screen -mx-4 overflow-x-clip overflow-y-visible">
      <div className="relative z-20 top-6">
        <img src="/images/points/carousel-union.png" alt="" className="w-8 h-auto" />
      </div>

      <div className="relative w-full">
        <div className="flex items-center py-4">
          <motion.div
            className="flex items-center"
            style={{
              gap: `${cardGap}px`,
              marginLeft: `calc(50% - ${cardWidth / 2}px)`,
              x: motionX,
            }}
          >
            {extendedNfts.map((nft, index) => (
              <NFTCard
                key={`${nft.id}-${index}`}
                nft={nft}
                index={index}
                motionX={motionX}
                itemTotalWidth={itemTotalWidth}
              />
            ))}
          </motion.div>
        </div>
      </div>

      <div className="relative z-20 bottom-6">
        <img src="/images/points/carousel-union.png" alt="" className="w-8 h-auto rotate-180" />
      </div>

      <div className="hidden lg:block absolute left-0 inset-y-0 w-48 bg-gradient-to-r from-[#1a1714] via-[#1a1714]/80 to-transparent z-30 pointer-events-none" />
      <div className="hidden lg:block absolute right-0 inset-y-0 w-48 bg-gradient-to-l from-[#1a1714] via-[#1a1714]/80 to-transparent z-30 pointer-events-none" />
    </div>
  );
};
