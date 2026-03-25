import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useConnectors } from "./useConnectors.js";
import { registerOat } from "./pointsApi.js";
import { encodeBase64, encodeUtf8, decodeHex } from "@left-curve/dango/encoding";

import type { EIP1193Provider } from "../types/eip1193.js";

export type UseRegisterOatParameters = {
  pointsUrl: string;
  userIndex: number | undefined;
  onSuccess?: (evmAddress: string) => void;
  onError?: (error: Error) => void;
};

type RegisterOatApiResult = {
  success: Array<{ collection_id: number; token_id: string }>;
  errors: Array<{ collection_id: number; token_id: string; reason: string }>;
};

type RegisterOatResult = {
  evmAddress: string;
  apiResult: RegisterOatApiResult;
};

/**
 * Build EIP-712 typed data for OAT registration
 */
function buildOatTypedData(userIndex: number, evmAddress: string) {
  return {
    types: {
      EIP712Domain: [
        { name: "name", type: "string" },
        { name: "chainId", type: "uint256" },
        { name: "verifyingContract", type: "address" },
      ],
      Message: [
        { name: "userIndex", type: "uint256" },
        { name: "evmAddress", type: "string" },
      ],
    },
    domain: {
      name: "dango-oat",
      chainId: 1,
      verifyingContract: "0x0000000000000000000000000000000000000000",
    },
    primaryType: "Message" as const,
    message: {
      userIndex,
      evmAddress,
    },
  };
}

export function useRegisterOat(parameters: UseRegisterOatParameters) {
  const { pointsUrl, userIndex, onSuccess, onError } = parameters;
  const connectors = useConnectors();
  const queryClient = useQueryClient();

  const mutation = useMutation<RegisterOatResult, Error, string>({
    mutationFn: async (connectorId: string) => {
      if (!userIndex) {
        throw new Error("User not connected");
      }

      const connector = connectors.find((c) => c.id === connectorId);
      if (!connector) {
        throw new Error("Connector not found");
      }

      // Get provider and request accounts
      const provider = await (
        connector as unknown as { getProvider: () => Promise<EIP1193Provider> }
      ).getProvider();

      const accounts = await provider.request({ method: "eth_requestAccounts" });
      const evmAddress = (accounts[0] as string).toLowerCase();

      // Build EIP-712 typed data
      const typedData = buildOatTypedData(userIndex, evmAddress);
      const signData = JSON.stringify(typedData);

      // Sign the typed data
      const signature = (await provider.request({
        method: "eth_signTypedData_v4",
        params: [evmAddress as `0x${string}`, signData],
      })) as string;

      // Encode signature in the format expected by the backend
      const eip712Signature = {
        sig: encodeBase64(decodeHex(signature.slice(2))),
        typed_data: encodeBase64(encodeUtf8(signData)),
      };

      // Call the register OAT API
      const apiResult = await registerOat(pointsUrl, {
        user_index: userIndex,
        evm_address: evmAddress,
        signature: eip712Signature,
      });

      return { evmAddress, apiResult: apiResult as RegisterOatApiResult };
    },
    onSuccess: (data) => {
      // Invalidate OAT queries to refresh the list
      queryClient.invalidateQueries({ queryKey: ["oats", userIndex] });

      if (data.apiResult.success.length > 0 || data.apiResult.errors.length === 0) {
        onSuccess?.(data.evmAddress);
      }

      if (data.apiResult.errors.length > 0 && data.apiResult.success.length === 0) {
        onError?.(new Error(data.apiResult.errors[0]?.reason || "Registration failed"));
      }
    },
    onError: (error) => {
      onError?.(error);
    },
  });

  return {
    registerOat: mutation.mutateAsync,
    isLoading: mutation.isPending,
    isSuccess: mutation.isSuccess,
    isError: mutation.isError,
    error: mutation.error,
    data: mutation.data,
  };
}
