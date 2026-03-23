import { useSubmitTx } from "./useSubmitTx.js";
import { useAccount } from "./useAccount.js";
import { useSigningClient } from "./useSigningClient.js";
import { useAppConfig } from "./useAppConfig.js";

import { execute, type ExecuteMsg } from "@left-curve/dango/actions";

import type { PerpsOrderKind, TriggerDirection, AppConfig } from "@left-curve/dango/types";

type UsePerpsSubmissionParameters = {
  perpsPairId: string;
  action: "buy" | "sell";
  operation: "limit" | "market";
  sizeValue: string;
  priceValue: string;
  controllers: { reset: () => void };
  onError: (error: unknown) => void;
  onSuccess?: () => void;
  tpslEnabled?: boolean;
  tpPrice?: string;
  slPrice?: string;
};

export function usePerpsSubmission(parameters: UsePerpsSubmissionParameters) {
  const {
    perpsPairId,
    action,
    operation,
    sizeValue,
    priceValue,
    controllers,
    onError,
    onSuccess,
    tpslEnabled,
    tpPrice,
    slPrice,
  } = parameters;

  const { account } = useAccount();
  const { data: signingClient } = useSigningClient();
  const { data: appConfig } = useAppConfig();

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

        const hasTpsl = tpslEnabled && (tpPrice || slPrice);

        if (!hasTpsl) {
          await signingClient.submitPerpsOrder({
            sender: account.address,
            pairId: perpsPairId,
            size: signedSize,
            kind,
            reduceOnly: false,
          });
          return;
        }

        const perpsAddress = (appConfig as AppConfig)?.addresses?.perps;
        if (!perpsAddress) throw new Error("Perps contract address not found");

        const conditionalSize = action === "buy" ? `-${sizeValue}` : sizeValue;

        const kindTypedData = "market" in kind
          ? {
              kind: [{ name: "market", type: "Market" }],
              Market: [{ name: "maxSlippage", type: "string" }],
            }
          : {
              kind: [{ name: "limit", type: "Limit" }],
              Limit: [
                { name: "limitPrice", type: "string" },
                { name: "postOnly", type: "bool" },
              ],
            };

        const executeMsgs: ExecuteMsg[] = [
          {
            contract: perpsAddress,
            msg: {
              trade: {
                submitOrder: {
                  pairId: perpsPairId,
                  size: signedSize,
                  kind,
                  reduceOnly: false,
                },
              },
            },
            typedData: {
              type: [{ name: "trade", type: "Trade" }],
              extraTypes: {
                Trade: [{ name: "submitOrder", type: "SubmitOrder" }],
                SubmitOrder: [
                  { name: "pairId", type: "string" },
                  { name: "size", type: "string" },
                  { name: "kind", type: "Kind" },
                  { name: "reduceOnly", type: "bool" },
                ],
                Kind: kindTypedData.kind,
                ...(kindTypedData.Market ? { Market: kindTypedData.Market } : {}),
                ...(kindTypedData.Limit ? { Limit: kindTypedData.Limit } : {}),
              },
            },
          },
        ];

        const conditionalTypedData = {
          type: [{ name: "trade", type: "Trade" }],
          extraTypes: {
            Trade: [{ name: "submitConditionalOrder", type: "SubmitConditionalOrder" }],
            SubmitConditionalOrder: [
              { name: "pairId", type: "string" },
              { name: "size", type: "string" },
              { name: "triggerPrice", type: "string" },
              { name: "triggerDirection", type: "string" },
              { name: "maxSlippage", type: "string" },
            ],
          },
        };

        if (tpPrice) {
          const tpDirection: TriggerDirection = action === "buy" ? "above" : "below";
          executeMsgs.push({
            contract: perpsAddress,
            msg: {
              trade: {
                submitConditionalOrder: {
                  pairId: perpsPairId,
                  size: conditionalSize,
                  triggerPrice: tpPrice,
                  triggerDirection: tpDirection,
                  maxSlippage: "0.05",
                },
              },
            },
            typedData: conditionalTypedData,
          });
        }

        if (slPrice) {
          const slDirection: TriggerDirection = action === "buy" ? "below" : "above";
          executeMsgs.push({
            contract: perpsAddress,
            msg: {
              trade: {
                submitConditionalOrder: {
                  pairId: perpsPairId,
                  size: conditionalSize,
                  triggerPrice: slPrice,
                  triggerDirection: slDirection,
                  maxSlippage: "0.05",
                },
              },
            },
            typedData: conditionalTypedData,
          });
        }

        await execute(signingClient, {
          sender: account.address,
          execute: executeMsgs,
        });
      },
      onError,
      onSuccess: () => {
        controllers.reset();
        onSuccess?.();
      },
    },
  });
}
