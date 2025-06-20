import {
  Badge,
  Button,
  createContext,
  Input,
  PairAssets,
  Range,
  Tabs,
  twMerge,
} from "@left-curve/applets-kit";
import type { PairId } from "@left-curve/dango/types";
import { usePoolLiquidity } from "@left-curve/store";
import { useEffect, useState, type PropsWithChildren } from "react";
import { motion } from "framer-motion";

import { m } from "~/paraglide/messages";

const [PoolLiquidityProvider, usePoolLiquidityState] = createContext<{
  state: ReturnType<typeof usePoolLiquidity>;
}>({
  name: "PoolLiquidityContext",
});

type PoolLiquidityProps = {
  pairId: PairId;
};

const PoolLiquidityContainer: React.FC<PropsWithChildren<PoolLiquidityProps>> = ({
  children,
  pairId,
}) => {
  const [action, setAction] = useState<"deposit" | "withdraw">("deposit");
  const state = usePoolLiquidity({ pairId, action, onChangeAction: (v) => setAction(v) });

  useEffect(() => {
    setAction("deposit");
  }, [state.userLiquidity]);

  return (
    <PoolLiquidityProvider value={{ state }}>
      <motion.div
        layout="position"
        className={twMerge(
          "w-full mx-auto flex flex-col pt-6 mb-16 gap-8 p-4",
          state.userLiquidity ? "md:max-w-[50.1875rem]" : "md:max-w-[25rem]",
        )}
      >
        {children}
      </motion.div>
    </PoolLiquidityProvider>
  );
};

const PoolLiquidityHeader: React.FC = () => {
  const { state } = usePoolLiquidityState();
  const { coins, userLiquidity } = state;

  const { baseCoin, quoteCoin } = coins;

  return (
    <div
      className={twMerge(
        "flex flex-col gap-3 justify-between p-4 rounded-xl shadow-account-card bg-rice-50 relative w-full overflow-hidden",
        { "lg:flex-row": userLiquidity },
      )}
    >
      <div className="flex gap-2 items-center">
        <PairAssets assets={[baseCoin, quoteCoin]} />
        <p className="text-gray-700 h4-bold">
          {baseCoin.symbol}/{quoteCoin.symbol}
        </p>
        <Badge color="green" size="s" text="Stable Strategy" />
      </div>
      <div
        className={twMerge("flex flex-row justify-between items-center", {
          "lg:justify-center lg:gap-8": userLiquidity,
        })}
      >
        <div
          className={twMerge("flex flex-col items-start gap-0 ", {
            "lg:flex-row lg:gap-1 lg:items-center": userLiquidity,
          })}
        >
          <p className="text-gray-500 diatype-xs-medium">APY</p>
          <p className="text-gray-700 diatype-sm-bold">TBD</p>
        </div>
        <div
          className={twMerge("flex flex-col items-center gap-0 ", {
            "lg:flex-row lg:gap-1": userLiquidity,
          })}
        >
          <p className="text-gray-500 diatype-xs-medium">24h vol</p>
          <p className="text-gray-700 diatype-sm-bold">-</p>
        </div>
        <div
          className={twMerge("flex flex-col items-end gap-0 ", {
            "lg:flex-row lg:gap-1 lg:items-center": userLiquidity,
          })}
        >
          <p className="text-gray-500 diatype-xs-medium">TVL</p>
          <p className="text-gray-700 diatype-sm-bold">$15.63M</p>
        </div>
      </div>
      <img
        src="/images/characters/hippo.svg"
        alt="dango-hippo"
        className="max-w-[298px] absolute opacity-10 left-[8.75rem] top-0"
      />
    </div>
  );
};

const UserPoolLiquidity: React.FC = () => {
  const { state } = usePoolLiquidityState();
  const { coins, userLiquidity } = state;

  const { baseCoin, quoteCoin } = coins;

  if (!userLiquidity) return null;

  return (
    <div className="flex p-4 flex-col gap-4 rounded-xl bg-rice-25 shadow-account-card w-full h-fit">
      <div className="flex items-center justify-between">
        <p className="exposure-sm-italic text-gray-500">Liquidity</p>
        <p className="h4-bold text-gray-900">$1000.50</p>
      </div>
      <div className="flex flex-col w-full gap-2">
        <div className="flex items-center justify-between">
          <div className="flex gap-1 items-center justify-center">
            <img src={baseCoin.logoURI} alt={baseCoin.symbol} className="w-5 h-5 rounded-full" />
            <p className="text-gray-500 diatype-m-regular">{baseCoin.symbol}</p>
          </div>
          <p className="text-gray-700 diatype-m-regular">
            500.25 <span className="text-gray-500">($500.25)</span>
          </p>
        </div>
        <div className="flex items-center justify-between">
          <div className="flex gap-1 items-center justify-center">
            <img src={quoteCoin.logoURI} alt={quoteCoin.symbol} className="w-5 h-5 rounded-full" />
            <p className="text-gray-500 diatype-m-regular">{quoteCoin.symbol}</p>
          </div>
          <p className="text-gray-700 diatype-m-regular">
            500.25 <span className="text-gray-500">($500.25)</span>
          </p>
        </div>
      </div>
    </div>
  );
};

const PoolDeposit: React.FC = () => {
  const [amountToken0, setAmountToken0] = useState(0);
  const [amountToken1, setAmountToken1] = useState(0);
  const { state } = usePoolLiquidityState();
  const { coins, userLiquidity } = state;

  const { baseCoin, quoteCoin } = coins;
  return (
    <div className="flex flex-col gap-2">
      <p className="exposure-sm-italic text-gray-700">You deposit</p>
      <div className="flex flex-col rounded-xl bg-rice-25 shadow-account-card">
        <Input
          value={amountToken0}
          startText="right"
          startContent={
            <div className="flex items-center gap-2 pl-4">
              <img src={baseCoin.logoURI} alt={baseCoin.symbol} className="w-8 h-8 rounded-full" />
              <p className="text-gray-500 diatype-lg-medium">{baseCoin.symbol}</p>
            </div>
          }
          classNames={{
            inputWrapper:
              "pl-0 py-3 flex-col h-auto gap-[6px] bg-transparent shadow-none rounded-b-none",
            input: "!h3-medium",
          }}
          insideBottomComponent={
            <div className="w-full flex justify-between pl-4 h-[22px]">
              <div className="flex gap-1 items-center justify-center diatype-sm-regular text-gray-500">
                <span>160.00 {baseCoin.symbol}</span>
                <Button
                  type="button"
                  variant="secondary"
                  size="xs"
                  className="bg-red-bean-50 text-red-bean-500 hover:bg-red-bean-100 focus:[box-shadow:0px_0px_0px_3px_#F575893D] py-[2px] px-[6px]"
                  onClick={() => setAmountToken0(160)}
                >
                  {m["common.max"]()}
                </Button>
              </div>
              <p className="text-gray-500 diatype-sm-regular">$0.00</p>
            </div>
          }
        />
        <span className="w-full h-[1px] bg-gray-100" />
        <Input
          value={amountToken1}
          startText="right"
          startContent={
            <div className="flex items-center gap-2 pl-4">
              <img
                src={quoteCoin.logoURI}
                alt={quoteCoin.symbol}
                className="w-8 h-8 rounded-full"
              />
              <p className="text-gray-500 diatype-lg-medium">{quoteCoin.symbol}</p>
            </div>
          }
          classNames={{
            inputWrapper:
              "pl-0 py-3 flex-col h-auto gap-[6px] bg-transparent shadow-none rounded-t-none",
            input: "!h3-medium",
          }}
          insideBottomComponent={
            <div className="w-full flex justify-between pl-4 h-[22px]">
              <div className="flex gap-1 items-center justify-center diatype-sm-regular text-gray-500">
                <span>160.00 {quoteCoin.symbol}</span>
                <Button
                  type="button"
                  variant="secondary"
                  size="xs"
                  className="bg-red-bean-50 text-red-bean-500 hover:bg-red-bean-100 focus:[box-shadow:0px_0px_0px_3px_#F575893D] py-[2px] px-[6px]"
                  onClick={() => setAmountToken1(160)}
                >
                  {m["common.max"]()}
                </Button>
              </div>
              <p className="text-gray-500 diatype-sm-regular">$0.00</p>
            </div>
          }
        />
      </div>
    </div>
  );
};

const PoolWithdraw: React.FC = () => {
  const { state } = usePoolLiquidityState();
  const { coins } = state;

  const { baseCoin, quoteCoin } = coins;
  const [range, setRange] = useState(50);

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-col gap-2">
        <p className="exposure-sm-italic text-gray-700">Withdrawal amount</p>
        <div className="flex rounded-xl bg-rice-25 shadow-account-card flex-col gap-2 p-4 items-center">
          <p className="h1-regular text-gray-700">{range}%</p>
          <Range minValue={0} maxValue={100} value={range} onChange={(val) => setRange(val)} />
          <div className="flex gap-2 items-center justify-center mt-2">
            <Button size="xs" variant="secondary" onClick={() => setRange(25)}>
              25%
            </Button>
            <Button size="xs" variant="secondary" onClick={() => setRange(50)}>
              50%
            </Button>
            <Button size="xs" variant="secondary" onClick={() => setRange(75)}>
              75%
            </Button>
            <Button size="xs" variant="secondary" onClick={() => setRange(100)}>
              Max
            </Button>
          </div>
        </div>
      </div>
      <div className="w-full flex flex-col gap-1 diatype-sm-regular">
        <div className="flex items-center justify-between gap-2">
          <p className="text-gray-500">{baseCoin.symbol} amount</p>
          <div className="flex items-center gap-1 text-gray-700">
            <img src={baseCoin.logoURI} alt={baseCoin.symbol} className="w-4 h-4 rounded-full" />
            <p>120.00 {baseCoin.symbol}</p>
          </div>
        </div>
        <div className="flex items-center justify-between gap-2">
          <p className="text-gray-500">{quoteCoin.symbol} amount</p>
          <div className="flex items-center gap-1 text-gray-700">
            <img src={quoteCoin.logoURI} alt={quoteCoin.symbol} className="w-4 h-4 rounded-full" />
            <p>120.00 {quoteCoin.symbol}</p>
          </div>
        </div>
        <div className="flex items-center justify-between gap-2">
          <p className="text-gray-500">Network fee</p>
          <div className="flex items-center gap-1 text-gray-700">
            <img src={baseCoin.logoURI} alt={baseCoin.symbol} className="w-4 h-4 rounded-full" />
            <p>$0.02</p>
          </div>
        </div>
      </div>
    </div>
  );
};

const PoolDepositWithdraw: React.FC = () => {
  const { state } = usePoolLiquidityState();
  const { coins, action, onChangeAction, userLiquidity } = state;

  const { baseCoin, quoteCoin } = coins;

  return (
    <div className="w-full flex flex-col gap-4">
      <Tabs
        layoutId="tabs-send-and-receive"
        selectedTab={action}
        keys={userLiquidity ? ["deposit", "withdraw"] : ["deposit"]}
        fullWidth
        onTabChange={(tab) => onChangeAction(tab as "deposit" | "withdraw")}
      />
      {action === "deposit" ? <PoolDeposit /> : <PoolWithdraw />}
      <Button size="md" fullWidth>
        {action === "deposit" ? m["common.deposit"]() : m["common.withdraw"]()}
      </Button>
    </div>
  );
};

export const PoolLiquidity = Object.assign(PoolLiquidityContainer, {
  Header: PoolLiquidityHeader,
  UserLiquidity: UserPoolLiquidity,
  Deposit: PoolDeposit,
  Withdraw: PoolWithdraw,
  DepositWithdraw: PoolDepositWithdraw,
});
