import { Badge, FormattedNumber, twMerge, useTheme } from "@left-curve/applets-kit";
import { getReferralLink, useAccount, useConfig } from "@left-curve/store";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import type { Ref } from "react";

import { CHARACTERS } from "../../foundation/CharacterSelector";

export type PnlCardProps = {
  ref?: Ref<HTMLDivElement>;
  symbol: string;
  /** Signed size — sign determines long/short. */
  size: string;
  /** Price label shown under the "Entry Price" slot. */
  referencePrice: string;
  /**
   * Price label shown under the "Mark Price" slot. For historical fills
   * this equals `referencePrice` (the fill price); for live positions
   * it's the current mark.
   */
  markPrice: string | number;
  /** Precomputed signed percent — caller decides the formula. */
  displayPercent: number;
  /** Leverage suffix, e.g. "2.50". `null` omits the suffix from the badge. */
  leverage: string | null;
  /** Optional small subtitle line under the symbol row. */
  subtitle?: string;
  selectedCharacter: number;
};

export function PreviewCard({
  ref,
  symbol,
  size,
  referencePrice,
  markPrice,
  displayPercent,
  leverage,
  subtitle,
  selectedCharacter,
}: PnlCardProps) {
  const { theme } = useTheme();
  const { coins } = useConfig();
  const { userIndex } = useAccount();

  const isLong = !size.startsWith("-") && Number(size) > 0;
  const isPositive = displayPercent >= 0;

  const referralLink = getReferralLink(userIndex);
  const logoURI = coins.bySymbol[symbol]?.logoURI;
  const dangoLogoSrc = `/images/dango${theme === "dark" ? "-dark" : ""}.svg`;
  const characterImg = `/images/pnl-modal/${CHARACTERS[selectedCharacter]}.png`;

  const pctText = `${isPositive ? "+" : ""}${displayPercent.toFixed(2)}%`;

  return (
    <div
      ref={ref}
      className="group bg-surface-secondary-rice rounded-2xl shadow-account-card p-6 relative overflow-hidden flex flex-col min-h-[345px] lg:min-w-[47rem] lg:min-h-[26.4375rem] data-[export=true]:w-[47rem] data-[export=true]:h-[26.4375rem]"
    >
      <img src={dangoLogoSrc} alt="Dango" className="relative z-10 h-8 w-auto self-start" />

      <div className="relative z-10 flex-1 flex flex-col justify-center gap-3">
        <div className="flex items-center gap-2">
          {logoURI && <img src={logoURI} alt={symbol} className="w-6 h-6 rounded-full" />}
          <span className="diatype-m-bold text-ink-primary-900">{symbol}</span>
          <Badge
            text={`${isLong ? "Long" : "Short"}${leverage ? ` ${leverage}x` : ""}`}
            color={isLong ? "green" : "red"}
            size="s"
          />
        </div>
        {subtitle ? (
          <span className="diatype-xs-regular text-ink-tertiary-500">{subtitle}</span>
        ) : null}
        <p
          className={twMerge(
            "exposure-h1-italic leading-tight",
            isPositive ? "text-utility-success-600" : "text-utility-error-600",
          )}
        >
          {pctText}
        </p>
      </div>

      <div className="relative z-10 flex flex-col gap-2">
        <div className="flex flex-col lg:flex-row gap-2 lg:gap-6 group-data-[export=true]:flex-row group-data-[export=true]:gap-6">
          <div className="flex flex-col">
            <span className="diatype-xs-regular text-ink-tertiary-500">
              {m["modals.pnlShare.entryPrice"]()}
            </span>
            <FormattedNumber
              as="span"
              number={referencePrice}
              formatOptions={{ currency: "USD" }}
              className="diatype-sm-bold text-ink-primary-900"
            />
          </div>
          <div className="flex flex-col">
            <span className="diatype-xs-regular text-ink-tertiary-500">
              {m["modals.pnlShare.markPrice"]()}
            </span>
            <FormattedNumber
              as="span"
              number={markPrice}
              formatOptions={{ currency: "USD" }}
              className="diatype-sm-bold text-ink-primary-900"
            />
          </div>
        </div>

        {referralLink && (
          <div>
            <span className="diatype-xs-regular text-ink-tertiary-500">
              {m["modals.shareCard.referralCode"]()}
            </span>
            <p className="diatype-xs-regular text-ink-secondary-700 break-all">{referralLink}</p>
          </div>
        )}
      </div>

      <img
        src={characterImg}
        alt="character"
        className="absolute right-0 bottom-0 h-[60%] lg:h-full max-h-[9rem] lg:max-h-[24rem] group-data-[export=true]:h-full group-data-[export=true]:max-h-[24rem] opacity-90 pointer-events-none select-none"
      />
    </div>
  );
}
