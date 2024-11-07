import { useAccount, useBalances, usePrices } from "@leftcurve/react";
import { useQueryState } from "nuqs";
import type React from "react";
import { Button, GradientContainer, Input } from "../";
import { twMerge } from "../../utils";

interface Props {
  onRequestPoolSelection?: () => void;
}

export const PoolManagment: React.FC<Props> = ({ onRequestPoolSelection }) => {
  const { account } = useAccount();
  const { data: balances = {} } = useBalances({ address: account?.address });
  const { calculateBalance } = usePrices();

  const [action] = useQueryState("action");
  const [poolId] = useQueryState("pool");

  const totalBalance = calculateBalance(balances, { format: true });

  return (
    <div className="flex flex-col gap-12 w-full items-center">
      <GradientContainer className="flex flex-col gap-2 w-full">
        <div className="h-[104px] w-[104px] flex items-center justify-center bg-surface-rose-200 rounded-full">
          <img
            src="/images/applets/deposit-and-withdraw.png"
            alt="deposit-and-withdraw"
            className="h-[74px] w-[74px] object-contain"
          />
        </div>
        <div className="flex sm:gap-4 sm:flex-row sm:items-start justify-center flex-col items-center">
          <div
            className={twMerge(
              "flex flex-col gap-1",
              action === "withdraw" ? "order-1" : "order-3",
            )}
          >
            <Button
              variant="bordered"
              className="bg-surface-green-300 hover:bg-surface-green-400 border-green-600/20 text-typography-green-500 rounded-2xl font-diatype-rounded not-italic px-4 min-w-40"
              onClick={onRequestPoolSelection}
            >
              stETH - USDC
            </Button>
            {action === "withdraw" ? (
              <p className="text-xs font-extrabold px-4">{totalBalance} Available</p>
            ) : null}
          </div>
          <p className="py-4 uppercase text-sm font-extrabold text-sand-900 font-diatype-rounded mx-2 tracking-widest text-start order-2">
            {action === "withdraw" ? "withdraw to" : "deposits into"}
          </p>
          <div
            className={twMerge(
              "flex flex-col gap-1",
              action === "withdraw" ? "order-3" : "order-1",
            )}
          >
            <Button
              variant="bordered"
              className={twMerge(
                "bg-surface-green-300 hover:bg-surface-green-400 border-green-600/20 text-typography-green-500 rounded-2xl font-diatype-rounded not-italic capitalize px-4 min-w-40",
              )}
            >
              {account?.type} #{account?.index}
            </Button>
            {action === "deposit" ? (
              <p className="text-xs font-extrabold px-4">{totalBalance} Available</p>
            ) : null}
          </div>
        </div>
      </GradientContainer>
      <div className="flex flex-col gap-6 w-full">
        <div className="w-full flex flex-col gap-2">
          <div className="w-full flex flex-col p-3 bg-surface-rose-200 rounded-[20px] items-start justify-center gap-1">
            <p className="font-extrabold text-typography-rose-500 tracking-widest uppercase text-sm">
              You {action}
            </p>
            <Input
              classNames={{ input: "text-3xl", inputWrapper: "py-4 pl-6 pr-4" }}
              placeholder="0"
              bottomComponent={
                <div className="w-full items-center justify-between px-6 text-typography-rose-600 text-xs flex font-bold uppercase tracking-widest my-2">
                  <p>BALANCE</p>
                  <p>$0</p>
                </div>
              }
              endContent={
                <div className="flex items-center justify-center gap-2 text-typography-black-200 ">
                  <img
                    src="https://raw.githubusercontent.com/cosmos/chain-registry/master/_non-cosmos/ethereum/images/wsteth.svg"
                    alt="usdc"
                    className="w-8 h-8 z-10"
                  />
                  <p>wsETH</p>
                </div>
              }
            />
          </div>
          <div className="w-full flex flex-col p-3 bg-surface-rose-200 rounded-[20px] items-start justify-center gap-1">
            <p className="font-extrabold text-typography-rose-500 tracking-widest uppercase text-sm">
              You {action}
            </p>
            <Input
              classNames={{ input: "text-3xl", inputWrapper: "py-4 pl-6 pr-4" }}
              placeholder="0"
              bottomComponent={
                <div className="w-full items-center justify-between px-6 text-typography-rose-600 text-xs flex font-bold uppercase tracking-widest my-2">
                  <p>BALANCE</p>
                  <p>$0</p>
                </div>
              }
              endContent={
                <div className="flex items-center justify-center gap-2 text-typography-black-200 ">
                  <img
                    src="https://raw.githubusercontent.com/cosmos/chain-registry/master/_non-cosmos/ethereum/images/usdc.svg"
                    alt="usdc"
                    className="w-8 h-8 z-10"
                  />
                  <p>USDC</p>
                </div>
              }
            />
          </div>
        </div>
        <Button className="capitalize">{action}</Button>
      </div>
    </div>
  );
};
