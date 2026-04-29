import { forwardRef, useRef, useState } from "react";
import { toPng } from "html-to-image";
import {
  Badge,
  Button,
  IconButton,
  IconChecked,
  IconClose,
  twMerge,
  useApp,
  useTheme,
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

const CHARACTERS = [
  "frog1",
  "clouds",
  "frog2",
  "dog1",
  "bird2",
  "rich-cocodrile",
  "hippo",
  "pig",
  "candle",
  "bird-lost",
];

export const PnlShare = forwardRef<unknown, PnlShareProps>((props, _ref) => {
  const { pairId, symbol, size, entryPrice, currentPrice, pnl } = props;
  const { hideModal } = useApp();
  const { theme } = useTheme();
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
        <div
          ref={cardRef}
          className="bg-surface-secondary-rice rounded-2xl shadow-account-card p-6 relative overflow-hidden"
        >
          <img src={`/images/dango${theme === "dark" ? "-dark" : ""}.svg`} alt="Dango" className="relative z-10 h-8 mb-4" />

          <div className="relative z-10 flex items-center gap-2 mb-3">
            {logoURI && <img src={logoURI} alt={symbol} className="w-6 h-6 rounded-full" />}
            <span className="diatype-m-bold text-ink-primary-900">{symbol}</span>
            <Badge
              text={`${isLong ? "Long" : "Short"} 10x`}
              color={isLong ? "green" : "red"}
              size="s"
            />
          </div>

          <p
            className={twMerge(
              "relative z-10 exposure-h1-italic leading-tight mb-4",
              isPositive ? "text-utility-success-600" : "text-utility-error-600",
            )}
          >
            {isPositive ? "+" : ""}
            {displayPercent.toFixed(2)}%
          </p>

          <div className="relative z-10 flex flex-col md:flex-row gap-2 md:gap-6 mb-3">
            <div className="flex flex-col">
              <span className="diatype-xs-regular text-ink-tertiary-500">
                {m["modals.pnlShare.entryPrice"]()}
              </span>
              <span className="diatype-sm-bold text-ink-primary-900">
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

          {referralLink && (
            <div className="relative z-10 mt-2">
              <span className="diatype-xs-regular text-ink-tertiary-500">
                {m["modals.pnlShare.referralCode"]()}
              </span>
              <p className="diatype-xs-regular text-ink-secondary-700 break-all">{referralLink}</p>
            </div>
          )}

          <img
            src={`/images/pnl-modal/${CHARACTERS[selectedCharacter]}.png`}
            alt="character"
            className={twMerge(
              "absolute right-0 bottom-0 h-[60%] md:h-[80%] max-h-[9rem] md:max-h-[12rem] lg:max-h-[17rem] opacity-90 pointer-events-none select-none",
            )}
          />
        </div>
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

        {/* Overlay selection */}
        <div className="flex flex-col gap-2">
          <span className="exposure-sm-italic text-ink-tertiary-500">
            {m["modals.pnlShare.overlay"]()}
          </span>
          <div className="grid grid-cols-5 gap-2">
            {CHARACTERS.map((img, idx) => (
              <button
                key={img}
                type="button"
                onClick={() => setSelectedCharacter(idx)}
                className={twMerge(
                  "relative w-12 h-12 md:w-10 md:h-10 rounded-lg overflow-hidden border-2 cursor-pointer transition-colors",
                  idx === selectedCharacter
                    ? "border-primitives-red-light-500"
                    : "border-outline-secondary-gray hover:border-ink-tertiary-500",
                )}
              >
                <img
                  src={`/images/pnl-modal/${img}-thumbnail.png`}
                  alt={img}
                  className="w-full h-full object-cover"
                />
                {idx === selectedCharacter && (
                  <div className="absolute top-0.5 right-0.5 w-4 h-4 rounded-full bg-primitives-red-light-500 flex items-center justify-center">
                    <IconChecked className="w-2.5 h-2.5 text-white" />
                  </div>
                )}
              </button>
            ))}
          </div>
        </div>

        <div className="flex flex-col gap-2 mt-auto">
          <Button fullWidth onClick={handleSaveImage}>
            {m["modals.pnlShare.saveImage"]()}
          </Button>
        </div>
      </div>
    </div>
  );
});
