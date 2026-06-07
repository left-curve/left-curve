import { Button } from "@left-curve/applets-kit";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import { motion } from "framer-motion";
import type React from "react";

import type { LootDisplay } from "./loot";
import { Image } from "~/components/foundation/Image";

type LootResultProps = {
  display: LootDisplay;
  onContinue: () => void;
  isOpenAllMode?: boolean;
  currentBoxIndex?: number;
  totalBoxesToOpen?: number;
  onNext?: () => void;
};

export const LootResult: React.FC<LootResultProps> = ({
  display,
  onContinue,
  isOpenAllMode = false,
  currentBoxIndex = 0,
  totalBoxesToOpen = 1,
  onNext,
}) => {
  const handleShareToX = () => {
    const text =
      display.kind === "hunted"
        ? m["points.chestOpening.shareBoostText"]({ multiplier: display.label.replace(/x.*/, "") })
        : m["points.chestOpening.shareNftText"]({
            article: /^[aeiou]/i.test(display.label) ? "an" : "a",
            label: display.label,
          });
    const url = `https://twitter.com/intent/tweet?text=${encodeURIComponent(text)}`;
    window.open(url, "_blank");
  };

  const isLastBox = currentBoxIndex >= totalBoxesToOpen - 1;
  const showNextButton = isOpenAllMode && !isLastBox;

  const handleButtonClick = () => {
    if (showNextButton && onNext) onNext();
    else onContinue();
  };

  const headerLabel =
    display.kind === "hunted"
      ? m["points.chestOpening.boosterTitle"]()
      : m["points.chestOpening.nftCardTitle"]();

  return (
    <motion.div
      className="bg-surface-primary-rice border border-outline-secondary-gray rounded-2xl overflow-hidden shadow-2xl w-[320px] lg:w-[380px]"
      initial={{ opacity: 0, scale: 0.9, y: 20 }}
      animate={{ opacity: 1, scale: 1, y: 0 }}
      transition={{ duration: 0.3, ease: "easeOut" }}
    >
      <div className="flex items-center justify-between px-4 py-3 border-b border-outline-secondary-gray">
        <div className="w-6" />
        <p className="diatype-m-medium text-ink-secondary-700">{headerLabel}</p>
        <button
          onClick={onContinue}
          className="text-ink-tertiary-500 hover:text-ink-primary-900 transition-colors"
          type="button"
          aria-label={m["points.chestOpening.closeLabel"]()}
        >
          <svg
            width="20"
            height="20"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
          >
            <path d="M18 6L6 18M6 6l12 12" />
          </svg>
        </button>
      </div>

      <div className="p-6 flex flex-col items-center gap-4">
        <motion.div
          className="w-full max-w-[280px] lg:max-w-[320px] aspect-[320/374] rounded-xl overflow-hidden"
          initial={{ scale: 0.8, opacity: 0 }}
          animate={{ scale: 1, opacity: 1 }}
          transition={{ delay: 0.1, duration: 0.4, ease: [0.34, 1.56, 0.64, 1] }}
        >
          <Image
            src={display.frameSrc}
            alt={display.label}
            crossOrigin="anonymous"
            className="w-full h-full object-cover"
          />
        </motion.div>

        {isOpenAllMode && (
          <p className="diatype-m-medium text-ink-secondary-700">
            {currentBoxIndex + 1}/{totalBoxesToOpen}
          </p>
        )}
      </div>

      <div className="px-6 pb-6 flex flex-col gap-3">
        <Button variant="secondary" className="w-full" onClick={handleButtonClick}>
          {showNextButton ? m["points.chestOpening.next"]() : m["points.chestOpening.done"]()}
        </Button>

        <button
          onClick={handleShareToX}
          className="flex items-center justify-center gap-2 text-ink-tertiary-500 hover:text-ink-primary-900 transition-colors diatype-sm-medium py-2"
          type="button"
        >
          <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
            <path d="M18.244 2.25h3.308l-7.227 8.26 8.502 11.24H16.17l-5.214-6.817L4.99 21.75H1.68l7.73-8.835L1.254 2.25H8.08l4.713 6.231zm-1.161 17.52h1.833L7.084 4.126H5.117z" />
          </svg>
          {m["points.chestOpening.shareToX"]()}
        </button>
      </div>
    </motion.div>
  );
};
