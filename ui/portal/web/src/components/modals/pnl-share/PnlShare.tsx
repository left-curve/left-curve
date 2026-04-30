import { forwardRef, useRef, useState } from "react";
import { toPng } from "html-to-image";
import { Button, IconButton, IconClose, useApp, useTheme } from "@left-curve/applets-kit";
import { useAccount, useConfig, getReferralLink } from "@left-curve/store";
import { Decimal } from "@left-curve/dango/utils";
import { m } from "@left-curve/foundation/paraglide/messages.js";

import { CHARACTERS } from "./constants.js";
import { waitForImages } from "./utils.js";
import { cloneCardForExport } from "./buildExportCardHtml.js";
import { PreviewCard } from "./PreviewCard.js";
import { CharacterSelector } from "./CharacterSelector.js";
import type { PnlShareProps } from "./types.js";

export const PnlShare = forwardRef<unknown, PnlShareProps>((props, _ref) => {
  const { symbol, size, entryPrice, currentPrice, equity } = props;
  const { hideModal } = useApp();
  const { theme } = useTheme();
  const { coins } = useConfig();
  const { userIndex } = useAccount();

  const cardRef = useRef<HTMLDivElement>(null);
  const [selectedCharacter, setSelectedCharacter] = useState(0);

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
  const isDark = theme === "dark";
  const dangoLogoSrc = `/images/dango${isDark ? "-dark" : ""}.svg`;
  const characterImg = `/images/pnl-modal/${CHARACTERS[selectedCharacter]}.png`;

  const handleSaveImage = async () => {
    if (!cardRef.current) return;

    const clone = cloneCardForExport(cardRef.current, {
      symbol,
      entryPrice,
      currentPrice,
      displayPercent,
      isPositive,
      referralLink,
    });

    clone.style.width = "500px";

    const container = document.createElement("div");
    container.style.cssText = "position:fixed;left:-9999px;top:0;";
    container.appendChild(clone);
    document.body.appendChild(container);

    try {
      await waitForImages(clone);
      const dataUrl = await toPng(clone, { cacheBust: true });
      const link = document.createElement("a");
      link.download = `pnl-${symbol}.png`;
      link.href = dataUrl;
      link.click();
    } catch (err) {
      console.error("Failed to save image", err);
    } finally {
      document.body.removeChild(container);
    }
  };

  return (
    <div className="flex flex-col md:flex-row bg-surface-primary-rice md:border border-outline-secondary-gray pt-0 md:pt-6 rounded-xl relative p-4 md:p-6 gap-6 w-full md:max-w-[50rem]">
      <IconButton
        className="hidden md:block absolute right-4 top-4 z-10"
        variant="link"
        onClick={() => hideModal()}
      >
        <IconClose />
      </IconButton>

      <div className="flex-1 min-w-0">
        <PreviewCard
          ref={cardRef}
          symbol={symbol}
          entryPrice={entryPrice}
          currentPrice={currentPrice}
          displayPercent={displayPercent}
          isPositive={isPositive}
          isLong={isLong}
          leverage={leverage}
          characterImg={characterImg}
          dangoLogoSrc={dangoLogoSrc}
          logoURI={logoURI}
          referralLink={referralLink}
        />
      </div>

      <div className="flex flex-col gap-4 md:w-[16rem] shrink-0">
        {referralLink && (
          <div className="flex flex-col gap-1">
            <span className="exposure-sm-italic text-ink-tertiary-500">
              {m["modals.pnlShare.referralCode"]()}
            </span>
            <p className="diatype-xs-regular text-ink-secondary-700 break-all">{referralLink}</p>
          </div>
        )}

        <CharacterSelector selected={selectedCharacter} onSelect={setSelectedCharacter} />

        <div className="mt-auto">
          <Button fullWidth onClick={handleSaveImage}>
            {m["modals.pnlShare.saveImage"]()}
          </Button>
        </div>
      </div>
    </div>
  );
});
