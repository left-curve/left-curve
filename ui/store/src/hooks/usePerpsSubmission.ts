import { useSubmitTx } from "./useSubmitTx.js";
import { useAccount } from "./useAccount.js";
import { useSigningClient } from "./useSigningClient.js";

import { truncateDec } from "@left-curve/dango/utils";

import type { ChildOrder, PerpsOrderKind, PerpsTimeInForce } from "@left-curve/dango/types";

type UsePerpsSubmissionParameters = {
  perpsPairId: string;
  action: "buy" | "sell";
  operation: "limit" | "market";
  sizeValue: string;
  priceValue: string;
  tpPrice?: string;
  slPrice?: string;
  reduceOnly?: boolean;
  timeInForce?: PerpsTimeInForce;
  controllers: { reset: () => void };
  onSuccess?: () => void;
};

const DEFAULT_TPSL_SLIPPAGE = "0.05";

export function usePerpsSubmission(parameters: UsePerpsSubmissionParameters) {
  const {
    perpsPairId,
    action,
    operation,
    sizeValue,
    priceValue,
    tpPrice,
    slPrice,
    reduceOnly = false,
    timeInForce,
    controllers,
    onSuccess,
  } = parameters;

  const { account } = useAccount();
  const { data: signingClient } = useSigningClient();

  return useSubmitTx({
    mutation: {
      mutationFn: async () => {
        if (!signingClient) throw new Error("No signing client available");
        if (!account) throw new Error("No account found");

        const truncatedSize = truncateDec(sizeValue);
        const signedSize = action === "buy" ? truncatedSize : `-${truncatedSize}`;

        const kind: PerpsOrderKind =
          operation === "market"
            ? { market: { maxSlippage: "0.05" } }
            : { limit: { limitPrice: truncateDec(priceValue), timeInForce: timeInForce ?? "GTC" } };

        const tp: ChildOrder | undefined =
          tpPrice && Number(tpPrice) > 0
            ? { triggerPrice: truncateDec(tpPrice), maxSlippage: DEFAULT_TPSL_SLIPPAGE }
            : undefined;

        const sl: ChildOrder | undefined =
          slPrice && Number(slPrice) > 0
            ? { triggerPrice: truncateDec(slPrice), maxSlippage: DEFAULT_TPSL_SLIPPAGE }
            : undefined;

        await signingClient.submitPerpsOrder({
          sender: account.address,
          pairId: perpsPairId,
          size: signedSize,
          kind,
          reduceOnly,
          ...(tp ? { tp } : {}),
          ...(sl ? { sl } : {}),
        });
      },
      onSuccess: () => {
        controllers.reset();
        onSuccess?.();
      },
    },
  });
}
