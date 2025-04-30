import {
  Badge,
  Button,
  CoinSelector,
  IconArrowDown,
  IconGear,
  Input,
  twMerge,
  useInputs,
  useWatchEffect,
} from "@left-curve/applets-kit";
import { formatUnits } from "@left-curve/dango/utils";
import { useAccount, useBalances, useConfig, usePrices } from "@left-curve/store";
import { useQuery } from "@tanstack/react-query";
import { createLazyFileRoute, useNavigate } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { useApp } from "~/hooks/useApp";

import { m } from "~/paraglide/messages";

export const Route = createLazyFileRoute("/(app)/_app/swap")({
  component: SwapApplet,
});

function SwapApplet() {
  const navigate = useNavigate();
  const { register, inputs, setValue } = useInputs();
  const { from, to } = Route.useSearch();
  const { account } = useAccount();
  const { coins } = useConfig();
  const { settings, config } = useApp();
  const { formatNumberOptions } = settings;
  const { data: appConfig = { pairs: {} } } = config;
  const { pairs } = appConfig;

  const [direction, setDirection] = useState<"reverse" | "normal">(
    from === "USDC" ? "normal" : "reverse",
  );

  const isReverse = direction === "reverse";

  const [selectedDenom, setSelectedDenom] = useState<string>("hyp/all/eth");

  const { data: balances = {} } = useBalances({ address: account?.address });
  const { data: pairStatistics } = useQuery({
    queryKey: ["pair_statistics"],
    queryFn: () => {
      return { tvl: 0, apy: 0, volume: 0 };
    },
  });

  const { tvl, apy, volume } = pairStatistics || { tvl: 0, apy: 0, volume: 0 };

  const usdc = coins["hyp/eth/usdc"];
  const selectedCoin = coins[selectedDenom];
  const selectedCoinAmount = formatUnits(balances[selectedDenom] || 0, selectedCoin.decimals);
  const usdcAmount = formatUnits(balances[usdc.denom] || 0, usdc.decimals);
  const { getPrice } = usePrices();

  useEffect(() => {
    const { symbol } = selectedCoin;
    navigate({
      to: ".",
      search: isReverse ? { from: symbol, to: "USDC" } : { from: "USDC", to: symbol },
      replace: false,
    });
  }, [selectedCoin, direction]);

  return (
    <div className="w-full md:max-w-[25rem] mx-auto flex flex-col p-4 pt-6 gap-4 min-h-[100svh] md:min-h-fit">
      <div className="flex flex-col gap-3 rounded-3xl bg-rice-50 shadow-card-shadow p-4 relative overflow-hidden mb-4">
        <div className="flex gap-2 items-center relative z-10">
          <img src={selectedCoin.logoURI} alt="token" className="h-6 w-6" />
          <p className="text-gray-700 h4-bold">{selectedCoin.symbol}</p>
          <Badge text="Stable Strategy" color="green" size="s" />
        </div>
        <div className="flex items-center justify-between gap-2 relative z-10 min-h-[22px]">
          <div className="flex items-center gap-2">
            <p className="text-gray-500 diatype-xs-medium">APY</p>
            <p className="text-gray-700 diatype-xs-bold">{apy}%</p>
          </div>
          <div className="flex items-center gap-2">
            <p className="text-gray-500 diatype-xs-medium">24h </p>
            <p className="text-gray-700 diatype-xs-bold">{volume}%</p>
          </div>
          <div className="flex items-center gap-2">
            <p className="text-gray-500 diatype-xs-medium">TVL</p>
            <p className="text-gray-700 diatype-xs-bold">${tvl}</p>
          </div>
        </div>
        <img
          src="/images/characters/hippo.svg"
          alt=""
          className="absolute right-[-2.8rem] top-[-0.5rem] opacity-10"
        />
      </div>
      <div
        className={twMerge("flex flex-col items-center relative", {
          "flex-col-reverse": direction === "reverse",
        })}
      >
        <IconGear className="w-[18px] h-[18px] absolute right-0 top-0" />
        <Input
          placeholder="0"
          {...register(`${usdc.denom}.amount`, {
            strategy: "onChange",
            validate: (v) => {
              if (Number(v) > Number(usdcAmount))
                return m["validations.errors.insufficientFunds"]();
              return true;
            },
            mask: (v, prev) => {
              const regex = /^\d+(\.\d{0,18})?$/;
              if (v === "" || regex.test(v)) return v;
              return prev;
            },
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
                <img src={usdc.logoURI} alt={usdc.symbol} className="w-8 h-8" />
                <p>{usdc.symbol}</p>
              </div>
            </div>
          }
          insideBottomComponent={
            <div className="flex items-center justify-between gap-2 w-full h-[22px] text-gray-500 diatype-sm-regular pl-4">
              <div className="flex items-center gap-2">
                <p>
                  {usdcAmount} {usdc.symbol}
                </p>
                <Button
                  type="button"
                  variant="secondary"
                  size="xs"
                  className="bg-red-bean-50 text-red-bean-500 hover:bg-red-bean-100 focus:[box-shadow:0px_0px_0px_3px_#F575893D] py-[2px] px-[6px]"
                  onClick={() => setValue(`${usdc.denom}.amount`, usdcAmount)}
                >
                  Max
                </Button>
              </div>
              <p className="">
                {getPrice(inputs[`${usdc.denom}.amount`]?.value || 0, usdc.denom, {
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
          onClick={() => setDirection(direction === "normal" ? "reverse" : "normal")}
        >
          <IconArrowDown className="h-3 w-3 text-gray-300" />
        </button>
        <Input
          placeholder="0"
          label={isReverse ? "You swap" : "You get"}
          {...register("selectedCoin", {
            strategy: "onChange",
            validate: (v) => {
              if (Number(v) > Number(selectedCoinAmount))
                return m["validations.errors.insufficientFunds"]();
              return true;
            },
            mask: (v, prev) => {
              const regex = /^\d+(\.\d{0,18})?$/;
              if (v === "" || regex.test(v)) return v;
              return prev;
            },
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
                return Object.keys(pairs).some((d) => d.includes(c?.denom.split("/")[2]));
              })}
              value={selectedDenom}
              onChange={(v) => {
                setSelectedDenom(v);
              }}
            />
          }
          insideBottomComponent={
            <div className="flex items-center justify-between gap-2 w-full h-[22px] text-gray-500 diatype-sm-regular pl-4">
              <div className="flex items-center gap-2">
                <p>
                  {selectedCoinAmount} {selectedCoin.symbol}
                </p>
                <Button
                  type="button"
                  variant="secondary"
                  size="xs"
                  className="bg-red-bean-50 text-red-bean-500 hover:bg-red-bean-100 focus:[box-shadow:0px_0px_0px_3px_#F575893D] py-[2px] px-[6px]"
                  onClick={() => setValue("selectedCoin", selectedCoinAmount)}
                >
                  Max
                </Button>
              </div>
              <p className="">
                {" "}
                {getPrice(inputs.selectedCoin?.value || 0, selectedCoin.denom, {
                  format: true,
                  formatOptions: formatNumberOptions,
                })}
              </p>
            </div>
          }
        />
      </div>
      <div className="flex flex-col gap-1 w-full">
        <div className="flex w-full gap-2 items-center justify-between">
          <p className="text-gray-500 diatype-sm-regular">Fee (0.02%)</p>
          <p className="text-gray-700 diatype-sm-medium">$0.02</p>
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
      <Button fullWidth size="md">
        Swap
      </Button>
    </div>
  );
}
