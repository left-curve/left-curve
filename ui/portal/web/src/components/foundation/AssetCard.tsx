import { useConfig, usePrices } from "@left-curve/store";

import { FormattedNumber, PairAssets } from "@left-curve/applets-kit";
import { twMerge } from "@left-curve/applets-kit";
import { motion } from "framer-motion";

import { formatUnits } from "@left-curve/utils";
import { MarketPair } from "@left-curve/foundation/market-pair";
import { Image } from "~/components/foundation/Image";

import type { Coin } from "@left-curve/types";

const usd = MarketPair.USD;

interface SpotProps {
  coin: Coin;
}

const Spot: React.FC<SpotProps> = ({ coin }) => {
  const { coins } = useConfig();

  const coinInfo = coins.getCoinInfo(coin.denom);

  const humanAmount = formatUnits(coin.amount, coinInfo.decimals);

  const { getPrice } = usePrices();

  return (
    <motion.div layout="position" className="flex flex-col p-4 w-full">
      <div className={twMerge("flex items-center justify-between transition-all")}>
        <div className="flex gap-2 items-center">
          <div className="flex h-8 w-12">
            {coinInfo.type === "lp" ? (
              <PairAssets assets={[coinInfo.base, coinInfo.quote]} />
            ) : (
              <Image src={coinInfo.logoURI} className="h-8 w-8" alt={coinInfo.denom} />
            )}
          </div>
          <div className="flex flex-col">
            <p className="text-ink-primary-900 diatype-m-bold">{coinInfo.symbol}</p>
            <p className="text-ink-tertiary-500 diatype-m-regular">{coinInfo.name}</p>
          </div>
        </div>
        <div className="flex flex-col items-end text-ink-primary-900">
          <FormattedNumber
            className="diatype-m-bold"
            number={getPrice(humanAmount, coin.denom)}
            formatOptions={{ currency: "USD" }}
          />
          <FormattedNumber number={humanAmount} />
        </div>
      </div>
    </motion.div>
  );
};

interface PerpsProps {
  amount: string;
}

const Perp: React.FC<PerpsProps> = ({ amount }) => {
  return (
    <motion.div layout="position" className="flex flex-col p-4 w-full">
      <div className={twMerge("flex items-center justify-between transition-all")}>
        <div className="flex gap-2 items-center">
          <div className="flex h-8 w-12">
            <Image
              src={usd.logoURI}
              className="h-8 w-8"
              alt={usd.symbol}
            />
          </div>
          <div className="flex flex-col">
            <p className="text-ink-primary-900 diatype-m-bold">{usd.name}</p>
          </div>
        </div>
        <div className="flex flex-col items-end text-ink-primary-900">
          <FormattedNumber className="diatype-m-bold" number={amount} />
        </div>
      </div>
    </motion.div>
  );
};

interface VaultProps {
  shares: string;
  usdValue: string;
}

const Vault: React.FC<VaultProps> = ({ shares, usdValue }) => {
  return (
    <motion.div layout="position" className="flex flex-col p-4 w-full">
      <div className={twMerge("flex items-center justify-between transition-all")}>
        <div className="flex gap-2 items-center">
          <div className="flex h-8 w-12">
            <Image src={usd.logoURI} className="h-8 w-8" alt="DLP" />
          </div>
          <div className="flex flex-col">
            <p className="text-ink-primary-900 diatype-m-bold">DLP</p>
            <p className="text-ink-tertiary-500 diatype-m-regular">Dango Liquidity Provider</p>
          </div>
        </div>
        <div className="flex flex-col items-end text-ink-primary-900">
          <FormattedNumber
            className="diatype-m-bold"
            number={usdValue}
            formatOptions={{ currency: "USD" }}
          />
          <FormattedNumber number={shares} />
        </div>
      </div>
    </motion.div>
  );
};

export const AssetCard = Object.assign(Spot, { Spot, Perp, Vault });
