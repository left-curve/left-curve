import { Button, CoinSelector, Input, SwapArrowDownIcon } from "@left-curve/applets-kit";
import type React from "react";
import { useAccount, useConfig } from "../../../../../sdk/packages/dango/src/store/react";

export const SwapContainer: React.FC = () => {
  const { coins: chainCoins } = useConfig();
  const { chainId } = useAccount();
  const coins = chainCoins[chainId as string];
  const arrayOfCoins = Object.values(coins);

  return (
    <div className="dango-grid-4x4-L gap-4 flex flex-col justify-center items-center">
      <p className="font-extrabold text-typography-black-200 tracking-widest uppercase">Swap</p>

      <div className="flex flex-col gap-4 w-full max-w-[31.5rem] items-center justify-center">
        <div className="p-6 rounded-full bg-surface-rose-200 w-fit">
          <img src="/images/send-and-receive.webp" alt="transfer" className="w-[120px] h-[120px]" />
        </div>
        <div className="w-full flex flex-col p-3 bg-surface-rose-200 rounded-[20px] items-center justify-center">
          <div className="flex flex-col gap-2">
            <p className="font-extrabold text-typography-rose-500 tracking-widest uppercase text-sm">
              SELL
            </p>
            <Input
              classNames={{ input: "text-3xl", inputWrapper: "py-4 pl-6 pr-4" }}
              placeholder="0"
              bottomComponent={
                <div className="w-full items-center justify-between px-6 text-typography-rose-600 text-xs flex font-bold uppercase tracking-widest my-2">
                  <p>$0</p>
                </div>
              }
              endContent={<CoinSelector label="coins" coins={arrayOfCoins} />}
            />
          </div>
          <button
            type="button"
            className="my-[-0.75rem] p-0 h-12 w-12 flex items-center justify-center text-typography-purple-300 bg-surface-purple-100 border-2 border-typography-purple-300 hover:bg-surface-purple-300 shadow-md rounded-full"
          >
            <SwapArrowDownIcon />
          </button>
          <div className="flex flex-col gap-2">
            <p className="font-extrabold text-typography-rose-500 tracking-widest uppercase text-sm">
              BUY
            </p>
            <Input
              classNames={{ input: "text-3xl", inputWrapper: "py-4 pl-6 pr-4" }}
              placeholder="0"
              bottomComponent={
                <div className="w-full items-center justify-between px-6 text-typography-rose-600 text-xs flex font-bold uppercase tracking-widest my-2">
                  <p>$0</p>
                </div>
              }
              endContent={<CoinSelector label="coins" coins={arrayOfCoins} />}
            />
          </div>
        </div>
        <Button fullWidth>Swap</Button>
      </div>
    </div>
  );
};
