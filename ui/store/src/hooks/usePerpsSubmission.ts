import { useSubmitTx } from "./useSubmitTx.js";
import { useAccount } from "./useAccount.js";
import { useSigningClient } from "./useSigningClient.js";

import type { PerpsOrderKind } from "@left-curve/dango/types";

type UsePerpsSubmissionParameters = {
  perpsPairId: string;
  action: "buy" | "sell";
  operation: "limit" | "market";
  sizeValue: string;
  priceValue: string;
  controllers: { reset: () => void };
  onSuccess?: () => void;
};

export function usePerpsSubmission(parameters: UsePerpsSubmissionParameters) {
  const { perpsPairId, action, operation, sizeValue, priceValue, controllers, onSuccess } =
    parameters;

  const { account } = useAccount();
  const { data: signingClient } = useSigningClient();

  return useSubmitTx({
    mutation: {
      mutationFn: async () => {
        if (!signingClient) throw new Error("No signing client available");
        if (!account) throw new Error("No account found");

        const signedSize = action === "buy" ? sizeValue : `-${sizeValue}`;

        const kind: PerpsOrderKind =
          operation === "market"
            ? { market: { maxSlippage: "0.05" } }
            : { limit: { limitPrice: priceValue, postOnly: false } };

        await signingClient.submitPerpsOrder({
          sender: account.address,
          pairId: perpsPairId,
          size: signedSize,
          kind,
          reduceOnly: false,
        });
      },
      onSuccess: () => {
        controllers.reset();
        onSuccess?.();
      },
    },
  });
}
