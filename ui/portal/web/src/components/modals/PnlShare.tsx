import { forwardRef, useRef, useState } from "react";
import { toPng } from "html-to-image";
import {
  Badge,
  Button,
  IconButton,
  IconClose,
  IconTwitter,
  twMerge,
  useApp,
} from "@left-curve/applets-kit";
import { useAccount, useConfig, getReferralLink } from "@left-curve/store";
import { m } from "@left-curve/foundation/paraglide/messages.js";

type PnlShareProps = {
  pairId: string;
  symbol: string;
  size: string;
  entryPrice: string;
  currentPrice: number;
  pnl: number;
};

const CHARACTER_IMAGES = [
  "cocodrile-2.svg",
  "froggo-2.svg",
  "alien.svg",
  "purple-bird.svg",
  "friends.svg",
];

export const PnlShare = forwardRef<unknown, PnlShareProps>((props, _ref) => {
  const { pairId, symbol, size, entryPrice, currentPrice, pnl } = props;
  const { hideModal } = useApp();
  const { coins } = useConfig();
  const { userIndex } = useAccount();

  const [selectedCharacter, setSelectedCharacter] = useState(0);
  const cardRef = useRef<HTMLDivElement>(null);

  const isLong = Number(size) > 0;
  const pnlPercent = ((currentPrice - Number(entryPrice)) / Number(entryPrice)) * 100;
  const displayPercent = isLong ? pnlPercent : -pnlPercent;
  const isPositive = displayPercent >= 0;

  const referralLink = getReferralLink(userIndex);

  const coinConfig = coins.bySymbol[symbol];
  const logoURI = coinConfig?.logoURI;

  const handleSaveImage = async () => {
    if (!cardRef.current) return;
    try {
      const dataUrl = await toPng(cardRef.current, { cacheBust: true });
      const link = document.createElement("a");
      link.download = `pnl-${symbol}.png`;
      link.href = dataUrl;
      link.click();
    } catch (err) {
      console.error("Failed to save image", err);
    }
  };

  const handleShareOnX = () => {
    const text = `Check out my PnL on @dangoxyz! ${displayPercent >= 0 ? "+" : ""}${displayPercent.toFixed(2)}% on ${symbol}`;
    window.open(`https://twitter.com/intent/tweet?text=${encodeURIComponent(text)}`, "_blank");
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

      {/* Shareable Card */}
      <div className="flex-1 min-w-0">
        <div
          ref={cardRef}
          className="bg-surface-secondary-rice rounded-2xl shadow-account-card p-6 relative overflow-hidden"
        >
          {/* Logo */}
          <img src="/images/dango.svg" alt="Dango" className="h-8 mb-4" />

          {/* Coin + Symbol + Badge */}
          <div className="flex items-center gap-2 mb-3">
            {logoURI && <img src={logoURI} alt={symbol} className="w-6 h-6 rounded-full" />}
            <span className="diatype-m-bold text-ink-primary-900">{symbol}</span>
            <Badge
              text={`${isLong ? "Long" : "Short"} 10x`}
              color={isLong ? "green" : "red"}
              size="s"
            />
          </div>

          {/* PnL Percentage */}
          <p
            className={twMerge(
              "exposure-h1-italic leading-tight mb-4",
              isPositive ? "text-utility-success-600" : "text-utility-error-600",
            )}
          >
            {isPositive ? "+" : ""}
            {displayPercent.toFixed(2)}%
          </p>

          {/* Entry Price + Mark Price */}
          <div className="flex gap-6 mb-3">
            <div className="flex flex-col">
              <span className="diatype-xs-regular text-ink-tertiary-500">
                {m["modals.pnlShare.entryPrice"]()}
              </span>
              <span className="diatype-sm-bold text-ink-primary-900">
                $
                {Number(entryPrice).toLocaleString(undefined, {
                  minimumFractionDigits: 2,
                  maximumFractionDigits: 2,
                })}
              </span>
            </div>
            <div className="flex flex-col">
              <span className="diatype-xs-regular text-ink-tertiary-500">
                {m["modals.pnlShare.markPrice"]()}
              </span>
              <span className="diatype-sm-bold text-ink-primary-900">
                $
                {currentPrice.toLocaleString(undefined, {
                  minimumFractionDigits: 2,
                  maximumFractionDigits: 2,
                })}
              </span>
            </div>
          </div>

          {/* Referral Code on card */}
          {referralLink && (
            <div className="mt-2">
              <span className="diatype-xs-regular text-ink-tertiary-500">
                {m["modals.pnlShare.referralCode"]()}
              </span>
              <p className="diatype-xs-regular text-ink-secondary-700 break-all">{referralLink}</p>
            </div>
          )}

          {/* Character overlay */}
          <img
            src={`/images/characters/${CHARACTER_IMAGES[selectedCharacter]}`}
            alt="character"
            className={twMerge(
              "absolute -right-8 bottom-8 lg:right-4 h-[80%] max-h-[12rem] lg:max-h-[17rem] opacity-90 pointer-events-none select-none",
              CHARACTER_IMAGES[selectedCharacter] === "purple-bird.svg" && "-scale-x-100",
              CHARACTER_IMAGES[selectedCharacter] === "alien.svg" && "-scale-x-100 lg:scale-x-100",
            )}
          />
        </div>
      </div>

      {/* Controls */}
      <div className="flex flex-col gap-4 md:w-[16rem] shrink-0">
        {/* Referral Code */}
        {referralLink && (
          <div className="flex flex-col gap-1">
            <span className="exposure-sm-italic text-ink-tertiary-500">
              {m["modals.pnlShare.referralCode"]()}
            </span>
            <p className="diatype-xs-regular text-ink-secondary-700 break-all">{referralLink}</p>
          </div>
        )}

        {/* Overlay selection */}
        <div className="flex flex-col gap-2">
          <span className="exposure-sm-italic text-ink-tertiary-500">
            {m["modals.pnlShare.overlay"]()}
          </span>
          <div className="grid grid-cols-5 gap-2">
            {CHARACTER_IMAGES.map((img, idx) => (
              <button
                key={img}
                type="button"
                onClick={() => setSelectedCharacter(idx)}
                className={twMerge(
                  "relative w-10 h-10 rounded-lg overflow-hidden border-2 cursor-pointer transition-colors",
                  idx === selectedCharacter
                    ? "border-primitives-red-light-500"
                    : "border-outline-secondary-gray hover:border-ink-tertiary-500",
                )}
              >
                <img
                  src={`/images/characters/${img}`}
                  alt={img.replace(".svg", "")}
                  className="w-full h-full object-cover"
                />
                {idx === selectedCharacter && (
                  <div className="absolute top-0.5 right-0.5 w-4 h-4 rounded-full bg-primitives-red-light-500 flex items-center justify-center">
                    <svg width="10" height="10" viewBox="0 0 10 10" fill="none">
                      <path
                        d="M2 5L4 7L8 3"
                        stroke="white"
                        strokeWidth="1.5"
                        strokeLinecap="round"
                        strokeLinejoin="round"
                      />
                    </svg>
                  </div>
                )}
              </button>
            ))}
          </div>
        </div>

        {/* Buttons */}
        <div className="flex flex-col gap-2 mt-auto">
          <div className="flex gap-2">
            <Button fullWidth onClick={handleSaveImage}>
              {m["modals.pnlShare.saveImage"]()}
            </Button>
            <Button fullWidth disabled>
              {m["modals.pnlShare.copyLink"]()}
            </Button>
          </div>
          <Button variant="primary" fullWidth onClick={handleShareOnX}>
            <IconTwitter className="w-4 h-4" />
            {m["modals.pnlShare.shareOnX"]()}
          </Button>
        </div>
      </div>
    </div>
  );
});
