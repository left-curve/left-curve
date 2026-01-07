import { useAccount, useBalances, useConvertState } from "@left-curve/store";
import { useState } from "react";

import {
  AssetInputWithRange,
  Badge,
  Button,
  IconArrowDown,
  Modals,
  Skeleton,
  useApp,
  useDebounceFn,
} from "@left-curve/applets-kit";
import HippoSvg from "@left-curve/foundation/images/characters/hippo.svg";

import { createContext, useInputs } from "@left-curve/applets-kit";
import { Decimal, formatNumber, formatUnits, withResolvers } from "@left-curve/dango/utils";
import { m } from "@left-curve/foundation/paraglide/messages.js";

import type { PropsWithChildren } from "react";
import type React from "react";

const [ConvertProvider, useConvert] = createContext<{
  state: ReturnType<typeof useConvertState>;
  controllers: ReturnType<typeof useInputs>;
}>({
  name: "ConvertContext",
});

type ConvertProps = {
  pair: { from: string; to: string };
  onChangePair: (pair: { from: string; to: string }) => void;
};

const ConvertContainer: React.FC<PropsWithChildren<ConvertProps>> = ({
  children,
  ...parameters
}) => {
  const { toast, settings, showModal } = useApp();
  const { formatNumberOptions } = settings;
  const controllers = useInputs();

  const state = useConvertState({
    ...parameters,
    controllers,
    submission: {
      confirm: async () => {
        const { coins, fee } = state;
        const { input, output } = state.simulation.data!;
        const { promise, resolve: confirmSwap, reject: rejectSwap } = withResolvers();

        showModal(Modals.ConfirmSwap, {
          input: {
            coin: coins.byDenom[input.denom],
            amount: input.amount,
          },
          output: {
            coin: coins.byDenom[output.denom],
            amount: output.amount,
          },
          fee: formatNumber(fee, { ...formatNumberOptions, currency: "usd" }),
          confirmSwap,
          rejectSwap,
        });
        await promise;
      },
      onError: (_) => {
        toast.error({
          title: m["common.error"](),
          description: m["dex.convert.errors.failure"](),
        });
      },
    },
    simulation: {
      onError: (_) => {
        toast.error({
          title: m["common.error"](),
          description: m["dex.convert.errors.simulationFailed"](),
        });
      },
    },
  });

  return <ConvertProvider value={{ state, controllers }}>{children}</ConvertProvider>;
};

const ConvertHeader: React.FC = () => {
  const { state } = useConvert();
  const { pairId, statistics } = state;
  const { tvl, apy, volume } = statistics.data;

  const { base } = pairId;
  return (
    <div className="flex flex-col gap-3 rounded-xl bg-surface-tertiary-rice shadow-account-card p-4 relative overflow-hidden mb-4">
      <div className="flex gap-2 items-center relative z-10">
        <img src={base.logoURI} alt="token" className="h-6 w-6" />
        <p className="text-ink-secondary-700 h4-bold">{base.symbol}</p>
      </div>
      <div className="flex items-center justify-between gap-2 relative z-10 min-h-[22px]">
        <div className="flex items-center gap-2">
          <p className="text-ink-tertiary-500 diatype-xs-medium">{m["dex.apy"]()}</p>
          <p className="text-ink-secondary-700 diatype-xs-bold">{apy}</p>
        </div>
        <div className="flex items-center gap-2">
          <p className="text-ink-tertiary-500 diatype-xs-medium">{m["dex.24h"]()}</p>
          <p className="text-ink-secondary-700 diatype-xs-bold">{volume}</p>
        </div>
        <div className="flex items-center gap-2">
          <p className="text-ink-tertiary-500 diatype-xs-medium">{m["dex.tvl"]()}</p>
          <p className="text-ink-secondary-700 diatype-xs-bold">{tvl}</p>
        </div>
      </div>
      <img
        src={HippoSvg}
        alt=""
        className="absolute right-[-2.8rem] top-[-0.5rem] opacity-10 select-none drag-none"
      />
    </div>
  );
};

const ConvertForm: React.FC = () => {
  const { account } = useAccount();
  const { state, controllers } = useConvert();
  const { revalidate } = controllers;
  const [activeInput, setActiveInput] = useState<"from" | "to">("from");

  const { isReverse, fromCoin, toCoin, changePair, toggleDirection, submission } = state;

  const { simulation } = state;

  const { data: balances = {} } = useBalances({ address: account?.address });

  const simulate = useDebounceFn(simulation.mutateAsync, 300);

  return (
    <form
      id="convert-form"
      className="flex flex-col items-center relative"
      onSubmit={(e) => {
        e.preventDefault();
        submission.mutate();
      }}
    >
      <AssetInputWithRange
        name="from"
        label={m["dex.convert.youSwap"]()}
        asset={fromCoin}
        balances={balances}
        controllers={controllers}
        isDisabled={submission.isPending}
        isLoading={activeInput !== "from" ? simulation.isPending : false}
        onFocus={() => setActiveInput("from")}
        shouldValidate
        showRange
        showCoinSelector={isReverse}
        onSelectCoin={changePair}
        triggerSimulation={async (reverse) => {
          await simulate(reverse ? "to" : "from");
          revalidate();
        }}
      />
      <button
        type="button"
        disabled={submission.isPending}
        className="flex items-center justify-center border border-primitives-gray-light-300 rounded-full h-5 w-5 cursor-pointer mt-4"
        onClick={async () => {
          toggleDirection();
          await simulate(activeInput);
          revalidate();
        }}
      >
        <IconArrowDown className="h-3 w-3 text-primitives-gray-light-300" />
      </button>
      <AssetInputWithRange
        name="to"
        label={m["dex.convert.youGet"]()}
        asset={toCoin}
        balances={balances}
        controllers={controllers}
        isDisabled={submission.isPending}
        isLoading={activeInput !== "to" ? simulation.isPending : false}
        onFocus={() => setActiveInput("to")}
        showCoinSelector={!isReverse}
        onSelectCoin={changePair}
        triggerSimulation={async (reverse) => {
          await simulate(reverse ? "from" : "to");
          revalidate();
        }}
      />
    </form>
  );
};

const ConvertDetails: React.FC = () => {
  const { isConnected } = useAccount();
  const { state } = useConvert();
  const { settings } = useApp();
  const { pair, simulation, fee, coins } = state;
  const { formatNumberOptions } = settings;
  const { data, isPending } = simulation;

  if (!data || !isConnected || data.input.denom === "0") return <div />;

  const { input, output } = data;

  const inputCoin = coins.byDenom[input.denom];
  const outputCoin = coins.byDenom[output.denom];

  const inputAmount = formatUnits(input.amount, inputCoin.decimals);

  const outputAmount = formatUnits(output.amount, outputCoin.decimals);

  return (
    <div className="flex flex-col gap-1 w-full">
      <div className="flex w-full gap-2 items-center justify-between">
        <p className="text-ink-tertiary-500 diatype-sm-regular">
          {m["dex.fee"]()} ({Number(pair?.params.swapFeeRate || 0) * 100}%)
        </p>
        {isPending ? (
          <Skeleton className="w-14 h-4" />
        ) : (
          <p className="text-ink-secondary-700 diatype-sm-medium">
            {formatNumber(fee, { ...formatNumberOptions, currency: "usd" })}
          </p>
        )}
      </div>
      <div className="flex w-full gap-2 items-center justify-between">
        <p className="text-ink-tertiary-500 diatype-sm-regular">{m["dex.convert.rate"]()}</p>
        {isPending ? (
          <Skeleton className="w-36 h-4" />
        ) : (
          <p className="text-ink-secondary-700 diatype-sm-medium">
            1 {inputCoin.symbol} ≈{" "}
            {formatNumber(Decimal(outputAmount).div(inputAmount).toFixed(), {
              ...formatNumberOptions,
              maximumTotalDigits: 10,
            })}{" "}
            {outputCoin.symbol}
          </p>
        )}
      </div>
    </div>
  );
};

const ConvertTrigger: React.FC = () => {
  const { isConnected } = useAccount();
  const { state, controllers } = useConvert();
  const { simulation, submission } = state;
  const { isValid } = controllers;
  const { showModal } = useApp();

  return isConnected ? (
    <Button
      fullWidth
      size="md"
      type="submit"
      form="convert-form"
      isDisabled={
        Number(simulation.data?.output.amount || 0) <= 0 || simulation.isPending || !isValid
      }
      isLoading={submission.isPending}
    >
      {m["dex.convert.swap"]()}
    </Button>
  ) : (
    <Button
      fullWidth
      size="md"
      onClick={() => showModal(Modals.Authenticate, { action: "signin" })}
    >
      {m["common.signin"]()}
    </Button>
  );
};

export const Convert = Object.assign(ConvertContainer, {
  Header: ConvertHeader,
  Form: ConvertForm,
  Details: ConvertDetails,
  Trigger: ConvertTrigger,
});
