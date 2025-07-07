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

  const { getPrice } = usePrices({ defaultFormatOptions: formatNumberOptions });
  const price = getPrice(humanAmount, coin.denom, { format: true });

  return (
    <motion.div layout="position" className="flex flex-col p-4">
      <div className={twMerge("flex items-center justify-between transition-all")}>
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
        </div>
      </div>
    </motion.div>
  );
};
