import { Badge, FormattedNumber, twMerge, useTheme } from "@left-curve/applets-kit";
import { Decimal } from "@left-curve/dango/utils";
import { getReferralLink, useAccount, useConfig } from "@left-curve/store";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import type { Ref } from "react";

import { CHARACTERS } from "../../foundation/CharacterSelector";

type PnlCardProps = {
  ref?: Ref<HTMLDivElement>;
  symbol: string;
  size: string;
  entryPrice: string;
  currentPrice: number;
  equity: string | null;
  selectedCharacter: number;
};

export function PreviewCard({
  ref,
  symbol,
  size,
  entryPrice,
  currentPrice,
  equity,
  selectedCharacter,
}: PnlCardProps) {
  const { theme } = useTheme();
  const { coins } = useConfig();
  const { userIndex } = useAccount();

  const sizeD = Decimal(size);
  const entryD = Decimal(entryPrice);
  const currentD = Decimal(currentPrice);

  const isLong = sizeD.gt(0);
  const pnlPercent = currentD.minus(entryD).div(entryD).mul(100);
  const displayPercent = isLong ? pnlPercent.toNumber() : -pnlPercent.toNumber();
  const isPositive = displayPercent >= 0;

  const equityD = equity ? Decimal(equity) : Decimal(0);
  const leverage = equityD.gt(0) ? sizeD.abs().mul(currentD).div(equityD).toFixed(2) : null;

  const referralLink = getReferralLink(userIndex);
  const logoURI = coins.bySymbol[symbol]?.logoURI;
  const dangoLogoSrc = `/images/dango${theme === "dark" ? "-dark" : ""}.svg`;
  const characterImg = `/images/pnl-modal/${CHARACTERS[selectedCharacter]}.png`;

  const pctText = `${isPositive ? "+" : ""}${displayPercent.toFixed(2)}%`;

  return (
    <div
      ref={ref}
      className="bg-surface-secondary-rice rounded-2xl shadow-account-card p-6 relative overflow-hidden flex flex-col min-h-[345px] md:w-[47rem] md:h-[26.4375rem]"
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
        <div data-pnl="prices-row" className="flex flex-col md:flex-row gap-2 md:gap-6">
          <div className="flex flex-col">
            <span className="diatype-xs-regular text-ink-tertiary-500">
              {m["modals.pnlShare.entryPrice"]()}
            </span>
            <FormattedNumber
              as="span"
              number={entryPrice}
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
              number={currentPrice}
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
        className="absolute right-0 bottom-0 h-[60%] md:h-full max-h-[9rem] md:max-h-[24rem] opacity-90 pointer-events-none select-none"
      />
    </div>
  );
}
