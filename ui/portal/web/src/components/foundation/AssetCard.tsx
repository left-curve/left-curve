import { useConfig, usePrices } from "@left-curve/store";
import { useApp } from "~/hooks/useApp";

import { PairAssets } from "@left-curve/applets-kit";
import { twMerge } from "@left-curve/applets-kit";
import { motion } from "framer-motion";

import { formatNumber, formatUnits } from "@left-curve/dango/utils";

import type { AnyCoin, WithAmount } from "@left-curve/store/types";
interface Props {
  coin: WithAmount<AnyCoin>;
}

export const AssetCard: React.FC<Props> = ({ coin }) => {
  const { getCoinInfo } = useConfig();
  const { settings } = useApp();
  const { formatNumberOptions } = settings;

  const coinInfo = getCoinInfo(coin.denom);

  const humanAmount = formatUnits(coin.amount, coinInfo.decimals);
  /*  const [isExpanded, setIsExpanded] = useState<boolean>(false); */

  const { getPrice } = usePrices({ defaultFormatOptions: formatNumberOptions });
  const price = getPrice(humanAmount, coin.denom, { format: true });

  return (
    <motion.div
      layout="position"
      className="flex flex-col p-4 hover:bg-rice-50 hover:cursor-pointer"
      /*  onClick={() => setIsExpanded(!isExpanded)} */
    >
      <div
        className={twMerge(
          "flex items-center justify-between transition-all" /* {
          "pb-2": isExpanded,
        } */,
        )}
      >
        <div className="flex gap-2 items-center">
          <div className="flex h-8 w-12">
            {coinInfo.type === "lp" ? (
              <PairAssets assets={[coinInfo.base, coinInfo.quote]} />
            ) : (
              <img src={coinInfo.logoURI} className="h-8 w-8" alt={coinInfo.denom} />
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
          {/* <IconChevronDownFill
            className={twMerge("w-4 h-4 text-gray-200 transition-all", {
              "rotate-180": isExpanded,
            })}
          /> */}
        </div>
      </div>
      {/* <AnimatePresence initial={false}>
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
              <p>{m["common.accountMenu.assets.wallet"]()}</p>
              <p>$0</p>
            </div>
            <div className="flex items-center justify-between text-gray-500 diatype-m-regular">
              <p>{m["common.accountMenu.assets.lendingMarket"]()}</p>
              <p>$0</p>
            </div>
            <div className="flex items-center justify-between text-gray-500 diatype-m-regular">
              <p>{m["common.accountMenu.assets.dex"]()}</p>
              <p>{price}</p>
            </div>
          </motion.div>
        )}
      </AnimatePresence> */}
    </motion.div>
  );
};
