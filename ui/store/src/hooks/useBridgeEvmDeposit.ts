import { createPublicClient, createWalletClient, custom, http } from "viem";
import { useConfig } from "./useConfig.js";
import { useQuery } from "@tanstack/react-query";
import { useSigningClient } from "./useSigningClient.js";
import { useAccount } from "./useAccount.js";
import { useSubmitTx } from "./useSubmitTx.js";

import { parseUnits } from "@left-curve/dango/utils";

import { ERC20_ABI, HYPERLANE_ROUTER_ABI, toAddr32 } from "@left-curve/dango/hyperlane";

import type { Connector } from "../types/connector.js";
import type { Chain as ViemChain } from "viem";
import type { AnyCoin } from "../types/coin.js";
import type { EIP1193Provider } from "../types/eip1193.js";
import type { MailBoxConfig, NonNullablePropertiesBy } from "@left-curve/dango/types";
import type { useBridgeState } from "./useBridgeState.js";

const MAX_SAFE = 2n ** 256n - 1n;

export type UseBridgeEvmDepositParameters = {
  connector?: Connector;
  coin: AnyCoin;
  config: NonNullable<ReturnType<typeof useBridgeState>["config"]>;
  amount: string;
};

export function useBridgeEvmDeposit(parameters: UseBridgeEvmDepositParameters) {
  const { connector, coin, amount, config } = parameters;
  if (!config || !config.router) throw new Error("Unexpected missing router config");

  const { bridger, router, chain } = config;

  console.log(router);

  const { account } = useAccount();
  const { getAppConfig } = useConfig();
  const { data: signingClient } = useSigningClient();

  const depositAmount = BigInt(parseUnits(amount, coin.decimals));

  const publicClient = createPublicClient({
    chain: chain as ViemChain,
    transport: http(),
  });

  const wallet = useQuery({
    enabled: !!connector,
    queryKey: ["bridge_evm", "provider", chain, connector?.id],
    queryFn: async () => {
      if (!connector) return undefined;
      const provider = await (
        connector as unknown as { getProvider: () => Promise<EIP1193Provider> }
      ).getProvider();

      const [evmAddress] = await provider.request({ method: "eth_requestAccounts" });

      return createWalletClient({
        chain: chain as ViemChain,
        transport: custom(provider),
        account: evmAddress,
      });
    },
  });

  const allowanceQuery = useQuery({
    enabled: !!wallet.data && !!router,
    queryKey: ["bridge_evm", "allowance", wallet?.data?.account.address, router.coin],
    initialData: MAX_SAFE,
    queryFn: async () => {
      if (router.coin === "native") return MAX_SAFE;
      const { data: client } = wallet as NonNullablePropertiesBy<typeof wallet, "data">;

      return await publicClient.readContract({
        address: router.coin,
        abi: ERC20_ABI,
        functionName: "allowance",
        args: [client.account.address, router.address],
      });
    },
  });

  const allowanceMutation = useSubmitTx({
    mutation: {
      mutationFn: async () => {
        if (!wallet.data || router.coin === "native") {
          throw new Error("Wasn't able to approve");
        }

        const { data: client } = wallet;

        await client.switchChain({ id: chain.id });

        const approveHash = await client.writeContract({
          address: router.coin,
          abi: ERC20_ABI,
          functionName: "approve",
          args: [router.address, depositAmount],
        });

        await publicClient.waitForTransactionReceipt({ hash: approveHash });

        await allowanceQuery.refetch();
      },
    },
  });

  const deposit = useSubmitTx({
    mutation: {
      mutationFn: async () => {
        if (!wallet.data || !signingClient || !account || !bridger) {
          throw new Error("Wasn't able to deposit");
        }

        const appConfig = await getAppConfig();
        const mailboxConfig: MailBoxConfig = await signingClient.queryWasmSmart({
          contract: appConfig.addresses.mailbox,
          msg: { config: {} },
        });

        const { data: client } = wallet;
        const { localDomain } = mailboxConfig;
        const recipientAddress = toAddr32(account.address);
        const protocolFee = BigInt(bridger.hyperlane_protocol_fee);

        const value = router.coin === "native" ? depositAmount + protocolFee : protocolFee;

        await client.switchChain({ id: chain.id });

        const txHash = await client.writeContract({
          address: router.address,
          abi: HYPERLANE_ROUTER_ABI,
          functionName: "transferRemote",
          args: [localDomain, `0x${recipientAddress}`, depositAmount],
          value,
        });

        await publicClient.waitForTransactionReceipt({ hash: txHash });
      },
    },
  });

  return {
    wallet,
    deposit,
    allowanceQuery,
    allowanceMutation,
  };
}
