import { useEffect, useState } from "react";
import { SearchToken } from "./SearchToken";
import {
  Badge,
  IconChevronDownFill,
  twMerge,
  useApp,
  useMediaQuery,
} from "@left-curve/applets-kit";
import { AnimatePresence, motion } from "framer-motion";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import { useOrderBookState, type useProTradeState } from "@left-curve/store";
import type React from "react";
import type { PairId } from "@left-curve/dango/types";
import { Decimal, formatNumber } from "@left-curve/dango/utils";

type TradeHeaderProps = {
  state: ReturnType<typeof useProTradeState>;
};

export const TradeHeader: React.FC<TradeHeaderProps> = ({ state }) => {
  const { isLg } = useMediaQuery();
  const [isExpanded, setIsExpanded] = useState(isLg);
  const { pairId, onChangePairId } = state;

  useEffect(() => {
    setIsExpanded(isLg);
  }, [isLg]);

  return (
    <div className="flex bg-surface-tertiary-rice lg:gap-8 px-4 py-3 flex-col lg:flex-row w-full lg:justify-between shadow-account-card z-20 lg:z-10">
      <div className="flex gap-8 items-center justify-between lg:items-start w-full lg:w-auto">
        <div className="flex lg:flex-col gap-[2px]">
          <SearchToken pairId={pairId} onChangePairId={onChangePairId} />
          <div className="lg:pl-8">
            <Badge text="Spot" color="blue" size="s" />
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
          {/*   <IconEmptyStar className="w-5 h-5 text-ink-tertiary-500" /> */}
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
            className="gap-2 lg:gap-5 grid grid-cols-1 lg:flex lg:flex-wrap lg:items-center overflow-hidden"
          >
            <HeaderPrice pairId={pairId} />
            <div className="items-center flex gap-1 flex-row lg:flex-col min-w-[4rem] lg:items-start">
              <p className="diatype-xs-medium text-ink-tertiary-500">
                {m["dex.protrade.spot.24hChange"]()}
              </p>
              <p className="diatype-sm-bold w-full text-center">-</p>
            </div>
            <div className="items-center flex gap-1 flex-row lg:flex-col min-w-[4rem] lg:items-start">
              <p className="diatype-xs-medium text-ink-tertiary-500">
                {m["dex.protrade.spot.volume"]()}
              </p>
              <p className="diatype-sm-bold w-full text-center">-</p>
            </div>
          </motion.div>
        ) : null}
      </AnimatePresence>
    </div>
  );
};

type HeaderPriceProps = {
  pairId: PairId;
};

const HeaderPrice: React.FC<HeaderPriceProps> = ({ pairId }) => {
  const { settings } = useApp();
  const { formatNumberOptions } = settings;

  const { orderBookStore } = useOrderBookState({ pairId });

  const previousPrice = orderBookStore((s) => s.previousPrice);
  const currentPrice = orderBookStore((s) => s.currentPrice);

  return (
    <div className="items-center flex gap-1 flex-row lg:flex-col min-w-[4rem] lg:items-start pt-8 lg:pt-0">
      <p className="diatype-xs-medium text-ink-tertiary-500">{m["dex.protrade.history.price"]()}</p>
      <p
        className={twMerge(
          "diatype-sm-bold text-ink-secondary-700",
          Decimal(previousPrice).lte(currentPrice) ? "text-status-fail" : "text-status-success",
        )}
      >
        {formatNumber(currentPrice, formatNumberOptions)}
      </p>
    </div>
  );
};
