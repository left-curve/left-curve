import { numberMask, useInputs } from "@left-curve/applets-kit";
import { usePoolLiquidityState, usePrices } from "@left-curve/store";
import { useApp } from "~/hooks/useApp";

import {
  Badge,
  Button,
  Input,
  PairAssets,
  Range,
  Tabs,
  createContext,
  twMerge,
} from "@left-curve/applets-kit";
import { motion } from "framer-motion";

import { formatNumber } from "@left-curve/dango/utils";
import Big from "big.js";
import { m } from "~/paraglide/messages";

import type { PairUpdate } from "@left-curve/dango/types";
import type { PropsWithChildren } from "react";
import { Modals } from "../modals/RootModal";

const [PoolLiquidityProvider, usePoolLiquidity] = createContext<{
  state: ReturnType<typeof usePoolLiquidityState>;
  controllers: ReturnType<typeof useInputs>;
}>({
  name: "PoolLiquidityContext",
});

type PoolLiquidityProps = {
  pair: PairUpdate;
  action: "deposit" | "withdraw";
  onChangeAction: (action: "deposit" | "withdraw") => void;
};

const PoolLiquidityContainer: React.FC<PropsWithChildren<PoolLiquidityProps>> = ({
  children,
  pair,
  action,
  onChangeAction,
}) => {
  const controllers = useInputs({ strategy: "onChange" });

  const state = usePoolLiquidityState({
    pair,
    action,
    onChangeAction,
    controllers,
  });

  return (
    <PoolLiquidityProvider value={{ state, controllers }}>
      <motion.div
        layout="position"
        className={twMerge(
          "w-full mx-auto flex flex-col pt-6 mb-16 gap-8 p-4",
          state.userHasLiquidity ? "md:max-w-[50.1875rem]" : "md:max-w-[25rem]",
        )}
      >
        {children}
      </motion.div>
    </PoolLiquidityProvider>
  );
};

const PoolLiquidityHeader: React.FC = () => {
  const { state } = usePoolLiquidity();
  const { coins, userHasLiquidity } = state;

  const { base, quote } = coins;

  return (
    <div
      className={twMerge(
        "flex flex-col gap-3 justify-between p-4 rounded-xl shadow-account-card bg-rice-50 relative w-full overflow-hidden",
        { "lg:flex-row": userHasLiquidity },
      )}
    >
      <div className="flex gap-2 items-center">
        <PairAssets assets={[base, quote]} />
        <p className="text-gray-700 h4-bold">
          {base.symbol}/{quote.symbol}
        </p>
        <Badge color="green" size="s" text="Stable Strategy" />
      </div>
      <div
        className={twMerge("flex flex-row justify-between items-center", {
          "lg:justify-center lg:gap-8": userHasLiquidity,
        })}
      >
        <div
          className={twMerge("flex flex-col items-start gap-0 ", {
            "lg:flex-row lg:gap-1 lg:items-center": userHasLiquidity,
          })}
        >
          <p className="text-gray-500 diatype-xs-medium">{m["poolLiquidity.apy"]()}</p>
          <p className="text-gray-700 diatype-sm-bold">-</p>
        </div>
        <div
          className={twMerge("flex flex-col items-center gap-0 ", {
            "lg:flex-row lg:gap-1": userHasLiquidity,
          })}
        >
          <p className="text-gray-500 diatype-xs-medium">{m["poolLiquidity.24hVol"]()}</p>
          <p className="text-gray-700 diatype-sm-bold">-</p>
        </div>
        <div
          className={twMerge("flex flex-col items-end gap-0 ", {
            "lg:flex-row lg:gap-1 lg:items-center": userHasLiquidity,
          })}
        >
          <p className="text-gray-500 diatype-xs-medium">{m["poolLiquidity.tvl"]()}</p>
          <p className="text-gray-700 diatype-sm-bold">-</p>
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

const PoolLiquidityUserLiquidity: React.FC = () => {
  const { settings } = useApp();
  const { state } = usePoolLiquidity();
  const { formatNumberOptions } = settings;
  const { coins, userHasLiquidity, userLiquidity } = state;
  const { base, quote } = coins;

  const { getPrice } = usePrices({ defaultFormatOptions: formatNumberOptions });

  if (!userHasLiquidity || !userLiquidity.data) return null;

  const { innerBase, innerQuote } = userLiquidity.data;

  const basePrice = getPrice(innerBase, base.denom);
  const quotePrice = getPrice(innerQuote, quote.denom);

  const totalPrice = formatNumber(Big(basePrice).plus(quotePrice).toString(), {
    ...formatNumberOptions,
    currency: "USD",
  });

  return (
    <div className="flex p-4 flex-col gap-4 rounded-xl bg-rice-25 shadow-account-card w-full h-fit">
      <div className="flex items-center justify-between">
        <p className="exposure-sm-italic text-gray-500">{m["poolLiquidity.liquidity"]()}</p>
        <p className="h4-bold text-gray-900">{totalPrice}</p>
      </div>
      <div className="flex flex-col w-full gap-2">
        <div className="flex items-center justify-between">
          <div className="flex gap-1 items-center justify-center">
            <img src={base.logoURI} alt={base.symbol} className="w-8 h-8" />
            <p className="text-gray-500 diatype-m-regular">{base.symbol}</p>
          </div>
          <p className="text-gray-700 diatype-m-regular">
            {formatNumber(innerBase, formatNumberOptions)}{" "}
            <span className="text-gray-500">
              ({formatNumber(basePrice, { ...formatNumberOptions, currency: "USD" })})
            </span>
          </p>
        </div>
        <div className="flex items-center justify-between">
          <div className="flex gap-1 items-center justify-center">
            <img src={quote.logoURI} alt={quote.symbol} className="w-8 h-8" />
            <p className="text-gray-500 diatype-m-regular">{quote.symbol}</p>
          </div>
          <p className="text-gray-700 diatype-m-regular">
            {formatNumber(innerQuote, formatNumberOptions)}{" "}
            <span className="text-gray-500">
              ({formatNumber(quotePrice, { ...formatNumberOptions, currency: "USD" })})
            </span>
          </p>
        </div>
      </div>
    </div>
  );
};

const PoolLiquidityDeposit: React.FC = () => {
  const { settings, showModal } = useApp();
  const { state, controllers } = usePoolLiquidity();
  const { formatNumberOptions } = settings;
  const { coins, action, deposit } = state;
  const { base, quote } = coins;
  const { register, setValue } = controllers;

  const { getPrice } = usePrices({ defaultFormatOptions: formatNumberOptions });

  if (action !== "deposit") return null;

  return (
    <>
      <div className="flex flex-col gap-2">
        <p className="exposure-sm-italic text-gray-700">{m["poolLiquidity.deposit"]()}</p>
        <div className="flex flex-col rounded-xl bg-rice-25 shadow-account-card">
          <Input
            {...register("baseAmount", {
              validate: (v) => {
                if (Number(v) > Number(base.balance))
                  return m["errors.validations.insufficientFunds"]();
                return true;
              },
              mask: numberMask,
            })}
            placeholder="0"
            startText="right"
            startContent={
              <div className="flex items-center gap-2 pl-4">
                <img src={base.logoURI} alt={base.symbol} className="w-8 h-8 rounded-full" />
                <p className="text-gray-500 diatype-lg-medium">{base.symbol}</p>
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
                  <span>
                    {base.balance} {base.symbol}
                  </span>
                  <Button
                    type="button"
                    variant="secondary"
                    size="xs"
                    className="bg-red-bean-50 text-red-bean-500 hover:bg-red-bean-100 focus:[box-shadow:0px_0px_0px_3px_#F575893D] py-[2px] px-[6px]"
                    onClick={() => setValue("baseAmount", base.balance)}
                  >
                    {m["common.max"]()}
                  </Button>
                </div>
                <p className="text-gray-500 diatype-sm-regular">
                  {getPrice(base.amount, base.denom, { format: true })}
                </p>
              </div>
            }
          />
          <span className="w-full h-[1px] bg-gray-100" />
          <Input
            {...register("quoteAmount", {
              mask: numberMask,
              validate: (v) => {
                if (Number(v) > Number(quote.balance))
                  return m["errors.validations.insufficientFunds"]();
                return true;
              },
            })}
            placeholder="0"
            startText="right"
            startContent={
              <div className="flex items-center gap-2 pl-4">
                <img src={quote.logoURI} alt={quote.symbol} className="w-8 h-8 rounded-full" />
                <p className="text-gray-500 diatype-lg-medium">{quote.symbol}</p>
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
                  <span>
                    {quote.balance} {quote.symbol}
                  </span>
                  <Button
                    type="button"
                    variant="secondary"
                    size="xs"
                    className="bg-red-bean-50 text-red-bean-500 hover:bg-red-bean-100 focus:[box-shadow:0px_0px_0px_3px_#F575893D] py-[2px] px-[6px]"
                    onClick={() => setValue("quoteAmount", quote.balance)}
                  >
                    {m["common.max"]()}
                  </Button>
                </div>
                <p className="text-gray-500 diatype-sm-regular">
                  {getPrice(quote.amount, quote.denom, { format: true })}
                </p>
              </div>
            }
          />
        </div>
      </div>
      <Button
        size="md"
        fullWidth
        isLoading={deposit.isPending}
        onClick={() =>
          showModal(Modals.PoolAddLiquidity, {
            coins,
            confirmAddLiquidity: deposit.mutateAsync,
            rejectAddLiquidity: deposit.reset,
          })
        }
      >
        {m["common.deposit"]()}
      </Button>
    </>
  );
};

const PoolLiquidityWithdraw: React.FC = () => {
  const { showModal } = useApp();
  const { state, controllers } = usePoolLiquidity();
  const { coins, action, withdraw, withdrawPercent, withdrawAmount } = state;
  const { setValue } = controllers;

  const { base, quote } = coins;

  if (action !== "withdraw") return null;

  return (
    <>
      <div className="flex flex-col gap-4">
        <div className="flex flex-col gap-2">
          <p className="exposure-sm-italic text-gray-700">{m["poolLiquidity.withdrawAmount"]()}</p>
          <div className="flex rounded-xl bg-rice-25 shadow-account-card flex-col gap-2 p-4 items-center">
            <p className="h1-regular text-gray-700">{withdrawPercent}%</p>
            <Range
              isDisabled={withdraw.isPending}
              minValue={0}
              maxValue={100}
              value={+withdrawPercent}
              onChange={(v) => setValue("withdrawPercent", v.toString())}
            />
            <div className="flex gap-2 items-center justify-center mt-2">
              <Button
                isDisabled={withdraw.isPending}
                size="xs"
                variant="secondary"
                onClick={() => setValue("withdrawPercent", "25")}
              >
                25%
              </Button>
              <Button
                isDisabled={withdraw.isPending}
                size="xs"
                variant="secondary"
                onClick={() => setValue("withdrawPercent", "50")}
              >
                50%
              </Button>
              <Button
                isDisabled={withdraw.isPending}
                size="xs"
                variant="secondary"
                onClick={() => setValue("withdrawPercent", "75")}
              >
                75%
              </Button>
              <Button
                isDisabled={withdraw.isPending}
                size="xs"
                variant="secondary"
                onClick={() => setValue("withdrawPercent", "100")}
              >
                {m["common.max"]()}
              </Button>
            </div>
          </div>
        </div>
        <div className="w-full flex flex-col gap-1 diatype-sm-regular">
          <div className="flex items-center justify-between gap-2">
            <div className="flex items-center gap-2">
              <img src={base.logoURI} alt={base.symbol} className="w-8 h-8" />
              <p>{base.symbol}</p>
            </div>
            <p>{withdrawAmount.base}</p>
          </div>
          <div className="flex items-center justify-between gap-2">
            <div className="flex items-center gap-2">
              <img src={quote.logoURI} alt={quote.symbol} className="w-8 h-8" />
              <p>{quote.symbol}</p>
            </div>
            <p>{withdrawAmount.quote}</p>
          </div>
        </div>
      </div>
      <Button
        size="md"
        fullWidth
        isLoading={withdraw.isPending}
        onClick={() =>
          showModal(Modals.PoolWithdrawLiquidity, {
            confirmWithdrawal: withdraw.mutateAsync,
            rejectWithdrawal: withdraw.reset,
          })
        }
      >
        {m["common.withdraw"]()}
      </Button>
    </>
  );
};

const PoolLiquidityHeaderTabs: React.FC = () => {
  const { state } = usePoolLiquidity();
  const { action, onChangeAction, userHasLiquidity } = state;

  return (
    <Tabs
      layoutId="tabs-send-and-receive"
      selectedTab={action}
      keys={userHasLiquidity ? ["deposit", "withdraw"] : ["deposit"]}
      fullWidth
      onTabChange={(tab) => onChangeAction(tab as "deposit" | "withdraw")}
    />
  );
};

export const PoolLiquidity = Object.assign(PoolLiquidityContainer, {
  Header: PoolLiquidityHeader,
  HeaderTabs: PoolLiquidityHeaderTabs,
  UserLiquidity: PoolLiquidityUserLiquidity,
  Deposit: PoolLiquidityDeposit,
  Withdraw: PoolLiquidityWithdraw,
});
