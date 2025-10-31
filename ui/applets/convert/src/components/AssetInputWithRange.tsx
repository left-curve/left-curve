import { Input, numberMask, Skeleton, useApp, type useInputs } from "@left-curve/applets-kit";
import { useAccount, useBalances, usePrices } from "@left-curve/store";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import { RangeWithButtons } from "./RangeWithButtons";
import { formatNumber, formatUnits } from "@left-curve/dango/utils";

import type { AnyCoin } from "@left-curve/store/types";
import type React from "react";
import { PairAssetSelector } from "./PairAssetSelector";

type AssetInputWithRangeProps = {
  name: string;
  label?: string;
  asset: AnyCoin;
  isDisabled?: boolean;
  isLoading?: boolean;
  controllers: ReturnType<typeof useInputs>;
  showCoinSelector?: boolean;
  shouldValidate?: boolean;
  showRange?: boolean;
  onFocus?: () => void;
  onSelectCoin?: (denom: string) => void;
  triggerSimulation?: (reverse?: boolean) => void;
};

export const AssetInputWithRange: React.FC<AssetInputWithRangeProps> = (props) => {
  const { isConnected, account } = useAccount();
  const { getPrice } = usePrices();
  const { data: balances = {} } = useBalances({ address: account?.address });

  const { settings } = useApp();
  const { formatNumberOptions } = settings;

  const {
    name,
    asset,
    label,
    isDisabled,
    isLoading,
    shouldValidate,
    controllers,
    showRange,
    showCoinSelector,
    onFocus,
    onSelectCoin,
    triggerSimulation,
  } = props;
  const { register, setValue } = controllers;

  const balance = formatUnits(balances[asset.denom] || 0, asset.decimals);

  const {
    onChange,
    value = "0",
    ...control
  } = register(name, {
    strategy: "onChange",
    validate: (v) => {
      if (!isConnected || !shouldValidate) return true;
      if (Number(v) > Number(balance)) return m["errors.validations.insufficientFunds"]();
      return true;
    },
    mask: numberMask,
  });

  return (
    <Input
      isDisabled={isDisabled}
      placeholder="0"
      isLoading={isLoading}
      onFocus={() => onFocus?.()}
      {...control}
      value={value}
      onChange={(e) => {
        onChange(e);
        triggerSimulation?.();
      }}
      label={label}
      classNames={{
        base: "z-20",
        inputWrapper: "pl-0 py-3 flex-col h-auto gap-[6px] hover:bg-surface-secondary-rice",
        inputParent: "h-[34px] h3-bold",
        input: "!h3-bold",
      }}
      startText="right"
      startContent={
        showCoinSelector ? (
          <PairAssetSelector
            value={asset.denom}
            onChange={(d) => {
              onSelectCoin?.(d);
              triggerSimulation?.(true);
            }}
          />
        ) : (
          <div className="inline-flex flex-row items-center gap-3 diatype-m-regular h-[46px] rounded-md min-w-14 p-3 bg-transparent justify-start">
            <div className="flex gap-2 items-center font-semibold">
              <img src={asset.logoURI} alt={asset.symbol} className="w-8 h-8" />
              <p>{asset.symbol}</p>
            </div>
          </div>
        )
      }
      insideBottomComponent={
        <div className="flex flex-col w-full gap-2 pl-4">
          <div className="flex items-center justify-between gap-2 w-full h-[22px] text-ink-tertiary-500 diatype-sm-regular">
            <div className="flex items-center gap-2">
              <p>
                {formatNumber(balance, formatNumberOptions)} {asset.symbol}
              </p>
            </div>
            <div>
              {isLoading ? (
                <Skeleton className="w-14 h-4" />
              ) : (
                getPrice(value, asset.denom, {
                  format: true,
                  formatOptions: { ...formatNumberOptions, maximumTotalDigits: 6 },
                })
              )}
            </div>
          </div>
          {showRange && (
            <RangeWithButtons
              amount={value}
              balance={balance}
              setValue={(v) => {
                setValue(name, v);
                triggerSimulation?.();
              }}
              setActiveInput={() => onFocus?.()}
            />
          )}
        </div>
      }
    />
  );
};
