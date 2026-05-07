import { Badge, twMerge } from "@left-curve/applets-kit";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import type { Ref } from "react";

import { formatPrice } from "./utils.js";
import type { PnlCardData } from "./types.js";

export function PreviewCard({
  ref,
  symbol,
  entryPrice,
  currentPrice,
  displayPercent,
  isPositive,
  isLong,
  leverage,
  characterImg,
  dangoLogoSrc,
  logoURI,
  referralLink,
}: PnlCardData & { ref?: Ref<HTMLDivElement> }) {
  const pctText = `${isPositive ? "+" : ""}${displayPercent.toFixed(2)}%`;

  return (
    <div
      ref={ref}
      className="bg-surface-secondary-rice rounded-2xl shadow-account-card p-6 relative overflow-hidden"
    >
      <img src={dangoLogoSrc} alt="Dango" className="relative z-10 h-8 mb-4" />

      <div className="relative z-10 flex items-center gap-2 mb-3">
        {logoURI && <img src={logoURI} alt={symbol} className="w-6 h-6 rounded-full" />}
        <span data-pnl="symbol" className="diatype-m-bold text-ink-primary-900">
          {symbol}
        </span>
        <Badge
          text={`${isLong ? "Long" : "Short"}${leverage ? ` ${leverage}x` : ""}`}
          color={isLong ? "green" : "red"}
          size="s"
        />
      </div>

      <p
        data-pnl="percent"
        className={twMerge(
          "relative z-10 exposure-h1-italic leading-tight mb-4",
          isPositive ? "text-utility-success-600" : "text-utility-error-600",
        )}
      >
        {pctText}
      </p>

      <div className="relative z-10 flex flex-col md:flex-row gap-2 md:gap-6 mb-3">
        <div className="flex flex-col">
          <span className="diatype-xs-regular text-ink-tertiary-500">
            {m["modals.pnlShare.entryPrice"]()}
          </span>
          <span data-pnl="entry-price" className="diatype-sm-bold text-ink-primary-900">
            {formatPrice(Number(entryPrice))}
          </span>
        </div>
        <div className="flex flex-col">
          <span className="diatype-xs-regular text-ink-tertiary-500">
            {m["modals.pnlShare.markPrice"]()}
          </span>
          <span data-pnl="mark-price" className="diatype-sm-bold text-ink-primary-900">
            {formatPrice(currentPrice)}
          </span>
        </div>
      </div>

      {referralLink && (
        <div className="relative z-10 mt-2">
          <span className="diatype-xs-regular text-ink-tertiary-500">
            {m["modals.pnlShare.referralCode"]()}
          </span>
          <p data-pnl="referral" className="diatype-xs-regular text-ink-secondary-700 break-all">
            {referralLink}
          </p>
        </div>
      )}

      <img
        src={characterImg}
        alt="character"
        className="absolute right-0 bottom-0 h-[60%] md:h-[80%] max-h-[9rem] md:max-h-[12rem] lg:max-h-[17rem] opacity-90 pointer-events-none select-none"
      />
    </div>
  );
}
