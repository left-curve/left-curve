import { useQuery } from "@tanstack/react-query";

import type { AnyCoin } from "../types/coin.js";
import type { useBridgeState } from "./useBridgeState.js";
import { useSubmitTx } from "./useSubmitTx.js";
import { useSigningClient } from "./useSigningClient.js";
import { getWithdrawlFee, transferRemote } from "@left-curve/dango/actions";
import { usePublicClient } from "./usePublicClient.js";
import { useAccount } from "./useAccount.js";
import { toAddr32 } from "@left-curve/dango/hyperlane";
import { parseUnits } from "@left-curve/dango/utils";

export type UseBridgeWithdrawParameters = {
  coin: AnyCoin;
  config: ReturnType<typeof useBridgeState>["config"];
  amount: string;
  recipient: string;
};

export function useBridgeWithdraw(parameters: UseBridgeWithdrawParameters) {
  const { coin, config, amount, recipient } = parameters;
  const { data: signingClient } = useSigningClient();
  const publicClient = usePublicClient();
  const { account } = useAccount();

  const withdrawFee = useQuery({
    enabled: !!coin && !!config?.router,
    queryKey: ["withdrawFee", config],
    queryFn: async () => {
      if (!coin || !config?.router) return;
      return await getWithdrawlFee(publicClient, {
        denom: coin.denom,
        remote: config.router.remote,
      });
    },
  });

  const withdraw = useSubmitTx({
    mutation: {
      mutationFn: async () => {
        if (!signingClient) throw new Error("Signing client not initialized");
        if (!config || !config.router) throw new Error("Bridge config not available");
        if (!account) throw new Error("Account not connected");
        if (!coin) throw new Error("Coin not selected");

        await transferRemote(signingClient, {
          sender: account.address,
          recipient: toAddr32(recipient as `0x${string}`),
          remote: {
            warp: {
              domain: config.router.domain,
              contract: toAddr32(config.router.address),
            },
          },
          funds: {
            [coin.denom]: parseUnits(amount, coin.decimals),
          },
        });
      },
    },
  });

  return {
    withdraw,
    withdrawFee,
  };
}
