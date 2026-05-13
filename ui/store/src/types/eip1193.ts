import type { Hex } from "@left-curve/types";

type RpcSchema = readonly {
  Method: string;
  Parameters?: unknown;
  ReturnType?: unknown;
}[];

type PublicRpcSchema = [
  { Method: "eth_accounts"; Parameters?: undefined; ReturnType: Hex[] },
  { Method: "eth_chainId"; Parameters?: undefined; ReturnType: Hex },
  { Method: "eth_blockNumber"; Parameters?: undefined; ReturnType: Hex },
  { Method: "eth_call"; Parameters: [{ to: Hex; data?: Hex }, string?]; ReturnType: Hex },
  { Method: "eth_estimateGas"; Parameters: [{ to?: Hex; data?: Hex; value?: Hex }]; ReturnType: Hex },
  { Method: "eth_getBalance"; Parameters: [Hex, string?]; ReturnType: Hex },
  { Method: "eth_getTransactionReceipt"; Parameters: [Hex]; ReturnType: unknown },
  { Method: "eth_sendRawTransaction"; Parameters: [Hex]; ReturnType: Hex },
  { Method: "eth_sendTransaction"; Parameters: [{ from: Hex; to?: Hex; data?: Hex; value?: Hex; gas?: Hex }]; ReturnType: Hex },
  { Method: "personal_sign"; Parameters: [Hex, Hex]; ReturnType: Hex },
];

type WalletRpcSchema = [
  { Method: "eth_requestAccounts"; Parameters?: undefined; ReturnType: Hex[] },
  { Method: "eth_signTypedData_v4"; Parameters: [Hex, string]; ReturnType: Hex },
  { Method: "wallet_switchEthereumChain"; Parameters: [{ chainId: Hex }]; ReturnType: null },
  { Method: "wallet_addEthereumChain"; Parameters: [{ chainId: Hex; chainName: string; rpcUrls: string[]; nativeCurrency?: { name: string; symbol: string; decimals: number }; blockExplorerUrls?: string[] }]; ReturnType: null },
  { Method: "wallet_watchAsset"; Parameters: [{ type: string; options: { address: Hex; symbol: string; decimals: number; image?: string } }]; ReturnType: boolean },
];

type EIP1474Methods = [...PublicRpcSchema, ...WalletRpcSchema];

type ExtractMethod<methods extends RpcSchema, method extends string> = Extract<
  methods[number],
  { Method: method }
>;

export type EIP1193RequestFn<methods extends RpcSchema = EIP1474Methods> = <
  method extends methods[number]["Method"],
>(
  args: ExtractMethod<methods, method> extends { Parameters: infer params }
    ? { method: method; params: params }
    : { method: method; params?: undefined },
) => Promise<
  ExtractMethod<methods, method> extends { ReturnType: infer ret } ? ret : unknown
>;

export type EIP1193EventMap = {
  accountsChanged: (accounts: Hex[]) => void;
  chainChanged: (chainId: string) => void;
  connect: (info: { chainId: string }) => void;
  disconnect: (error: { code: number; message: string }) => void;
  message: (message: { type: string; data?: unknown }) => void;
};

export type EIP1193Provider = {
  request: EIP1193RequestFn;
  on: <event extends keyof EIP1193EventMap>(event: event, listener: EIP1193EventMap[event]) => void;
  removeListener: <event extends keyof EIP1193EventMap>(
    event: event,
    listener: EIP1193EventMap[event],
  ) => void;
};
