import { useMemo } from "react";
import { usePrices } from "./usePrices.js";

import { Decimal } from "@left-curve/dango/utils";

import type { AnyCoin, WithAmount } from "../types/coin.js";

type UseSpotMaxSizeParameters = {
  availableCoin: WithAmount<AnyCoin>;
  sizeCoin: WithAmount<AnyCoin>;
  action: "buy" | "sell";
  operation: "limit" | "market";
  priceValue: string;
};

export function useSpotMaxSize(parameters: UseSpotMaxSizeParameters) {
  const { availableCoin, sizeCoin, action, operation, priceValue } = parameters;
  const { convertAmount } = usePrices();

  const needsConversion = sizeCoin.denom !== availableCoin.denom;

  const maxSizeAmount = useMemo(() => {
    if (availableCoin.amount === "0") return 0;
    if (!needsConversion) return +availableCoin.amount;

    return operation === "limit"
      ? (() => {
          if (priceValue === "0") return 0;
          return action === "buy"
            ? Decimal(availableCoin.amount).div(priceValue).toNumber()
            : Decimal(availableCoin.amount).mul(priceValue).toNumber();
        })()
      : convertAmount(availableCoin.amount, availableCoin.denom, sizeCoin.denom);
  }, [sizeCoin, availableCoin, needsConversion, priceValue]);

  return maxSizeAmount;
}
