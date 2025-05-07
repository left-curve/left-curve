import { createContext, numberMask, twMerge, useInputs } from "@left-curve/applets-kit";
import { useSimpleSwap as state, useAccount, useBalances, usePrices } from "@left-curve/store";

import {
  Badge,
  Button,
  CoinSelector,
  IconArrowDown,
  IconGear,
  Input,
} from "@left-curve/applets-kit";

import { m } from "~/paraglide/messages";

import { formatUnits } from "@left-curve/dango/utils";
import type { UseSimpleSwapParameters } from "@left-curve/store";
import type { PropsWithChildren } from "react";
import type React from "react";
import { useApp } from "~/hooks/useApp";

const [SimpleSwapProvider, useSimpleSwap] = createContext<ReturnType<typeof state>>({
  name: "SimpleSwapContext",
});

const Root: React.FC<PropsWithChildren<UseSimpleSwapParameters>> = ({
  children,
  ...parameters
}) => {
  return <SimpleSwapProvider value={state(parameters)}>{children}</SimpleSwapProvider>;
};

const SimpleSwapHeader: React.FC = () => {
  const { statistics, quote } = useSimpleSwap();
  const { tvl, apy, volume } = statistics.data;
  return (
    <div className="flex flex-col gap-3 rounded-3xl bg-rice-50 shadow-card-shadow p-4 relative overflow-hidden mb-4">
      <div className="flex gap-2 items-center relative z-10">
        <img src={quote.logoURI} alt="token" className="h-6 w-6" />
        <p className="text-gray-700 h4-bold">{quote.symbol}</p>
        <Badge text="Stable Strategy" color="green" size="s" />
      </div>
      <div className="flex items-center justify-between gap-2 relative z-10 min-h-[22px]">
        <div className="flex items-center gap-2">
          <p className="text-gray-500 diatype-xs-medium">APY</p>
          <p className="text-gray-700 diatype-xs-bold">{apy}</p>
        </div>
        <div className="flex items-center gap-2">
          <p className="text-gray-500 diatype-xs-medium">24h </p>
          <p className="text-gray-700 diatype-xs-bold">{volume}</p>
        </div>
        <div className="flex items-center gap-2">
          <p className="text-gray-500 diatype-xs-medium">TVL</p>
          <p className="text-gray-700 diatype-xs-bold">{tvl}</p>
        </div>
      </div>
      <img
        src="/images/characters/hippo.svg"
        alt=""
        className="absolute right-[-2.8rem] top-[-0.5rem] opacity-10"
      />
    </div>
  );
};

export const SimpleSwapForm: React.FC = () => {
  const { settings } = useApp();
  const { account } = useAccount();
  const { register, setValue, inputs } = useInputs();
  const { coins, pairs, quote, base, isReverse, direction, toggleDirection, changeQuote } =
    useSimpleSwap();
  const { data: balances } = useBalances({ address: account?.address });
  const { getPrice } = usePrices();
  const { formatNumberOptions } = settings;

  const baseBalance = formatUnits(balances?.[base.denom] || 0, base.decimals);
  const quoteBalance = formatUnits(balances?.[quote.denom] || 0, quote.decimals);

  const baseAmount = inputs.base?.value || 0;
  const quoteAmount = inputs.selectedCoin?.value || 0;

  return (
    <div
      className={twMerge("flex flex-col items-center relative", {
        "flex-col-reverse": direction === "reverse",
      })}
    >
      <IconGear className="w-[18px] h-[18px] absolute right-0 top-0" />
      <Input
        placeholder="0"
        {...register("base", {
          strategy: "onChange",
          validate: (v) => {
            if (Number(v) > Number(baseBalance)) return m["validations.errors.insufficientFunds"]();
            return true;
          },
          mask: numberMask,
        })}
        label={isReverse ? "You get" : "You swap"}
        classNames={{
          base: "z-20",
          inputWrapper: "pl-0 py-3 flex-col h-auto gap-[6px]",
          inputParent: "h-[34px] h3-bold",
          input: "!h3-bold",
        }}
        startText="right"
        startContent={
          <div className="inline-flex flex-row items-center gap-3 diatype-m-regular h-[46px] rounded-md min-w-14 p-3 bg-transparent justify-start">
            <div className="flex gap-2 items-center font-semibold">
              <img src={base.logoURI} alt={base.symbol} className="w-8 h-8" />
              <p>{base.symbol}</p>
            </div>
          </div>
        }
        insideBottomComponent={
          <div className="flex items-center justify-between gap-2 w-full h-[22px] text-gray-500 diatype-sm-regular pl-4">
            <div className="flex items-center gap-2">
              <p>
                {baseBalance} {base.symbol}
              </p>
              <Button
                type="button"
                variant="secondary"
                size="xs"
                className="bg-red-bean-50 text-red-bean-500 hover:bg-red-bean-100 focus:[box-shadow:0px_0px_0px_3px_#F575893D] py-[2px] px-[6px]"
                onClick={() => setValue("base", baseBalance)}
              >
                Max
              </Button>
            </div>
            <p className="">
              {getPrice(baseAmount, base.denom, {
                format: true,
                formatOptions: formatNumberOptions,
              })}
            </p>
          </div>
        }
      />

      <button
        type="button"
        className="flex items-center justify-center border border-gray-300 rounded-full h-5 w-5 cursor-pointer mt-4"
        onClick={() => toggleDirection()}
      >
        <IconArrowDown className="h-3 w-3 text-gray-300" />
      </button>
      <Input
        placeholder="0"
        label={isReverse ? "You swap" : "You get"}
        {...register("quote", {
          strategy: "onChange",
          validate: (v) => {
            if (Number(v) > Number(quoteBalance))
              return m["validations.errors.insufficientFunds"]();
            return true;
          },
          mask: numberMask,
        })}
        classNames={{
          base: "z-20",
          inputWrapper: "pl-0 py-3 flex-col h-auto gap-[6px]",
          inputParent: "h-[34px] h3-bold",
          input: "!h3-bold",
        }}
        startText="right"
        startContent={
          <CoinSelector
            coins={Object.values(coins).filter((c) => {
              return Object.keys(pairs.data).some((d) => d.includes(c?.denom.split("/")[2]));
            })}
            value={quote.denom}
            onChange={(v) => changeQuote(coins[v].symbol)}
          />
        }
        insideBottomComponent={
          <div className="flex items-center justify-between gap-2 w-full h-[22px] text-gray-500 diatype-sm-regular pl-4">
            <div className="flex items-center gap-2">
              <p>
                {quoteBalance} {quote.symbol}
              </p>
              <Button
                type="button"
                variant="secondary"
                size="xs"
                className="bg-red-bean-50 text-red-bean-500 hover:bg-red-bean-100 focus:[box-shadow:0px_0px_0px_3px_#F575893D] py-[2px] px-[6px]"
                onClick={() => setValue("quote", quoteBalance)}
              >
                Max
              </Button>
            </div>
            <p className="">
              {" "}
              {getPrice(quoteAmount, quote.denom, {
                format: true,
                formatOptions: formatNumberOptions,
              })}
            </p>
          </div>
        }
      />
    </div>
  );
};

const SimpleSwapDetails: React.FC = () => {
  return (
    <div className="flex flex-col gap-1 w-full">
      <div className="flex w-full gap-2 items-center justify-between">
        <p className="text-gray-500 diatype-sm-regular">Fee ({0}%)</p>
        <p className="text-gray-700 diatype-sm-medium">0</p>
      </div>
      <div className="flex w-full gap-2 items-center justify-between">
        <p className="text-gray-500 diatype-sm-regular">Price Impact</p>
        <p className="text-gray-700 diatype-sm-medium">-0/05%</p>
      </div>
      <div className="flex w-full gap-2 items-center justify-between">
        <p className="text-gray-500 diatype-sm-regular">Max slippage</p>
        <p className="text-gray-700 diatype-sm-medium">0.50%</p>
      </div>
    </div>
  );
};

const SimpleSwapTrigger: React.FC = () => {
  return (
    <Button fullWidth size="md">
      Swap
    </Button>
  );
};

export const SimpleSwap = Object.assign(Root, {
  Header: SimpleSwapHeader,
  Form: SimpleSwapForm,
  Details: SimpleSwapDetails,
  Trigger: SimpleSwapTrigger,
});
