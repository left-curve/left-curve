import { formatUnits } from "@left-curve/utils";
import {
  useChainId,
  useConfig,
  usePrices,
} from "../../../../../../sdk/packages/dango/src/store/react";

import type { Pool, PoolId } from "@left-curve/types";

interface Props {
  poolId: PoolId;
  pool: Pool;
  onPoolSelected: (id: PoolId) => void;
}

export const PoolCard: React.FC<Props> = ({ onPoolSelected, poolId, pool }) => {
  const { coins } = useConfig();
  const chainId = useChainId();
  const { getPrice } = usePrices();

  const [poolType, poolInfo] = Object.entries(pool)[0];

  const chainCoins = coins[chainId];

  const firstCoin = poolInfo.liquidity[0];
  const secondCoin = poolInfo.liquidity[1];

  const firstCoinInfo = chainCoins[firstCoin.denom];
  const secondCoinInfo = chainCoins[secondCoin.denom];

  const firstCoinHumanAmount = formatUnits(firstCoin.amount, firstCoinInfo.decimals);
  const secondCoinHumanAmount = formatUnits(secondCoin.amount, secondCoinInfo.decimals);

  const firstCoinPrice = getPrice(firstCoinHumanAmount, firstCoinInfo.denom, { format: true });
  const secondCoinPrice = getPrice(secondCoinHumanAmount, secondCoinInfo.denom, { format: true });

  return (
    <div
      className="py-4 px-6 items-center gap-1 grid grid-cols-[1fr_80px_80px_80px] text-end
            bg-surface-rose-100 hover:bg-surface-off-white-200 border-2 border-surface-off-white-500
          text-typography-black-100 hover:text-typography-black-300 rounded-2xl transition-all cursor-pointer font-normal leading-5"
      onClick={() => onPoolSelected(poolId)}
    >
      <div className="flex gap-3 items-center">
        <div className="flex">
          <img src={firstCoinInfo.logoURI} alt={firstCoinInfo.denom} className="w-6 h-6 z-10" />
          <img
            src={secondCoinInfo.logoURI}
            alt={secondCoinInfo.denom}
            className="w-6 h-6 ml-[-0.5rem]"
          />
        </div>
        <p>
          {firstCoinInfo.symbol} - {secondCoinInfo.symbol}
        </p>
      </div>
      <p>{firstCoinPrice}</p>
      <p>{secondCoinPrice}</p>
      {/* TODO: Pending APR */}
      <p>0%</p>
    </div>
  );
};
