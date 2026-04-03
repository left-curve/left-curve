import { useEffect, useMemo, useState } from "react";
import { SearchToken } from "./SearchToken";
import {
  Badge,
  IconChevronDownFill,
  PairStatValue,
  twMerge,
  useApp,
  useMediaQuery,
} from "@left-curve/applets-kit";
import { useProTrade } from "./ProTrade";
import { AnimatePresence, motion } from "framer-motion";
import { Decimal, formatNumber } from "@left-curve/dango/utils";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import {
  useConfig,
  useCurrentPrice,
  usePairStats,
  usePerpsPairStats,
  tradePairStore,
  toPerpsPairId,
} from "@left-curve/store";
import type React from "react";
import type { SearchTokenRow } from "./SearchToken";

import { OpenInterestDisplay } from "./OpenInterestDisplay";
import { FundingCountdown } from "./FundingCountdown";

export const TradeHeader: React.FC = () => {
  const { isLg } = useMediaQuery();
  const [isExpanded, setIsExpanded] = useState(isLg);

  const mode = tradePairStore((s) => s.mode);
  const pairId = tradePairStore((s) => s.pairId);
  const { coins } = useConfig();

  const { onChangePairId } = useProTrade();

  const perpsPairId = useMemo(() => {
    const baseSymbol = coins.byDenom[pairId.baseDenom]?.symbol;
    const quoteSymbol = coins.byDenom[pairId.quoteDenom]?.symbol ?? "USD";
    return baseSymbol ? toPerpsPairId(baseSymbol, quoteSymbol) : "";
  }, [pairId, coins]);

  const spotStats = usePairStats({
    baseDenom: pairId.baseDenom,
    quoteDenom: pairId.quoteDenom,
    enabled: mode === "spot",
  });

  const perpsStats = usePerpsPairStats({
    pairId: perpsPairId,
    enabled: mode === "perps" && !!perpsPairId,
  });

  const pairStats = mode === "perps" ? perpsStats : spotStats;

  useEffect(() => {
    setIsExpanded(isLg);
  }, [isLg]);

  const handleChangePair = (row: SearchTokenRow) => {
    onChangePairId(`${row.baseCoin.symbol}-${row.quoteCoin.symbol}`, row.mode);
  };

  return (
    <div className="flex bg-surface-primary-rice lg:gap-8 px-4 py-3 flex-col lg:flex-row w-full lg:justify-between shadow-account-card z-20 lg:z-10">
      <div className="flex gap-8 items-center justify-between lg:items-start w-full lg:w-auto">
        <div className="flex lg:flex-col gap-1">
          <SearchToken pairId={pairId} onChangePairId={handleChangePair} />
          <div className="lg:pl-8">
            {mode === "perps" ? (
              <Badge text="Perp" color="green" size="s" />
            ) : (
              <Badge text="Spot" color="blue" size="s" />
            )}
          </div>
        </div>
        <div className="flex gap-2 items-center">
          <div
            className="cursor-pointer flex items-center justify-center lg:hidden"
            onClick={() => setIsExpanded(!isExpanded)}
          >
            <IconChevronDownFill
              className={twMerge("text-ink-tertiary-500 w-4 h-4 transition-all", {
                "rotate-180": isExpanded,
              })}
            />
          </div>
        </div>
      </div>
      <AnimatePresence initial={false}>
        {isExpanded ? (
          <motion.div
            layout="position"
            layoutId="protrade-header"
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: "auto", opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={{ duration: isLg ? 0 : 0.3, ease: "easeInOut" }}
            className="gap-2 lg:gap-5 grid grid-cols-3 lg:flex lg:flex-wrap lg:items-center overflow-hidden"
          >
            <span className="h-[1px] w-full bg-outline-tertiary-rice col-span-3 lg:hidden mt-2" />
            <HeaderPrice />
            <Header24hChange
              currentPrice={pairStats.data?.currentPrice}
              price24HAgo={pairStats.data?.price24HAgo}
              priceChange24H={pairStats.data?.priceChange24H}
            />
            <div className="flex gap-1 flex-col items-start lg:min-w-[4rem]">
              <p className="diatype-xs-medium text-ink-tertiary-500">
                {m["dex.protrade.spot.volume"]()}
              </p>
              <PairStatValue
                kind="volume24h"
                value={pairStats.data?.volume24H}
                formatOptions={{ maximumTotalDigits: 5 }}
                className="diatype-sm-bold text-center"
              />
            </div>
            {mode === "perps" && perpsPairId && (
              <>
                <OpenInterestDisplay pairId={perpsPairId} />
                <FundingCountdown pairId={perpsPairId} />
              </>
            )}
          </motion.div>
        ) : null}
      </AnimatePresence>
    </div>
  );
};

const HeaderPrice: React.FC = () => {
  const { settings } = useApp();
  const { formatNumberOptions } = settings;

  const { currentPrice, previousPrice } = useCurrentPrice();

  return (
    <div className="flex gap-1 flex-col lg:min-w-[4rem] items-start">
      <p className="diatype-xs-medium text-ink-tertiary-500">{m["dex.protrade.history.price"]()}</p>
      <p
        className={twMerge(
          "diatype-sm-bold text-ink-secondary-700",
          currentPrice && previousPrice
            ? Decimal(previousPrice).lte(currentPrice)
              ? "text-status-success"
              : "text-status-fail"
            : "",
        )}
      >
        {currentPrice ? formatNumber(currentPrice, formatNumberOptions) : "-"}
      </p>
    </div>
  );
};

type Header24hChangeProps = {
  currentPrice: string | null | undefined;
  price24HAgo: string | null | undefined;
  priceChange24H: string | null | undefined;
};

const Header24hChange: React.FC<Header24hChangeProps> = ({
  currentPrice,
  price24HAgo,
  priceChange24H,
}) => {
  const { settings } = useApp();
  const { formatNumberOptions } = settings;

  const { absoluteChange, isPositive } = useMemo(() => {
    if (!currentPrice || !price24HAgo) {
      return { absoluteChange: null, isPositive: true };
    }

    const current = Decimal(currentPrice);
    const previous = Decimal(price24HAgo);
    const change = current.minus(previous);

    return {
      absoluteChange: change.toString(),
      isPositive: change.gte(0),
    };
  }, [currentPrice, price24HAgo]);

  const formattedAbsoluteChange = useMemo(() => {
    if (!absoluteChange) return null;

    const prefix = isPositive ? "+" : "";
    return `${prefix}${formatNumber(absoluteChange, {
      ...formatNumberOptions,
      maximumTotalDigits: 6,
    })}`;
  }, [absoluteChange, isPositive, formatNumberOptions]);

  const formattedPercentage = useMemo(() => {
    if (!priceChange24H) return null;

    const change = Decimal(priceChange24H);
    const prefix = change.gte(0) ? "+" : "";
    return `${prefix}${formatNumber(priceChange24H, {
      ...formatNumberOptions,
      maximumTotalDigits: 6,
    })}%`;
  }, [priceChange24H, formatNumberOptions]);

  const colorClass = useMemo(() => {
    if (!priceChange24H) return "text-ink-secondary-700";
    return Decimal(priceChange24H).gte(0) ? "text-status-success" : "text-status-fail";
  }, [priceChange24H]);

  return (
    <div className="flex gap-1 flex-col items-start lg:min-w-[4rem]">
      <p className="diatype-xs-medium text-ink-tertiary-500">
        {m["dex.protrade.spot.24hChange"]()}
      </p>
      <p className={twMerge("diatype-sm-bold tabular-nums lining-nums", colorClass)}>
        {formattedAbsoluteChange && formattedPercentage
          ? `${formattedAbsoluteChange} / ${formattedPercentage}`
          : formattedPercentage ?? "-"}
      </p>
    </div>
  );
};
