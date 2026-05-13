import { useSubmitTx } from "./useSubmitTx.js";
import { useAccount } from "./useAccount.js";
import { useSigningClient } from "./useSigningClient.js";
import { useBalances } from "./useBalances.js";

import { Decimal, formatUnits, parseUnits } from "@left-curve/dango/utils";

import type { CreateOrderRequest, PairId, PriceOption } from "@left-curve/dango/types";
import type { AnyCoin, WithAmount } from "../types/coin.js";

type UseSpotSubmissionParameters = {
  pairId: PairId;
  baseCoin: WithAmount<AnyCoin>;
  quoteCoin: WithAmount<AnyCoin>;
  availableCoin: WithAmount<AnyCoin>;
  sizeCoin: WithAmount<AnyCoin>;
  action: "buy" | "sell";
  operation: "limit" | "market";
  amount: { base: string; quote: string };
  priceValue: string;
  controllers: { reset: () => void; setValue: (name: string, value: string) => void };
  onSuccess?: () => void;
};

export function useSpotSubmission(parameters: UseSpotSubmissionParameters) {
  const {
    pairId,
    baseCoin,
    quoteCoin,
    availableCoin,
    action,
    operation,
    amount,
    priceValue,
    controllers,
    onSuccess,
  } = parameters;

  const { account } = useAccount();
  const { data: signingClient } = useSigningClient();
  const { data: balances = {} } = useBalances({ address: account?.address });

  return useSubmitTx({
    mutation: {
      mutationFn: async () => {
        if (!signingClient) throw new Error("No signing client available");
        if (!account) throw new Error("No account found");

        const isBase = baseCoin.denom === availableCoin.denom;
        const maxAvailable = balances[availableCoin.denom];
        const { baseDenom, quoteDenom } = pairId;

        const parsedQuoteAmount = parseUnits(amount.quote, quoteCoin.decimals);
        const parsedAmount = isBase
          ? parseUnits(amount.base, baseCoin.decimals)
          : parsedQuoteAmount;

        const orderAmount = Decimal(parsedAmount).gte(maxAvailable) ? maxAvailable : parsedAmount;

        const price: PriceOption =
          operation === "market"
            ? { market: { maxSlippage: "0.001" } }
            : { limit: formatUnits(priceValue, baseCoin.decimals - quoteCoin.decimals) };

        const order: CreateOrderRequest = {
          baseDenom,
          quoteDenom,
          price,
          amount:
            action === "buy" ? { bid: { quote: orderAmount } } : { ask: { base: orderAmount } },
          timeInForce: operation === "market" ? "IOC" : "GTC",
        };

        await signingClient.batchUpdateOrders({
          sender: account.address,
          creates: [order],
          funds: { [availableCoin.denom]: orderAmount },
        });
      },
      onSuccess: () => {
        controllers.reset();
        onSuccess?.();
      },
    },
  });
}
