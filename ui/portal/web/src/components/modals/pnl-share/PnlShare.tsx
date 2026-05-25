import { forwardRef, useMemo, useRef, useState } from "react";
import { Button, IconButton, IconClose, useApp } from "@left-curve/applets-kit";
import { Decimal } from "@left-curve/utils";
import { saveCardAsImage } from "@left-curve/foundation";
import { getReferralLink, useAccount } from "@left-curve/store";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import { format } from "date-fns";

import { CharacterSelector } from "../../foundation/CharacterSelector";
import { shareCardFontEmbedCSS } from "../shareCardFonts.js";
import { PreviewCard } from "./PreviewCard.js";
import type { PnlShareProps } from "./types.js";

type NormalizedCardData = {
  symbol: string;
  size: string;
  referencePrice: string;
  markPrice: string | number;
  displayPercent: number;
  leverage: string | null;
  subtitle: string | undefined;
};

function normalize(props: PnlShareProps): NormalizedCardData {
  if (props.mode === "position") {
    const { symbol, size, entryPrice, currentPrice, equity } = props;
    const sizeD = Decimal(size);
    const entryD = Decimal(entryPrice);
    const currentD = Decimal(currentPrice);
    const equityD = equity ? Decimal(equity) : Decimal(0);

    const isLong = sizeD.gt(0);
    const leverageD = equityD.gt(0) ? sizeD.abs().mul(currentD).div(equityD) : null;
    const leverage = leverageD?.toFixed(2) ?? null;

    const priceChangePercent = currentD.minus(entryD).div(entryD).mul(100);
    const directionalPriceChange = isLong ? priceChangePercent : priceChangePercent.neg();
    const roiPercent = leverageD ? directionalPriceChange.mul(leverageD) : directionalPriceChange;
    return {
      symbol,
      size,
      referencePrice: entryPrice,
      markPrice: currentPrice,
      displayPercent: roiPercent.toNumber(),
      leverage,
      subtitle: undefined,
    };
  }
  const { symbol, size, fillPrice, realizedPnl, createdAt } = props;
  const sizeAbs = size.startsWith("-") ? size.slice(1) : size;
  const notional = Decimal(sizeAbs).times(Decimal(fillPrice));
  const displayPercent = notional.gt(0)
    ? Decimal(realizedPnl).div(notional).mul(100).toNumber()
    : 0;
  const subtitle = m["modals.pnlShare.closedAt"]({
    date: format(new Date(createdAt), "MMM d, yyyy HH:mm"),
  });
  return {
    symbol,
    size,
    referencePrice: fillPrice,
    markPrice: fillPrice,
    displayPercent,
    leverage: null,
    subtitle,
  };
}

export const PnlShare = forwardRef<unknown, PnlShareProps>((props, _ref) => {
  const { hideModal } = useApp();
  const { userIndex } = useAccount();
  const referralLink = getReferralLink(userIndex);

  const cardRef = useRef<HTMLDivElement>(null);
  const [selectedCharacter, setSelectedCharacter] = useState(0);

  const cardData = useMemo(() => normalize(props), [props]);

  const handleSaveImage = async () => {
    if (!cardRef.current) return;
    try {
      await saveCardAsImage({
        source: cardRef.current,
        filename: `pnl-${cardData.symbol}.png`,
        width: 752,
        fontEmbedCSS: shareCardFontEmbedCSS,
      });
    } catch (err) {
      console.error("Failed to save image", err);
    }
  };

  return (
    <div className="flex flex-col md:flex-row bg-surface-primary-rice md:border border-outline-secondary-gray pt-0 md:pt-6 rounded-xl relative p-4 md:p-6 gap-6 w-full md:max-w-[71.125rem]">
      <IconButton
        className="hidden md:block absolute right-4 top-4 z-10"
        variant="link"
        onClick={() => hideModal()}
      >
        <IconClose />
      </IconButton>

      <PreviewCard
        ref={cardRef}
        symbol={cardData.symbol}
        size={cardData.size}
        referencePrice={cardData.referencePrice}
        markPrice={cardData.markPrice}
        displayPercent={cardData.displayPercent}
        leverage={cardData.leverage}
        subtitle={cardData.subtitle}
        selectedCharacter={selectedCharacter}
      />

      <div className="flex flex-col gap-4 md:flex-1 md:min-w-0">
        {referralLink && (
          <div className="flex flex-col gap-1">
            <span className="exposure-sm-italic text-ink-tertiary-500">
              {m["modals.shareCard.referralCode"]()}
            </span>
            <p className="diatype-xs-regular text-ink-secondary-700 break-all">{referralLink}</p>
          </div>
        )}

        <CharacterSelector selected={selectedCharacter} onSelect={setSelectedCharacter} />

        <div className="mt-auto">
          <Button fullWidth onClick={handleSaveImage}>
            {m["modals.shareCard.saveImage"]()}
          </Button>
        </div>
      </div>
    </div>
  );
});
