import { useConfig, usePrices } from "@left-curve/store";
import { useApp } from "~/hooks/useApp";
import { useState } from "react";

import { PairAssets } from "@left-curve/applets-kit";
import type { Coin } from "@left-curve/dango/types";

import { AnimatePresence, motion } from "framer-motion";
import { IconChevronDownFill, twMerge } from "@left-curve/applets-kit";

import { formatNumber, formatUnits, uid } from "@left-curve/dango/utils";
interface Props {
  coin: Coin;
}

export const AssetCard: React.FC<Props> = ({ coin }) => {
  const { getCoinInfo } = useConfig();
  const { settings } = useApp();
  const { formatNumberOptions } = settings;

  const coinInfo = getCoinInfo(coin.denom);

  const humanAmount = formatUnits(coin.amount, coinInfo.decimals);
  const [isExpanded, setIsExpanded] = useState<boolean>(false);

  const { getPrice } = usePrices({ defaultFormatOptions: formatNumberOptions });
  const price = getPrice(humanAmount, coin.denom, { format: true });

  return (
    <motion.div
      layout="position"
      className="flex flex-col p-4 hover:bg-rice-50 hover:cursor-pointer"
      onClick={() => setIsExpanded(!isExpanded)}
    >
      <div
        className={twMerge("flex items-center justify-between transition-all", {
          "pb-2": isExpanded,
        })}
      >
        <div className="flex gap-2 items-center">
          <div className="flex h-8 w-12">
            {coinInfo.logoURI ? (
              <img src={coinInfo.logoURI} className="h-8 w-8" alt={coinInfo.denom} />
            ) : (
              <div className="h-8 w-8 rounded-full bg-gray-200" />
            )}
          </div>
          <div className="flex flex-col">
            <p className="text-gray-900 diatype-m-bold">{coinInfo.symbol}</p>
            <p className="text-gray-500 diatype-m-regular">
              {formatNumber(humanAmount, formatNumberOptions)}
            </p>
          </div>
        </div>
        <div className="flex flex-col items-end">
          <p className="text-gray-900 diatype-m-bold">{price}</p>
          <IconChevronDownFill
            className={twMerge("w-3 h-3 text-gray-200 transition-all", {
              "rotate-180": isExpanded,
            })}
          />
        </div>
      </div>
      <AnimatePresence initial={false}>
        {isExpanded && (
          <motion.div
            layout="position"
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: "auto", opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={{ duration: 0.3, ease: "easeInOut" }}
            className="overflow-hidden flex flex-col gap-2 pl-14 w-full"
          >
            <div className="flex items-center justify-between text-gray-500 diatype-m-regular">
              <p>Wallet</p>
              <p>$123</p>
            </div>
            <div className="flex items-center justify-between text-gray-500 diatype-m-regular">
              <p>Lending Market</p>
              <p>$123</p>
            </div>
            <div className="flex items-center justify-between text-gray-500 diatype-m-regular">
              <p>DEX</p>
              <p>$123</p>
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </motion.div>
  );
};
