import { forwardRef, useRef, useState } from "react";
import { Button, IconButton, IconClose, useApp } from "@left-curve/applets-kit";
import { saveCardAsImage } from "@left-curve/foundation";
import { getReferralLink, useAccount } from "@left-curve/store";
import { m } from "@left-curve/foundation/paraglide/messages.js";

import { CharacterSelector } from "../../foundation/CharacterSelector";
import { shareCardFontEmbedCSS } from "../shareCardFonts.js";
import { PreviewCard } from "./PreviewCard.js";
import type { PnlShareProps } from "./types.js";

export const PnlShare = forwardRef<unknown, PnlShareProps>((props, _ref) => {
  const { symbol, size, entryPrice, currentPrice, equity } = props;
  const { hideModal } = useApp();
  const { userIndex } = useAccount();
  const referralLink = getReferralLink(userIndex);

  const cardRef = useRef<HTMLDivElement>(null);
  const [selectedCharacter, setSelectedCharacter] = useState(0);

  const handleSaveImage = async () => {
    if (!cardRef.current) return;
    try {
      await saveCardAsImage({
        source: cardRef.current,
        filename: `pnl-${symbol}.png`,
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
        symbol={symbol}
        size={size}
        entryPrice={entryPrice}
        currentPrice={currentPrice}
        equity={equity}
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
