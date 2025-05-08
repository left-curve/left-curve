import { createContext, numberMask, twMerge, useInputs } from "@left-curve/applets-kit";
import {
  useSimpleSwap as state,
  useAccount,
  useBalances,
  useConfig,
  usePrices,
  useSigningClient,
} from "@left-curve/store";

import {
  Badge,
  Button,
  CoinSelector,
  IconArrowDown,
  IconGear,
  Input,
} from "@left-curve/applets-kit";
import { toast } from "../foundation/Toast";

import { m } from "~/paraglide/messages";

import { formatNumber, formatUnits, parseUnits, withResolvers } from "@left-curve/dango/utils";
import { useMutation } from "@tanstack/react-query";
import { type PropsWithChildren, useEffect, useState } from "react";
import { useApp } from "~/hooks/useApp";

import type { Address } from "@left-curve/dango/types";
import type { UseSimpleSwapParameters } from "@left-curve/store";
import type React from "react";

const [SimpleSwapProvider, useSimpleSwap] = createContext<{
  state: ReturnType<typeof state>;
  controllers: ReturnType<typeof useInputs>;
  isLoading: boolean;
  setIsLoading: (b: boolean) => void;
}>({
  name: "SimpleSwapContext",
});

const Root: React.FC<PropsWithChildren<UseSimpleSwapParameters>> = ({
  children,
  ...parameters
}) => {
  const [isLoading, setIsLoading] = useState(false);
  return (
    <SimpleSwapProvider
      value={{ state: state(parameters), controllers: useInputs(), isLoading, setIsLoading }}
    >
      {children}
    </SimpleSwapProvider>
  );
};

const SimpleSwapHeader: React.FC = () => {
  const { state } = useSimpleSwap();
  const { quote, statistics } = state;
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
          <p className="text-gray-500 diatype-xs-medium">{m["dex.apy"]()}</p>
          <p className="text-gray-700 diatype-xs-bold">{apy}</p>
        </div>
        <div className="flex items-center gap-2">
          <p className="text-gray-500 diatype-xs-medium">{m["dex.24h"]()}</p>
          <p className="text-gray-700 diatype-xs-bold">{volume}</p>
        </div>
        <div className="flex items-center gap-2">
          <p className="text-gray-500 diatype-xs-medium">{m["dex.tvl"]()}</p>
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
  const { coins } = useConfig();
  const { account } = useAccount();
  const { state, controllers, isLoading } = useSimpleSwap();
  const { data: balances } = useBalances({ address: account?.address });
  const [activeInput, setActiveInput] = useState<"base" | "quote">();
  const { getPrice } = usePrices();

  const { isReverse, direction, base, quote, pairs, changeQuote, toggleDirection } = state;
  const { register, setValue, inputs } = controllers;
  const { formatNumberOptions } = settings;
  const { simulate } = state.simulation;

  const baseBalance = formatUnits(balances?.[base.denom] || 0, base.decimals);
  const quoteBalance = formatUnits(balances?.[quote.denom] || 0, quote.decimals);

  const baseAmount = inputs.base?.value || "0";
  const quoteAmount = inputs.quote?.value || "0";

  useEffect(() => {
    if ((baseAmount === "0" && quoteAmount === "0") || !activeInput) return;
    (async () => {
      const request =
        activeInput === "base"
          ? {
              amount: baseAmount,
              input: base,
              target: "quote",
              output: quote,
            }
          : {
              amount: quoteAmount,
              input: quote,
              target: "base",
              output: base,
            };

      const output = await simulate({
        amount: parseUnits(request.amount, request.input.decimals).toString(),
        denom: request.input.denom,
      });

      if (output) {
        setValue(request.target, formatUnits(output.amount, request.output.decimals));
      }
    })();
  }, [baseAmount, quoteAmount, quote, base]);

  return (
    <form
      className={twMerge("flex flex-col items-center relative", {
        "flex-col-reverse": direction === "reverse",
      })}
    >
      <IconGear className="w-[18px] h-[18px] absolute right-0 top-0" />
      <Input
        isDisabled={isLoading}
        placeholder="0"
        onFocus={() => setActiveInput("base")}
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
                {m["common.max"]()}
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
        disabled={isLoading}
        className="flex items-center justify-center border border-gray-300 rounded-full h-5 w-5 cursor-pointer mt-4"
        onClick={() => toggleDirection()}
      >
        <IconArrowDown className="h-3 w-3 text-gray-300" />
      </button>
      <Input
        isDisabled={isLoading}
        placeholder="0"
        onFocus={() => setActiveInput("quote")}
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
            coins={Object.values(coins).filter((c) => Object.keys(pairs.data).includes(c.denom))}
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
                {m["common.max"]()}
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
    </form>
  );
};

const SimpleSwapDetails: React.FC = () => {
  const { settings } = useApp();
  const { state } = useSimpleSwap();
  const { pair, priceImpact, fee, slippage } = state;
  const { formatNumberOptions } = settings;

  return (
    <div className="flex flex-col gap-1 w-full">
      <div className="flex w-full gap-2 items-center justify-between">
        <p className="text-gray-500 diatype-sm-regular">
          {m["dex.fee"]()} ({pair?.params.swapFeeRate || 0}%)
        </p>
        <p className="text-gray-700 diatype-sm-medium">
          {formatNumber(fee, { ...formatNumberOptions, currency: "usd" })}
        </p>
      </div>
      <div className="flex w-full gap-2 items-center justify-between">
        <p className="text-gray-500 diatype-sm-regular">{m["dex.simpleSwap.priceImpact"]()}</p>
        <p className="text-gray-700 diatype-sm-medium">{priceImpact.toFixed(5)}%</p>
      </div>
      <div className="flex w-full gap-2 items-center justify-between">
        <p className="text-gray-500 diatype-sm-regular">{m["dex.simpleSwap.slippage"]()}</p>
        <p className="text-gray-700 diatype-sm-medium">{Number(slippage) * 100}%</p>
      </div>
    </div>
  );
};

const SimpleSwapTrigger: React.FC = () => {
  const { eventBus, showModal } = useApp();
  const { account } = useAccount();
  const { data: signingClient } = useSigningClient();
  const { state, controllers, isLoading, setIsLoading } = useSimpleSwap();
  const { reset } = controllers;
  const { refetch: refreshBalances } = useBalances({ address: account?.address });
  const { pair, simulation } = state;

  const { mutateAsync: swap } = useMutation({
    mutationFn: async () => {
      if (!signingClient) throw new Error("error: no signing client");
      if (!pair) throw new Error("error: no pair");
      if (!simulation.input) throw new Error("error: no simulation input");
      eventBus.publish("submit_tx", { isSubmitting: true });
      try {
        setIsLoading(true);

        /*
        const { promise, resolve: confirmSend, reject: rejectSend } = withResolvers();

        const response = await promise
          .then(() => true)
          .catch(() => {
            eventBus.publish("submit_tx", {
              isSubmitting: false,
              txResult: { hasSucceeded: false, message: m["transfer.error.description"]() },
            });
            return false;
          });

        if (!response) return undefined;
        */

        await signingClient.swapExactAmountIn({
          sender: account!.address as Address,
          route: [{ baseDenom: pair.baseDenom, quoteDenom: pair.quoteDenom }],
          input: simulation.input,
        });

        reset();
        toast.success({ title: m["dex.simpleSwap.swapSuccessfully"]() });
        eventBus.publish("submit_tx", {
          isSubmitting: false,
          txResult: { hasSucceeded: true, message: m["dex.simpleSwap.swapSuccessfully"]() },
        });
        refreshBalances();
      } catch (e) {
        console.error(e);
        eventBus.publish("submit_tx", {
          isSubmitting: false,
          txResult: { hasSucceeded: false, message: m["dex.simpleSwap.errors.failure"]() },
        });
        toast.error(
          {
            title: m["dex.simpleSwap.errors.failure"](),
          },
          {
            duration: Number.POSITIVE_INFINITY,
          },
        );
      } finally {
        setIsLoading(false);
      }
    },
  });

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Enter") {
        swap();
      }
    };
    addEventListener("keydown", handleKeyDown);
    return () => {
      removeEventListener("keydown", handleKeyDown);
    };
  }, []);

  return (
    <Button fullWidth size="md" isLoading={isLoading} onClick={() => swap()}>
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
