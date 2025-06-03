import {
  Badge,
  IconChevronDown,
  IconChevronDownFill,
  IconEmptyStar,
  twMerge,
  useMediaQuery,
} from "@left-curve/applets-kit";
import type React from "react";
import { useEffect, useState } from "react";

import { AnimatePresence, motion } from "framer-motion";

export const PairHeader: React.FC = () => {
  const { isLg } = useMediaQuery();
  const [expanded, setExpanded] = useState<boolean>(false);

  useEffect(() => {
    isLg ? setExpanded(true) : setExpanded(false);
  }, [isLg]);

  return (
    <div className="flex bg-rice-50 lg:gap-8 p-4 flex-col lg:flex-row w-full lg:justify-between">
      <div className="flex gap-8 items-center justify-between lg:items-start w-full lg:w-auto">
        <div className="flex lg:flex-col gap-2">
          <div className="flex gap-2 items-center">
            <img
              src="https://raw.githubusercontent.com/cosmos/chain-registry/master/noble/images/USDCoin.svg"
              alt=""
              className="h-7 w-7 drag-none select-none"
            />
            <p className="diatype-lg-heavy text-gray-700 min-w-fit">ETH-USDC</p>
          </div>
          <Badge text="Spot" color="blue" />
          <div
            className="cursor-pointer flex items-center justify-center lg:hidden"
            onClick={() => setExpanded(!expanded)}
          >
            <IconChevronDownFill
              className={twMerge("text-gray-500 w-4 h-4 transition-all", {
                "rotate-180": expanded,
              })}
            />
          </div>
        </div>
        <div className="flex gap-2 items-center">
          <IconEmptyStar className="w-5 h-5 text-gray-500" />
        </div>
      </div>
      <AnimatePresence initial={false}>
        {expanded && (
          <motion.div
            layout
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: "auto", opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={{ duration: 0.3, ease: "easeInOut" }}
            className="gap-2 lg:gap-4 grid grid-cols-1 lg:flex lg:flex-wrap overflow-hidden"
          >
            <div className="flex gap-1 flex-row lg:flex-col lg:items-start pt-8 lg:pt-0">
              <p className="diatype-sm-medium text-gray-500 lg:min-w-[8rem]">Mark</p>
              <p>83,565</p>
            </div>
            <div className="flex gap-1 flex-row lg:flex-col lg:items-start">
              <p className="diatype-sm-medium text-gray-500 lg:min-w-[8rem]">Last price</p>
              <p>$2,578</p>
            </div>
            <div className="flex gap-1 flex-row lg:flex-col lg:items-start">
              <p className="diatype-sm-medium text-gray-500 lg:min-w-[8rem]">Oracle</p>
              <p>83,565</p>
            </div>
            <div className="flex gap-1 flex-row lg:flex-col lg:items-start">
              <p className="diatype-sm-medium text-gray-500 lg:min-w-[8rem]">24h Change</p>
              <p className="text-red-bean-400">-542 / 0.70</p>
            </div>
            <div className="flex gap-1 flex-row lg:flex-col lg:items-start">
              <p className="diatype-sm-medium text-gray-500 lg:min-w-[8rem]">24h Volume</p>
              <p>$2,457,770,700.50</p>
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
};
