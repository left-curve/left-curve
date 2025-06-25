import {
  useAccount,
  useBalances,
  useConfig,
  usePrices,
  useSigningClient,
  useSimpleSwapState,
  useSubmitTx,
} from "@left-curve/store";
import { useQueryClient } from "@tanstack/react-query";
import { useApp } from "~/hooks/useApp";

import {
  Badge,
  Button,
  CoinSelector,
  IconArrowDown,
  Input,
  Skeleton,
} from "@left-curve/applets-kit";
import { Link } from "@tanstack/react-router";
import { Modals } from "../modals/RootModal";

import { m } from "~/paraglide/messages";

import { createContext, numberMask, twMerge, useInputs } from "@left-curve/applets-kit";
import { formatNumber, formatUnits, parseUnits, withResolvers } from "@left-curve/dango/utils";

import { type PropsWithChildren, useEffect, useState } from "react";

import type { Address } from "@left-curve/dango/types";
import type { UseSimpleSwapStateParameters, UseSubmitTxReturnType } from "@left-curve/store";
import type React from "react";

const [SimpleSwapProvider, useSimpleSwap] = createContext<{
  state: ReturnType<typeof useSimpleSwapState>;
  submission: UseSubmitTxReturnType<void, Error, void, unknown>;
  controllers: ReturnType<typeof useInputs>;
}>({
  name: "SimpleSwapContext",
});

const SimpleSwapContainer: React.FC<PropsWithChildren<UseSimpleSwapStateParameters>> = ({
  children,
  ...parameters
}) => {
  const state = useSimpleSwapState(parameters);
  const controllers = useInputs();
  const { toast, settings, showModal } = useApp();
  const { account } = useAccount();
  const { data: signingClient } = useSigningClient();
  const queryClient = useQueryClient();
  const { refetch: refreshBalances } = useBalances({ address: account?.address });
  const { pair, simulation, fee, coins } = state;
  const { formatNumberOptions } = settings;

  const submission = useSubmitTx({
    toast: {
      success: () => toast.success({ title: m["dex.convert.convertSuccessfully"]() }),
      error: () =>
        toast.error(
          { title: m["dex.convert.errors.failure"]() },
          { duration: Number.POSITIVE_INFINITY },
        ),
    },
    submission: {
      success: m["dex.convert.convertSuccessfully"](),
      error: m["dex.convert.errors.failure"](),
    },
    mutation: {
      mutationFn: async (_, { abort }) => {
        if (!signingClient) throw new Error("error: no signing client");
        if (!pair) throw new Error("error: no pair");
        if (!simulation.data) throw new Error("error: no simulation data");

        const { input, output } = simulation.data;

        const { promise, resolve: confirmSwap, reject: rejectSwap } = withResolvers();

        showModal(Modals.ConfirmSwap, {
          input: {
            coin: coins[input.denom],
            amount: input.amount,
          },
          output: {
            coin: coins[output.denom],
            amount: output.amount,
          },
          fee: formatNumber(fee, { ...formatNumberOptions, currency: "usd" }),
          confirmSwap,
          rejectSwap,
        });

        await promise.catch(abort);

        await signingClient.swapExactAmountIn({
          sender: account!.address as Address,
          route: [{ baseDenom: pair.baseDenom, quoteDenom: pair.quoteDenom }],
          input: {
            denom: input.denom,
            amount: input.amount,
          },
        });
      },
      onSuccess: () => {
        controllers.reset();
        simulation.reset();
        refreshBalances();
        queryClient.invalidateQueries({ queryKey: ["quests", account?.username] });
      },
    },
  });

  return (
    <SimpleSwapProvider value={{ state, controllers, submission }}>{children}</SimpleSwapProvider>
  );
};

const SimpleSwapHeader: React.FC = () => {
  const { state } = useSimpleSwap();
  const { quote, statistics } = state;
  const { tvl, apy, volume } = statistics.data;
  return (
    <div className="flex flex-col gap-3 rounded-3xl bg-surface-tertiary-rice shadow-account-card p-4 relative overflow-hidden mb-4">
      <div className="flex gap-2 items-center relative z-10">
        <img src={quote.logoURI} alt="token" className="h-6 w-6" />
        <p className="text-secondary-700 h4-bold">{quote.symbol}</p>
        <Badge text="Stable Strategy" color="green" size="s" />
      </div>
      <div className="flex items-center justify-between gap-2 relative z-10 min-h-[22px]">
        <div className="flex items-center gap-2">
          <p className="text-tertiary-500 diatype-xs-medium">{m["dex.apy"]()}</p>
          <p className="text-secondary-700 diatype-xs-bold">{apy}</p>
        </div>
        <div className="flex items-center gap-2">
          <p className="text-tertiary-500 diatype-xs-medium">{m["dex.24h"]()}</p>
          <p className="text-secondary-700 diatype-xs-bold">{volume}</p>
        </div>
        <div className="flex items-center gap-2">
          <p className="text-tertiary-500 diatype-xs-medium">{m["dex.tvl"]()}</p>
          <p className="text-secondary-700 diatype-xs-bold">{tvl}</p>
        </div>
      </div>
      <img
        src="/images/characters/hippo.svg"
        alt=""
        className="absolute right-[-2.8rem] top-[-0.5rem] opacity-10 select-none drag-none"
      />
    </div>
  );
};

const SimpleSwapForm: React.FC = () => {
  const { settings } = useApp();
  const { coins } = useConfig();
  const { account, isConnected } = useAccount();
  const { state, controllers, submission } = useSimpleSwap();
  const { data: balances } = useBalances({ address: account?.address });
  const [activeInput, setActiveInput] = useState<"base" | "quote">();
  const { getPrice } = usePrices();

  const { isReverse, direction, base, quote, pair, pairs, changeQuote, toggleDirection } = state;
  const { register, setValue, revalidate, inputs } = controllers;
  const { isPending } = submission;
  const { formatNumberOptions } = settings;
  const { simulation } = state;

  const baseBalance = formatUnits(balances?.[base.denom] || 0, base.decimals);
  const quoteBalance = formatUnits(balances?.[quote.denom] || 0, quote.decimals);

  const baseAmount = inputs.base?.value || "0";
  const quoteAmount = inputs.quote?.value || "0";

  const coinPairs = Object.values(coins).filter((c) => Object.keys(pairs.data).includes(c.denom));

  useEffect(() => {
    if ((baseAmount === "0" && quoteAmount === "0") || !activeInput || !pair) return;
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

      const { output } = await simulation.simulate({
        pair,
        input: {
          amount: parseUnits(request.amount, request.input.decimals).toString(),
          denom: request.input.denom,
        },
      });

      if (output) {
        setValue(request.target, formatUnits(output.amount, request.output.decimals));
      }
      revalidate();
    })();
  }, [baseAmount, quoteAmount, pair]);

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
        isLoading={activeInput !== "base" ? simulation.isPending : false}
        onFocus={() => setActiveInput("base")}
        {...register("base", {
          strategy: "onChange",
          validate: (v) => {
            if (!isConnected || isReverse) return true;
            if (Number(v) > Number(baseBalance)) return m["errors.validations.insufficientFunds"]();
            return true;
          },
          mask: numberMask,
        })}
        label={isReverse ? m["dex.convert.youGet"]() : m["dex.convert.youSwap"]()}
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
          <div className="flex items-center justify-between gap-2 w-full h-[22px] text-tertiary-500 diatype-sm-regular pl-4">
            <div className="flex items-center gap-2">
              <p>
                {baseBalance} {base.symbol}
              </p>
              {isReverse ? null : (
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
              )}
            </div>
            <div>
              {simulation.isPending && activeInput !== "base" ? (
                <Skeleton className="w-14 h-4" />
              ) : (
                getPrice(baseAmount, base.denom, {
                  format: true,
                  formatOptions: formatNumberOptions,
                })
              )}
            </div>
          </div>
        }
      />

      <button
        type="button"
        disabled={isPending}
        className="flex items-center justify-center border border-gray-300 rounded-full h-5 w-5 cursor-pointer mt-4"
        onClick={() => {
          toggleDirection();
          setActiveInput(activeInput === "base" ? "quote" : "base");
        }}
      >
        <IconArrowDown className="h-3 w-3 text-gray-300" />
      </button>
      <Input
        isDisabled={isPending}
        placeholder="0"
        isLoading={activeInput !== "quote" ? simulation.isPending : false}
        onFocus={() => setActiveInput("quote")}
        label={isReverse ? m["dex.convert.youSwap"]() : m["dex.convert.youGet"]()}
        {...register("quote", {
          strategy: "onChange",
          validate: (v) => {
            if (!isConnected || !isReverse) return true;
            if (Number(v) > Number(quoteBalance))
              return m["errors.validations.insufficientFunds"]();
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
          coinPairs.length ? (
            <CoinSelector
              coins={Object.values(coins).filter(
                (c) => Object.keys(pairs.data).includes(c.denom) && c.denom !== "dango",
              )}
              value={quote.denom}
              onChange={(v) => changeQuote(coins[v].symbol)}
            />
          ) : (
            <Skeleton className="w-36 h-11" />
          )
        }
        insideBottomComponent={
          <div className="flex items-center justify-between gap-2 w-full h-[22px] text-tertiary-500 diatype-sm-regular pl-4">
            <div className="flex items-center gap-2">
              <p>
                {quoteBalance} {quote.symbol}
              </p>
              {isReverse ? (
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
              ) : null}
            </div>
            <div>
              {simulation.isPending && activeInput !== "quote" ? (
                <Skeleton className="w-14 h-4" />
              ) : (
                getPrice(quoteAmount, quote.denom, {
                  format: true,
                  formatOptions: formatNumberOptions,
                })
              )}
            </div>
          </div>
        }
      />
    </form>
  );
};

const SimpleSwapDetails: React.FC = () => {
  const { isConnected } = useAccount();
  const { settings } = useApp();
  const { state } = useSimpleSwap();
  const { pair, simulation, fee, coins } = state;
  const { formatNumberOptions } = settings;
  const { data, isPending } = simulation;

  if (!data || !isConnected || data.input.denom === "0") return <div />;

  const { input, output } = data;

  const inputCoin = coins[input.denom];
  const outputCoin = coins[output.denom];

  const inputAmount = formatUnits(input.amount, inputCoin.decimals);

  const outputAmount = formatUnits(output.amount, outputCoin.decimals);

  return (
    <div className="flex flex-col gap-1 w-full">
      <div className="flex w-full gap-2 items-center justify-between">
        <p className="text-tertiary-500 diatype-sm-regular">
          {m["dex.fee"]()} ({Number(pair?.params.swapFeeRate || 0) * 100}%)
        </p>
        {isPending ? (
          <Skeleton className="w-14 h-4" />
        ) : (
          <p className="text-secondary-700 diatype-sm-medium">
            {formatNumber(fee, { ...formatNumberOptions, currency: "usd" })}
          </p>
        )}
      </div>
      <div className="flex w-full gap-2 items-center justify-between">
        <p className="text-tertiary-500 diatype-sm-regular">{m["dex.convert.rate"]()}</p>
        {isPending ? (
          <Skeleton className="w-36 h-4" />
        ) : (
          <p className="text-secondary-700 diatype-sm-medium">
            1 {inputCoin.symbol} ≈{" "}
            {formatNumber(Number(outputAmount) / Number(inputAmount), {
              ...formatNumberOptions,
              maxFractionDigits: outputCoin.decimals,
            })}{" "}
            {outputCoin.symbol}
          </p>
        )}
      </div>
    </div>
  );
};

const SimpleSwapTrigger: React.FC = () => {
  const { isConnected } = useAccount();
  const { submission, state, controllers } = useSimpleSwap();
  const { simulation } = state;
  const { isValid } = controllers;

  return isConnected ? (
    <Button
      fullWidth
      size="md"
      type="submit"
      form="simple-swap-form"
      isDisabled={
        Number(simulation.data?.output.amount || 0) <= 0 || simulation.isPending || !isValid
      }
      isLoading={submission.isPending}
    >
      {m["dex.convert.swap"]()}
    </Button>
  ) : (
    <Button fullWidth size="md" as={Link} to="/signin">
      {m["common.signin"]()}
    </Button>
  );
};

export const SimpleSwap = Object.assign(SimpleSwapContainer, {
  Header: SimpleSwapHeader,
  Form: SimpleSwapForm,
  Details: SimpleSwapDetails,
  Trigger: SimpleSwapTrigger,
});
