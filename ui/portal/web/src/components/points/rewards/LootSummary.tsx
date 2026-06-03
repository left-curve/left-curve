import { Button, twMerge } from "@left-curve/applets-kit";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import { motion } from "framer-motion";
import type React from "react";

import type { LootDisplay } from "./loot";

export type LootBucket = { display: LootDisplay; count: number };

type LootSummaryProps = {
  buckets: readonly LootBucket[];
  onClose: () => void;
};

export const LootSummary: React.FC<LootSummaryProps> = ({ buckets, onClose }) => {
  const totalRevealed = buckets.reduce((sum, b) => sum + b.count, 0);

  const handleShareToX = () => {
    const text = m["points.chestOpening.shareBulkText"]({ count: totalRevealed });
    const url = `https://twitter.com/intent/tweet?text=${encodeURIComponent(text)}`;
    window.open(url, "_blank");
  };

  return (
    <motion.div
      className="bg-surface-primary-rice border border-outline-secondary-gray rounded-2xl overflow-hidden shadow-2xl w-[340px] lg:w-[600px] max-h-[90vh] flex flex-col"
      initial={{ opacity: 0, scale: 0.9, y: 20 }}
      animate={{ opacity: 1, scale: 1, y: 0 }}
      transition={{ duration: 0.3, ease: "easeOut" }}
    >
      <div className="flex-shrink-0 flex items-center justify-between px-4 py-3 border-b border-outline-secondary-gray">
        <div className="w-6" />
        <p className="diatype-m-medium text-ink-secondary-700">
          {m["points.chestOpening.summaryTitle"]()}
        </p>
        <button
          onClick={onClose}
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

      <div className="flex-1 min-h-0 overflow-y-auto p-4 lg:p-6">
        <div className="grid grid-cols-2 lg:grid-cols-3 gap-3 lg:gap-4">
          {buckets.map((bucket, index) => (
            <motion.div
              key={bucket.display.id}
              className="flex flex-col items-center gap-2"
              initial={{ opacity: 0, scale: 0.8, y: 20 }}
              animate={{ opacity: 1, scale: 1, y: 0 }}
              transition={{ delay: index * 0.05, duration: 0.3, ease: "easeOut" }}
            >
              <div
                className={twMerge(
                  "w-full aspect-[320/374] rounded-xl overflow-hidden max-w-[140px] lg:max-w-[160px]",
                  bucket.count === 0 && "opacity-50",
                )}
              >
                <img
                  src={bucket.display.frameSrc}
                  alt={bucket.display.label}
                  crossOrigin="anonymous"
                  className="w-full h-full object-cover"
                />
              </div>
              <p className="diatype-m-bold text-ink-primary-900">x{bucket.count}</p>
            </motion.div>
          ))}
        </div>
      </div>

      <div className="flex-shrink-0 px-4 lg:px-6 pb-4 lg:pb-6 pt-2 flex flex-col gap-3 border-t border-outline-secondary-gray">
        <Button variant="secondary" className="w-full" onClick={onClose}>
          {m["points.chestOpening.done"]()}
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
