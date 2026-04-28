import { useEffect, useMemo, useState } from "react";
import { SearchToken } from "./SearchToken";
import {
  Badge,
  FormattedNumber,
  IconChevronDownFill,
  PairStatValue,
  Tooltip,
  twMerge,
  useMediaQuery,
} from "@left-curve/applets-kit";
import { useProTrade } from "./ProTrade";
import { AnimatePresence, motion } from "framer-motion";
import { OpenInterestDisplay } from "./OpenInterestDisplay";
import { FundingCountdown } from "./FundingCountdown";
import { Decimal } from "@left-curve/dango/utils";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import {
  useCurrentPrice,
  oraclePricesStore,
  allPairStatsStore,
  allPerpsPairStatsStore,
  TradePairStore,
} from "@left-curve/store";
import type React from "react";
import type { SearchTokenRow } from "./SearchToken";

export const TradeHeader: React.FC = () => {
  const { isLg } = useMediaQuery();
  const [isExpanded, setIsExpanded] = useState(isLg);

  const mode = TradePairStore((s) => s.mode);
  const pairId = TradePairStore((s) => s.pairId);
  const getPerpsPairId = TradePairStore((s) => s.getPerpsPairId);

  const { onChangePairId } = useProTrade();

  const statsByPair = allPairStatsStore((s) => s.pairStatsByKey);
  const statsByPairId = allPerpsPairStatsStore((s) => s.perpsPairStatsByPairId);

  const pairStatsData =
    mode === "perps"
      ? statsByPairId[getPerpsPairId()]
      : statsByPair[`${pairId.baseDenom}:${pairId.quoteDenom}`];

  useEffect(() => {
    setIsExpanded(isLg);
  }, [isLg]);

  const handleChangePair = (row: SearchTokenRow) => {
    onChangePairId(`${row.baseCoin.symbol}-${row.quoteCoin.symbol}`, row.mode);
  };

  return (
    <div className="flex bg-surface-primary-rice lg:gap-6 px-4 py-3 flex-col lg:flex-row w-full lg:justify-start shadow-account-card z-20 lg:z-10">
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
            className="gap-2 xxl:gap-6 grid grid-cols-3 lg:flex lg:justify-end lg:items-center overflow-hidden diatype-xxs-medium lg:diatype-xs-medium"
          >
            <span className="h-[1px] w-full bg-outline-tertiary-rice col-span-3 lg:hidden mt-2" />
            <HeaderPrice />
            {mode === "perps" && <HeaderOraclePrice denom={getPerpsPairId()} />}
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
            {mode === "perps" && (
              <>
                <OpenInterestDisplay />
                <FundingCountdown />
              </>
            )}
          </motion.div>
        ) : null}
      </AnimatePresence>
    </div>
  );
};

const HeaderPrice: React.FC = () => {
  const { currentPrice, previousPrice } = useCurrentPrice();

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
  const prices = oraclePricesStore((s) => s.prices);
  const oraclePrice = prices?.[denom]?.humanizedPrice ? Number(prices[denom].humanizedPrice) : null;

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
