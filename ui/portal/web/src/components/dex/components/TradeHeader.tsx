import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { SearchToken } from "./SearchToken";
import {
  Badge,
  FormattedNumber,
  IconChevronDownFill,
  IconChevronLeft,
  IconChevronRight,
  PairStatValue,
  Tooltip,
  twMerge,
  useMediaQuery,
} from "@left-curve/applets-kit";
import { useProTrade } from "./ProTrade";
import { AnimatePresence, motion } from "framer-motion";
import { OpenInterestDisplay } from "./OpenInterestDisplay";
import { FundingCountdown } from "./FundingCountdown";
import { Decimal } from "@left-curve/utils";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import { useCurrentPrice, useOraclePrices, useAllPerpsPairStats } from "@left-curve/store";
import type React from "react";
import type { SearchTokenRow } from "./SearchToken";

export const TradeHeader: React.FC = () => {
  const { isLg } = useMediaQuery();
  const [isExpanded, setIsExpanded] = useState(isLg);

  const { pairId, perpsPairId, onChangePairId } = useProTrade();
  const pairStatsData = useAllPerpsPairStats((s) => s.perpsPairStatsByPairId[perpsPairId]);

  useEffect(() => {
    setIsExpanded(isLg);
  }, [isLg]);

  const handleChangePair = (row: SearchTokenRow) => {
    onChangePairId(`${row.baseCoin.symbol}-${row.quoteCoin.symbol}`);
  };

  return (
    <div className="flex bg-surface-primary-rice lg:gap-6 px-4 py-3 flex-col lg:flex-row w-full lg:justify-start shadow-account-card z-20 lg:z-10">
      <div className="flex gap-8 items-center justify-between lg:items-start w-full lg:w-auto">
        <div className="flex lg:flex-col gap-1">
          <SearchToken pairId={pairId} onChangePairId={handleChangePair} />
          <div className="lg:pl-8">
            <Badge text="Perp" color="green" size="s" />
          </div>
        </div>
        <div className="flex gap-2 items-center lg:hidden">
          <div
            className="cursor-pointer flex items-center justify-center"
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
            className="lg:flex-1 lg:min-w-0 flex items-center"
          >
            <HeaderMetricsScroller pairStatsData={pairStatsData} perpsPairId={perpsPairId} />
          </motion.div>
        ) : null}
      </AnimatePresence>
    </div>
  );
};

type HeaderMetricsScrollerProps = {
  pairStatsData: any;
  perpsPairId: string;
};

const HeaderMetricsScroller: React.FC<HeaderMetricsScrollerProps> = ({
  pairStatsData,
  perpsPairId,
}) => {
  const scrollRef = useRef<HTMLDivElement>(null);
  const [canScrollLeft, setCanScrollLeft] = useState(false);
  const [canScrollRight, setCanScrollRight] = useState(false);

  const checkScroll = useCallback(() => {
    const el = scrollRef.current;
    if (!el) return;
    const hasOverflow = el.scrollWidth > el.clientWidth;
    setCanScrollLeft(hasOverflow && el.scrollLeft > 1);
    setCanScrollRight(hasOverflow && el.scrollLeft < el.scrollWidth - el.clientWidth - 1);
  }, []);

  useEffect(() => {
    checkScroll();
    const el = scrollRef.current;
    if (!el) return;
    el.addEventListener("scroll", checkScroll);
    const observer = new ResizeObserver(checkScroll);
    observer.observe(el);
    return () => {
      el.removeEventListener("scroll", checkScroll);
      observer.disconnect();
    };
  }, [checkScroll]);

  return (
    <div className="relative">
      {canScrollLeft && (
        <button
          type="button"
          className="absolute left-0 top-0 bottom-0 w-8 z-10 bg-gradient-to-r from-surface-primary-rice to-transparent hidden lg:flex items-center justify-start pl-1 cursor-pointer"
          onClick={() => scrollRef.current?.scrollBy({ left: -100, behavior: "smooth" })}
        >
          <IconChevronLeft className="w-5 h-5 text-ink-tertiary-500" />
        </button>
      )}
      {canScrollRight && (
        <button
          type="button"
          className="absolute right-0 top-0 bottom-0 w-8 z-10 bg-gradient-to-l from-surface-primary-rice to-transparent hidden lg:flex items-center justify-end pr-1 cursor-pointer"
          onClick={() => scrollRef.current?.scrollBy({ left: 100, behavior: "smooth" })}
        >
          <IconChevronRight className="w-5 h-5 text-ink-tertiary-500" />
        </button>
      )}
      <div
        ref={scrollRef}
        className="gap-2 xxl:gap-6 grid grid-cols-3 lg:flex lg:flex-nowrap lg:items-center overflow-x-auto overflow-y-hidden diatype-xxs-medium lg:diatype-xs-medium scrollbar-none"
      >
        <span className="h-[1px] w-full bg-outline-tertiary-rice col-span-3 lg:hidden mt-2" />
        <HeaderPrice perpsPairId={perpsPairId} />
        <HeaderOraclePrice denom={perpsPairId} />
        <Header24hChange
          currentPrice={pairStatsData?.currentPrice}
          price24HAgo={pairStatsData?.price24HAgo}
          priceChange24H={pairStatsData?.priceChange24H}
        />
        <div className="flex gap-1 flex-col items-start lg:w-[5rem] lg:shrink-0">
          <p className="diatype-xxs-medium lg:diatype-xs-medium text-ink-tertiary-500">
            {m["dex.protrade.spot.volume"]()}
          </p>
          <PairStatValue
            kind="volume24h"
            value={pairStatsData?.volume24H}
            className="diatype-xxs-medium lg:diatype-xs-medium text-center"
          />
        </div>
        <OpenInterestDisplay />
        <FundingCountdown />
      </div>
    </div>
  );
};

const HeaderPrice: React.FC<{ perpsPairId: string }> = ({ perpsPairId }) => {
  const { currentPrice, previousPrice } = useCurrentPrice({ perpsPairId });

  return (
    <div className="flex gap-1 flex-col lg:w-[3.5rem] lg:shrink-0 items-start">
      <Tooltip title={m["dex.protrade.spot.lastPriceTooltip"]()}>
        <p className="diatype-xxs-medium lg:diatype-xs-medium text-ink-tertiary-500 cursor-help underline decoration-dashed underline-offset-[4px] decoration-current">
          {m["dex.protrade.spot.lastPrice"]()}
        </p>
      </Tooltip>
      <p
        className={twMerge(
          "diatype-xs-medium text-ink-secondary-700 h-[16.8px]",
          currentPrice && previousPrice
            ? Decimal(previousPrice).lte(currentPrice)
              ? "text-status-success"
              : "text-status-fail"
            : "",
        )}
      >
        {currentPrice ? <FormattedNumber number={currentPrice} as="span" /> : "-"}
      </p>
    </div>
  );
};

const HeaderOraclePrice: React.FC<{ denom: string }> = ({ denom }) => {
  const oraclePriceValue = useOraclePrices((s) => s.prices[denom]?.humanizedPrice ?? null);
  const oraclePrice = oraclePriceValue ? Number(oraclePriceValue) : null;

  return (
    <div className="flex gap-1 flex-col lg:w-[3.5rem] lg:shrink-0 items-start">
      <Tooltip title={m["dex.protrade.spot.oraclePriceTooltip"]()}>
        <p className="diatype-xxs-medium lg:diatype-xs-medium text-ink-tertiary-500 cursor-help underline decoration-dashed underline-offset-[4px] decoration-current">
          {m["dex.protrade.spot.oraclePrice"]()}
        </p>
      </Tooltip>
      <p className="diatype-xs-medium text-ink-secondary-700 h-[16.8px]">
        {oraclePrice ? <FormattedNumber number={oraclePrice} as="span" /> : "-"}
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

  const colorClass = useMemo(() => {
    if (!priceChange24H) return "text-ink-secondary-700";
    return Decimal(priceChange24H).gte(0) ? "text-status-success" : "text-status-fail";
  }, [priceChange24H]);

  return (
    <div className="flex gap-1 flex-col items-start lg:w-[7.5rem] lg:shrink-0">
      <p className="diatype-xxs-medium lg:diatype-xs-medium text-ink-tertiary-500">
        {m["dex.protrade.spot.24hChange"]()}
      </p>
      <p className={twMerge("diatype-xxs-medium lg:diatype-xs-medium", colorClass)}>
        {absoluteChange && priceChange24H ? (
          <>
            {isPositive ? "+" : ""}
            <FormattedNumber number={absoluteChange} as="span" />
            {" / "}
            {Decimal(priceChange24H).gte(0) ? "+" : ""}
            <FormattedNumber number={priceChange24H} as="span" />
            {"%"}
          </>
        ) : priceChange24H ? (
          <>
            {Decimal(priceChange24H).gte(0) ? "+" : ""}
            <FormattedNumber number={priceChange24H} as="span" />
            {"%"}
          </>
        ) : (
          "-"
        )}
      </p>
    </div>
  );
};
