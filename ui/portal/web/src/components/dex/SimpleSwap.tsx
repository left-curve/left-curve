import { createContext, numberMask, twMerge, useInputs } from "@left-curve/applets-kit";
import {
  useSimpleSwap as state,
  useAccount,
  useBalances,
  useConfig,
  usePrices,
  useSigningClient,
} from "@left-curve/store";

import { Badge, Button, CoinSelector, IconArrowDown, Input } from "@left-curve/applets-kit";
import { toast } from "../foundation/Toast";

import { m } from "~/paraglide/messages";

import { formatNumber, formatUnits, parseUnits, withResolvers } from "@left-curve/dango/utils";
import { useMutation } from "@tanstack/react-query";
import { type PropsWithChildren, useEffect, useState } from "react";
import { useApp } from "~/hooks/useApp";
import { Modals } from "../modals/RootModal";

import type { Address } from "@left-curve/dango/types";
import type { UseSimpleSwapParameters } from "@left-curve/store";
import type { UseMutationResult } from "@tanstack/react-query";
import type React from "react";

const [SimpleSwapProvider, useSimpleSwap] = createContext<{
  state: ReturnType<typeof state>;
  submission: UseMutationResult<undefined, Error, void, unknown>;
  controllers: ReturnType<typeof useInputs>;
}>({
  name: "SimpleSwapContext",
});

const SimpleSwapContainer: React.FC<PropsWithChildren<UseSimpleSwapParameters>> = ({
  children,
  ...parameters
}) => {
  const simpleSwapState = state(parameters);
  const controllers = useInputs();
  const { notifier, settings, showModal } = useApp();
  const { account } = useAccount();
  const { data: signingClient } = useSigningClient();
  const { reset } = controllers;
  const { refetch: refreshBalances } = useBalances({ address: account?.address });
  const { pair, simulation, fee, coins } = simpleSwapState;
  const { formatNumberOptions } = settings;

  const submission = useMutation({
    mutationFn: async () => {
      if (!signingClient) throw new Error("error: no signing client");
      if (!pair) throw new Error("error: no pair");
      if (!simulation.input || !simulation.data) throw new Error("error: no simulation");
      notifier.publish("submit_tx", { isSubmitting: true });

      try {
        const { promise, resolve: confirmSwap, reject: rejectSwap } = withResolvers();
        showModal(Modals.ConfirmSwap, {
          input: {
            coin: coins[simulation.input.denom],
            amount: simulation.input.amount,
          },
          output: {
            coin: coins[simulation.data.denom],
            amount: simulation.data.amount,
          },
          fee: formatNumber(fee, { ...formatNumberOptions, currency: "usd" }),
          confirmSwap,
          rejectSwap,
        });

        const response = await promise
          .then(() => true)
          .catch(() => {
            notifier.publish("submit_tx", {
              isSubmitting: false,
              txResult: { hasSucceeded: false, message: m["dex.simpleSwap.errors.failure"]() },
            });
            return false;
          });
        if (!response) return undefined;

        await signingClient.swapExactAmountIn({
          sender: account!.address as Address,
          route: [{ baseDenom: pair.baseDenom, quoteDenom: pair.quoteDenom }],
          input: simulation.input,
        });

        reset();
        toast.success({ title: m["dex.simpleSwap.swapSuccessfully"]() });
        notifier.publish("submit_tx", {
          isSubmitting: false,
          txResult: { hasSucceeded: true, message: m["dex.simpleSwap.swapSuccessfully"]() },
        });
        refreshBalances();
      } catch (e) {
        console.error(e);
        notifier.publish("submit_tx", {
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
      }
    },
  });

  return (
    <SimpleSwapProvider value={{ state: simpleSwapState, controllers, submission }}>
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
  const { state, controllers, submission } = useSimpleSwap();
  const { data: balances } = useBalances({ address: account?.address });
  const [activeInput, setActiveInput] = useState<"base" | "quote">();
  const { getPrice } = usePrices();

  const { isReverse, direction, base, quote, pairs, changeQuote, toggleDirection } = state;
  const { register, setValue, inputs } = controllers;
  const { isPending } = submission;
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
      id="simple-swap-form"
      className={twMerge("flex flex-col items-center relative", {
        "flex-col-reverse": direction === "reverse",
      })}
      onSubmit={(e) => {
        e.preventDefault();
        submission.mutate();
      }}
    >
      <Input
        isDisabled={isPending}
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
        label={isReverse ? m["dex.simpleSwap.youGet"]() : m["dex.simpleSwap.youSwap"]()}
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
                onClick={() => {
                  setActiveInput("base");
                  setValue("base", baseBalance);
                }}
              >
                {m["common.max"]()}
              </Button>
            </div>
            <p>
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
        disabled={isPending}
        className="flex items-center justify-center border border-gray-300 rounded-full h-5 w-5 cursor-pointer mt-4"
        onClick={() => toggleDirection()}
      >
        <IconArrowDown className="h-3 w-3 text-gray-300" />
      </button>
      <Input
        isDisabled={isPending}
        placeholder="0"
        onFocus={() => setActiveInput("quote")}
        label={isReverse ? m["dex.simpleSwap.youSwap"]() : m["dex.simpleSwap.youGet"]()}
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
                onClick={() => {
                  setActiveInput("quote");
                  setValue("quote", quoteBalance);
                }}
              >
                {m["common.max"]()}
              </Button>
            </div>
            <p>
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
  const { account } = useAccount();
  const { settings } = useApp();
  const { state, controllers } = useSimpleSwap();
  const { pair, simulation, fee, coins } = state;
  const { formatNumberOptions } = settings;
  const { input, data } = simulation;

  if (!input || !data || !account || input.amount === "0") return <div />;

  const inputCoin = coins[input.denom];
  const outputCoin = coins[data.denom];

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
        <p className="text-gray-500 diatype-sm-regular">{m["dex.simpleSwap.rate"]()}</p>
        <p className="text-gray-700 diatype-sm-medium">
          1 {inputCoin.symbol} ≈{" "}
          {formatNumber(
            formatUnits(
              Math.round(Number(data.amount) / Number(controllers.inputs.quote.value || 0)),
              outputCoin.decimals,
            ) || 0,
            formatNumberOptions,
          )}{" "}
          {outputCoin.symbol}
        </p>
      </div>
    </div>
  );
};

const SimpleSwapTrigger: React.FC = () => {
  return (
    <Button fullWidth size="md" type="submit" form="simple-swap-form">
      Swap
    </Button>
  );
};

export const SimpleSwap = Object.assign(SimpleSwapContainer, {
  Header: SimpleSwapHeader,
  Form: SimpleSwapForm,
  Details: SimpleSwapDetails,
  Trigger: SimpleSwapTrigger,
});
