import { useSubmitTx } from "./useSubmitTx.js";
import { useAccount } from "./useAccount.js";
import { useSigningClient } from "./useSigningClient.js";

import type { ChildOrder, PerpsOrderKind } from "@left-curve/dango/types";

type UsePerpsSubmissionParameters = {
  perpsPairId: string;
  action: "buy" | "sell";
  operation: "limit" | "market";
  sizeValue: string;
  priceValue: string;
  tpPrice?: string;
  slPrice?: string;
  reduceOnly?: boolean;
  controllers: { reset: () => void };
  onSuccess?: () => void;
};

const DEFAULT_TPSL_SLIPPAGE = "0.05";

// On-chain `Dec<i128, 6>` allows at most 6 fractional digits. Truncate (don't
// round) so we never exceed the user's intended/available size.
const SIZE_DECIMALS = 6;
function truncateSize(value: string): string {
  const trimmed = value.trim();
  if (!trimmed) return trimmed;
  const negative = trimmed.startsWith("-");
  const unsigned = negative ? trimmed.slice(1) : trimmed;
  const [intPart, fracPart = ""] = unsigned.split(".");
  const truncated =
    fracPart.length > SIZE_DECIMALS
      ? `${intPart}.${fracPart.slice(0, SIZE_DECIMALS)}`
      : unsigned;
  return negative ? `-${truncated}` : truncated;
}

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

        const truncatedSize = truncateSize(sizeValue);
        const signedSize = action === "buy" ? truncatedSize : `-${truncatedSize}`;

        const kind: PerpsOrderKind =
          operation === "market"
            ? { market: { maxSlippage: "0.05" } }
            : { limit: { limitPrice: priceValue, postOnly: false } };

        const tp: ChildOrder | undefined =
          tpPrice && Number(tpPrice) > 0
            ? { triggerPrice: tpPrice, maxSlippage: DEFAULT_TPSL_SLIPPAGE }
            : undefined;

        const sl: ChildOrder | undefined =
          slPrice && Number(slPrice) > 0
            ? { triggerPrice: slPrice, maxSlippage: DEFAULT_TPSL_SLIPPAGE }
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
