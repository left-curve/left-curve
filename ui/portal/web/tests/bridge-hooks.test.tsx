import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { toAddr32 } from "@left-curve/sdk/hyperlane";
import {
  BridgeConfigError,
  useBridgeEvmDeposit,
} from "../../../store/src/hooks/useBridgeEvmDeposit";
import { useBridgeState } from "../../../store/src/hooks/useBridgeState";
import { useBridgeWithdraw } from "../../../store/src/hooks/useBridgeWithdraw";
import { useEvmBalances } from "../../../store/src/hooks/useEvmBalances";
import { createQueryClientWrapper } from "./utils/query-client";

const hookMocks = vi.hoisted(() => ({
  createPublicClient: vi.fn(),
  createWalletClient: vi.fn(),
  custom: vi.fn(),
  getWithdrawalFee: vi.fn(),
  http: vi.fn(),
  publicClientGetBalance: vi.fn(),
  publicClientReadContract: vi.fn(),
  publicClientWaitForTransactionReceipt: vi.fn(),
  signingClientQueryWasmSmart: vi.fn(),
  transferRemote: vi.fn(),
  useAccount: vi.fn(),
  useAppConfig: vi.fn(),
  useConfig: vi.fn(),
  useConnectors: vi.fn(),
  usePublicClient: vi.fn(),
  useSigningClient: vi.fn(),
  walletSwitchChain: vi.fn(),
  walletWriteContract: vi.fn(),
}));

vi.mock("viem", () => ({
  createPublicClient: hookMocks.createPublicClient,
  createWalletClient: hookMocks.createWalletClient,
  custom: hookMocks.custom,
  http: hookMocks.http,
}));

vi.mock("@left-curve/sdk/actions", () => ({
  getWithdrawalFee: hookMocks.getWithdrawalFee,
  transferRemote: hookMocks.transferRemote,
}));

vi.mock("../../../store/src/hooks/useAccount.js", () => ({
  useAccount: hookMocks.useAccount,
}));

vi.mock("../../../store/src/hooks/useAppConfig.js", () => ({
  useAppConfig: hookMocks.useAppConfig,
}));

vi.mock("../../../store/src/hooks/useConfig.js", () => ({
  useConfig: hookMocks.useConfig,
}));

vi.mock("../../../store/src/hooks/useConnectors.js", () => ({
  useConnectors: hookMocks.useConnectors,
}));

vi.mock("../../../store/src/hooks/usePublicClient.js", () => ({
  usePublicClient: hookMocks.usePublicClient,
}));

vi.mock("../../../store/src/hooks/useSigningClient.js", () => ({
  useSigningClient: hookMocks.useSigningClient,
}));

vi.mock("../../../store/src/hooks/useStorage.js", async () => {
  const React = await import("react");

  return {
    useStorage: (_key: string, options: { initialValue?: unknown } = {}) =>
      React.useState(options.initialValue ?? null),
  };
});

vi.mock("../../../store/src/hooks/useSubmitTx.js", () => ({
  useSubmitTx: ({
    mutation,
  }: {
    mutation: { mutationFn: () => Promise<unknown>; onSuccess?: () => void };
  }) => ({
    isPending: false,
    mutateAsync: async () => {
      const result = await mutation.mutationFn();
      mutation.onSuccess?.();
      return result;
    },
  }),
}));

const usdcCoin = {
  decimals: 6,
  denom: "bridge/usdc",
  logoURI: "/usdc.png",
  name: "USD Coin",
  symbol: "USDC",
};

const ethCoin = {
  decimals: 18,
  denom: "bridge/eth",
  logoURI: "/eth.png",
  name: "Ether",
  symbol: "ETH",
};

const routerAddress = "0x1111111111111111111111111111111111111111";
const tokenAddress = "0x2222222222222222222222222222222222222222";
const nativeRouterAddress = "0x3333333333333333333333333333333333333333";
const mainnetRouterAddress = "0x8888888888888888888888888888888888888888";
const mainnetTokenAddress = "0x9999999999999999999999999999999999999999";
const arbitrumMainnetRouterAddress = "0x9d0ea335355da17ee89e50df43ab823416cf73d4";
const arbitrumMainnetTokenAddress = "0xaf88d065e77c8cc2239327c5edb3a432268e5831";
const arbitrumSepoliaRouterAddress = "0x9d0ea335355da17ee89e50df43ab823416cf73d4";
const arbitrumSepoliaTokenAddress = "0x75faf114eafb1bdbe2f0316df893fd58ce46aa4d";
const evmAccount = "0x0000000000000000000000000000000000000abc";
const evmRecipient = "0x4444444444444444444444444444444444444444";

const bridger = {
  chainId: 11155111,
  contracts: {
    mailbox: "0x5555555555555555555555555555555555555555",
    proxyAdmin: "0x7777777777777777777777777777777777777777",
    staticMessageIdMultisigIsmFactory: "0x6666666666666666666666666666666666666666",
  },
  domain: 17,
  estimatedTime: "5-30 mins",
  ism: {
    staticMessageIdMultisigIsm: {
      threshold: 1,
      validators: ["0xvalidator"],
    },
  },
  protocolFee: 77,
  name: "Sepolia Network",
  order: 0,
  rpcUrl: "https://sepolia.example",
  routes: [
    {
      symbol: "USDC",
      type: "erc20Collateral",
      tokenAddress,
      routerAddress,
      implementationAddress: "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    },
    {
      symbol: "ETH",
      type: "native",
      tokenAddress: "native",
      routerAddress: nativeRouterAddress,
      implementationAddress: "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
    },
  ],
};

const mainnetBridger = {
  ...bridger,
  chainId: 1,
  domain: 1,
  estimatedTime: "6 blocks | 1-3 mins",
  name: "Ethereum Network",
  order: 0,
  rpcUrl: "https://mainnet.example",
  routes: [
    {
      symbol: "USDC",
      type: "erc20Collateral",
      tokenAddress: mainnetTokenAddress,
      routerAddress: mainnetRouterAddress,
      implementationAddress: "0xcccccccccccccccccccccccccccccccccccccccc",
    },
  ],
};

const arbitrumMainnetBridger = {
  ...bridger,
  chainId: 42161,
  domain: 42161,
  estimatedTime: "1 block | <1 second",
  name: "Arbitrum Network",
  order: 1,
  rpcUrl: "https://arbitrum-mainnet.example",
  contracts: {
    ...bridger.contracts,
    mailbox: "0x979ca5202784112f4738403dbec5d0f3b9daabb9",
    proxyAdmin: "0x947303e34c1a2b97fb00c68c1cc4ca97b3361fe6",
    staticMessageIdMultisigIsmFactory: "0x12df53079d399a47e9e730df095b712b0fdfa791",
  },
  routes: [
    {
      symbol: "USDC",
      type: "erc20Collateral",
      tokenAddress: arbitrumMainnetTokenAddress,
      routerAddress: arbitrumMainnetRouterAddress,
      implementationAddress: "0x34dc3f292fc04e3dcc2830ac69bb5d4cd5e8f654",
    },
  ],
};

const arbitrumSepoliaBridger = {
  ...bridger,
  chainId: 421614,
  domain: 421614,
  estimatedTime: "1 block | <1 second",
  name: "Arbitrum Sepolia Network",
  order: 1,
  rpcUrl: "https://arbitrum-sepolia.example",
  contracts: {
    ...bridger.contracts,
    proxyAdmin: "0x947303e34c1a2b97fb00c68c1cc4ca97b3361fe6",
  },
  routes: [
    {
      symbol: "USDC",
      type: "erc20Collateral",
      tokenAddress: arbitrumSepoliaTokenAddress,
      routerAddress: arbitrumSepoliaRouterAddress,
      implementationAddress: "0x34dc3f292fc04e3dcc2830ac69bb5d4cd5e8f654",
    },
  ],
};

const bridgeEnvConfig = {
  evm: {
    "11155111": bridger,
    "421614": arbitrumSepoliaBridger,
  },
};

const evmBridgeConfig = {
  bridger,
  chain: {
    id: 11155111,
    name: "Sepolia",
  },
  router: {
    address: routerAddress,
    coin: tokenAddress,
    domain: bridger.domain,
    remote: {
      warp: {
        contract: toAddr32(routerAddress),
        domain: bridger.domain,
      },
    },
  },
};

const nativeEvmBridgeConfig = {
  bridger,
  chain: {
    id: 11155111,
    name: "Sepolia",
  },
  router: {
    address: nativeRouterAddress,
    coin: "native",
    domain: bridger.domain,
    remote: {
      warp: {
        contract: toAddr32(nativeRouterAddress),
        domain: bridger.domain,
      },
    },
  },
};

const unsupportedEvmBridgeConfig = {
  bridger,
  chain: {
    id: 11155111,
    name: "Sepolia",
  },
  router: undefined,
};

describe("bridge hooks", () => {
  const dangoPublicClient = { id: "dango-public-client" };
  const signingClient = {
    queryWasmSmart: hookMocks.signingClientQueryWasmSmart,
  };
  const evmPublicClient = {
    getBalance: hookMocks.publicClientGetBalance,
    readContract: hookMocks.publicClientReadContract,
    waitForTransactionReceipt: hookMocks.publicClientWaitForTransactionReceipt,
  };
  const walletClient = {
    account: {
      address: evmRecipient,
    },
    switchChain: hookMocks.walletSwitchChain,
    writeContract: hookMocks.walletWriteContract,
  };

  beforeEach(() => {
    hookMocks.useAccount.mockReturnValue({
      account: { address: evmAccount },
      isConnected: true,
    });
    hookMocks.useAppConfig.mockReturnValue({
      data: {
        addresses: {
          mailbox: "0x6d61696c626f7800000000000000000000000000",
        },
      },
    });
    hookMocks.useConfig.mockReturnValue({
      chain: {
        id: "dango-dev-1",
        name: "Devnet",
      },
      coins: {
        byDenom: {
          [ethCoin.denom]: ethCoin,
          [usdcCoin.denom]: usdcCoin,
        },
      },
    });
    hookMocks.useConnectors.mockReturnValue([]);
    hookMocks.usePublicClient.mockReturnValue(dangoPublicClient);
    hookMocks.useSigningClient.mockReturnValue({
      data: signingClient,
    });
    hookMocks.http.mockImplementation((url: string, options?: Record<string, unknown>) => ({
      ...options,
      type: "http",
      url,
    }));
    hookMocks.custom.mockImplementation((provider: unknown) => ({ provider, type: "custom" }));
    hookMocks.createPublicClient.mockReturnValue(evmPublicClient);
    hookMocks.publicClientGetBalance.mockResolvedValue(2000000000000000000n);
    hookMocks.createWalletClient.mockReturnValue(walletClient);
    hookMocks.publicClientReadContract.mockResolvedValue(100n);
    hookMocks.publicClientWaitForTransactionReceipt.mockResolvedValue({ status: "success" });
    hookMocks.signingClientQueryWasmSmart.mockResolvedValue({ localDomain: 999 });
    hookMocks.walletSwitchChain.mockResolvedValue(undefined);
    hookMocks.walletWriteContract.mockResolvedValue("0xtransaction");
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("exposes only supported bridge coins from the configured coin store", () => {
    const atomCoin = {
      decimals: 6,
      denom: "bridge/atom",
      logoURI: "/atom.png",
      name: "Cosmos Hub",
      symbol: "ATOM",
    };
    hookMocks.useConfig.mockReturnValue({
      chain: {
        id: "dango-dev-1",
        name: "Devnet",
      },
      coins: {
        byDenom: {
          [atomCoin.denom]: atomCoin,
          [ethCoin.denom]: ethCoin,
          [usdcCoin.denom]: usdcCoin,
        },
      },
    });
    const controllers = {
      inputs: {},
      reset: vi.fn(),
      setValue: vi.fn(),
    };

    const { result } = renderHook(
      () =>
        useBridgeState({
          action: "deposit",
          config: bridgeEnvConfig,
          controllers,
        }),
      { wrapper: createQueryClientWrapper() },
    );

    expect(result.current.coins).toEqual([usdcCoin]);

    act(() => result.current.changeCoin(atomCoin.denom));

    expect(result.current.coin).toEqual(atomCoin);
    expect(result.current.config).toBeUndefined();
  });

  it("loads native and ERC20 EVM balances for a connected external wallet address", async () => {
    const chain = {
      id: 11155111,
      name: "Sepolia",
      contracts: {
        erc20: [
          {
            address: tokenAddress,
            targetDenom: usdcCoin.denom,
          },
          {
            address: mainnetTokenAddress,
            targetDenom: "bridge/usdt",
          },
        ],
      },
    };
    hookMocks.publicClientReadContract.mockResolvedValueOnce(1500000n).mockResolvedValueOnce(250n);

    const { result } = renderHook(
      () =>
        useEvmBalances({
          address: evmRecipient,
          chain,
        }),
      { wrapper: createQueryClientWrapper() },
    );

    await waitFor(() =>
      expect(result.current.data).toEqual({
        "bridge/eth": "2000000000000000000",
        "bridge/usdc": "1500000",
        "bridge/usdt": "250",
      }),
    );

    expect(hookMocks.createPublicClient).toHaveBeenCalledWith({
      chain,
      transport: { batch: true, type: "http", url: expect.any(String) },
    });
    expect(hookMocks.publicClientGetBalance).toHaveBeenCalledWith({ address: evmRecipient });
    expect(hookMocks.publicClientReadContract).toHaveBeenNthCalledWith(
      1,
      expect.objectContaining({
        address: tokenAddress,
        args: [evmRecipient],
        functionName: "balanceOf",
      }),
    );
    expect(hookMocks.publicClientReadContract).toHaveBeenNthCalledWith(
      2,
      expect.objectContaining({
        address: mainnetTokenAddress,
        args: [evmRecipient],
        functionName: "balanceOf",
      }),
    );
  });

  it("surfaces native EVM balance failures before querying ERC20 balances", async () => {
    const chain = {
      id: 11155111,
      name: "Sepolia",
      contracts: {
        erc20: [
          {
            address: tokenAddress,
            targetDenom: usdcCoin.denom,
          },
        ],
      },
    };
    const balanceError = new Error("native balance unavailable");
    hookMocks.publicClientGetBalance.mockRejectedValueOnce(balanceError);

    const { result } = renderHook(
      () =>
        useEvmBalances({
          address: evmRecipient,
          chain,
        }),
      { wrapper: createQueryClientWrapper() },
    );

    await waitFor(() => expect(result.current.isError).toBe(true));

    expect(result.current.error).toBe(balanceError);
    expect(result.current.data).toBeUndefined();
    expect(hookMocks.createPublicClient).toHaveBeenCalledWith({
      chain,
      transport: { batch: true, type: "http", url: expect.any(String) },
    });
    expect(hookMocks.publicClientGetBalance).toHaveBeenCalledWith({ address: evmRecipient });
    expect(hookMocks.publicClientReadContract).not.toHaveBeenCalled();
  });

  it("does not return partial EVM balances when an ERC20 balance read fails", async () => {
    const chain = {
      id: 11155111,
      name: "Sepolia",
      contracts: {
        erc20: [
          {
            address: tokenAddress,
            targetDenom: usdcCoin.denom,
          },
          {
            address: mainnetTokenAddress,
            targetDenom: "bridge/usdt",
          },
        ],
      },
    };
    const tokenBalanceError = new Error("token balance unavailable");
    hookMocks.publicClientReadContract.mockRejectedValueOnce(tokenBalanceError);

    const { result } = renderHook(
      () =>
        useEvmBalances({
          address: evmRecipient,
          chain,
        }),
      { wrapper: createQueryClientWrapper() },
    );

    await waitFor(() => expect(result.current.isError).toBe(true));

    expect(result.current.error).toBe(tokenBalanceError);
    expect(result.current.data).toBeUndefined();
    expect(hookMocks.publicClientGetBalance).toHaveBeenCalledWith({ address: evmRecipient });
    expect(hookMocks.publicClientReadContract).toHaveBeenNthCalledWith(
      1,
      expect.objectContaining({
        address: tokenAddress,
        args: [evmRecipient],
        functionName: "balanceOf",
      }),
    );
    expect(hookMocks.publicClientReadContract).toHaveBeenNthCalledWith(
      2,
      expect.objectContaining({
        address: mainnetTokenAddress,
        args: [evmRecipient],
        functionName: "balanceOf",
      }),
    );
  });

  it("does not create an EVM client or query balances without an external wallet address", () => {
    const { result } = renderHook(
      () =>
        useEvmBalances({
          address: undefined,
          chain: {
            id: 11155111,
            name: "Sepolia",
            contracts: {
              erc20: [
                {
                  address: tokenAddress,
                  targetDenom: usdcCoin.denom,
                },
              ],
            },
          },
        }),
      { wrapper: createQueryClientWrapper() },
    );

    expect(result.current.data).toBeUndefined();
    expect(hookMocks.createPublicClient).not.toHaveBeenCalled();
    expect(hookMocks.publicClientGetBalance).not.toHaveBeenCalled();
    expect(hookMocks.publicClientReadContract).not.toHaveBeenCalled();
  });

  it("derives Hyperlane router config from the selected network and coin", async () => {
    const controllers = {
      inputs: {},
      reset: vi.fn(),
      setValue: vi.fn(),
    };

    const { result } = renderHook(
      () =>
        useBridgeState({
          action: "deposit",
          config: bridgeEnvConfig,
          controllers,
        }),
      { wrapper: createQueryClientWrapper() },
    );

    expect(result.current.networks).toEqual([
      { id: "11155111", name: "Sepolia Network", time: "5-30 mins" },
      { id: "421614", name: "Arbitrum Sepolia Network", time: "1 block | <1 second" },
    ]);

    act(() => result.current.changeCoin(usdcCoin.denom));
    act(() => result.current.setNetwork("11155111"));

    await waitFor(() => expect(result.current.config?.router).toBeDefined());

    expect(result.current.coin).toEqual(usdcCoin);
    expect(result.current.config?.router).toEqual({
      address: routerAddress,
      coin: tokenAddress,
      domain: bridger.domain,
      remote: {
        warp: {
          contract: toAddr32(routerAddress),
          domain: bridger.domain,
        },
      },
    });
  });

  it("derives Sepolia bridge routes when connected to a Dango testnet environment", async () => {
    hookMocks.useConfig.mockReturnValue({
      chain: {
        id: "dango-test-1",
        name: "Testnet",
      },
      coins: {
        byDenom: {
          [ethCoin.denom]: ethCoin,
          [usdcCoin.denom]: usdcCoin,
        },
      },
    });
    const controllers = {
      inputs: {},
      reset: vi.fn(),
      setValue: vi.fn(),
    };

    const { result } = renderHook(
      () =>
        useBridgeState({
          action: "deposit",
          config: bridgeEnvConfig,
          controllers,
        }),
      { wrapper: createQueryClientWrapper() },
    );

    expect(result.current.networks).toEqual([
      { id: "11155111", name: "Sepolia Network", time: "5-30 mins" },
      { id: "421614", name: "Arbitrum Sepolia Network", time: "1 block | <1 second" },
    ]);

    act(() => result.current.changeCoin(usdcCoin.denom));
    act(() => result.current.setNetwork("11155111"));

    await waitFor(() => expect(result.current.config?.router).toBeDefined());

    expect(result.current.config?.chain).toEqual(expect.objectContaining({ id: 11155111 }));
    expect(result.current.config?.bridger).toEqual(bridger);
    expect(result.current.config?.router).toEqual({
      address: routerAddress,
      coin: tokenAddress,
      domain: bridger.domain,
      remote: {
        warp: {
          contract: toAddr32(routerAddress),
          domain: bridger.domain,
        },
      },
    });
  });

  it("does not derive deprecated ETH router config even when backend config still has the route", async () => {
    const controllers = {
      inputs: {},
      reset: vi.fn(),
      setValue: vi.fn(),
    };

    const { result } = renderHook(
      () =>
        useBridgeState({
          action: "deposit",
          config: bridgeEnvConfig,
          controllers,
        }),
      { wrapper: createQueryClientWrapper() },
    );

    act(() => result.current.changeCoin(ethCoin.denom));
    act(() => result.current.setNetwork("11155111"));

    await waitFor(() => expect(result.current.config?.bridger).toBe(bridger));

    expect(result.current.coin).toEqual(ethCoin);
    expect(result.current.config?.chain).toEqual(expect.objectContaining({ id: 11155111 }));
    expect(result.current.config?.router).toBeUndefined();
  });

  it("derives Arbitrum Sepolia USDC bridge routes when connected to a Dango testnet environment", async () => {
    hookMocks.useConfig.mockReturnValue({
      chain: {
        id: "dango-test-1",
        name: "Testnet",
      },
      coins: {
        byDenom: {
          [ethCoin.denom]: ethCoin,
          [usdcCoin.denom]: usdcCoin,
        },
      },
    });
    const controllers = {
      inputs: {},
      reset: vi.fn(),
      setValue: vi.fn(),
    };

    const { result } = renderHook(
      () =>
        useBridgeState({
          action: "withdraw",
          config: bridgeEnvConfig,
          controllers,
        }),
      { wrapper: createQueryClientWrapper() },
    );

    expect(result.current.coins).toEqual([usdcCoin]);

    act(() => result.current.changeCoin(usdcCoin.denom));
    act(() => result.current.setNetwork("421614"));

    await waitFor(() => expect(result.current.config?.router).toBeDefined());

    expect(result.current.config?.chain).toEqual(expect.objectContaining({ id: 421614 }));
    expect(result.current.config?.bridger).toEqual(arbitrumSepoliaBridger);
    expect(result.current.config?.router).toEqual({
      address: arbitrumSepoliaRouterAddress,
      coin: arbitrumSepoliaTokenAddress,
      domain: arbitrumSepoliaBridger.domain,
      remote: {
        warp: {
          contract: toAddr32(arbitrumSepoliaRouterAddress),
          domain: arbitrumSepoliaBridger.domain,
        },
      },
    });
  });

  it("derives the production Ethereum router config when connected to the Dango mainnet", async () => {
    hookMocks.useConfig.mockReturnValue({
      chain: {
        id: "dango-1",
        name: "Mainnet",
      },
      coins: {
        byDenom: {
          [ethCoin.denom]: ethCoin,
          [usdcCoin.denom]: usdcCoin,
        },
      },
    });
    const controllers = {
      inputs: {},
      reset: vi.fn(),
      setValue: vi.fn(),
    };

    const { result } = renderHook(
      () =>
        useBridgeState({
          action: "deposit",
          config: {
            evm: {
              "1": mainnetBridger,
              "42161": arbitrumMainnetBridger,
            },
          },
          controllers,
        }),
      { wrapper: createQueryClientWrapper() },
    );

    expect(result.current.networks).toEqual([
      { id: "1", name: "Ethereum Network", time: "6 blocks | 1-3 mins" },
      { id: "42161", name: "Arbitrum Network", time: "1 block | <1 second" },
    ]);

    act(() => result.current.changeCoin(usdcCoin.denom));
    act(() => result.current.setNetwork("1"));

    await waitFor(() => expect(result.current.config?.router).toBeDefined());

    expect(result.current.config?.chain).toEqual(expect.objectContaining({ id: 1 }));
    expect(result.current.config?.bridger).toEqual(mainnetBridger);
    expect(result.current.config?.router).toEqual({
      address: mainnetRouterAddress,
      coin: mainnetTokenAddress,
      domain: mainnetBridger.domain,
      remote: {
        warp: {
          contract: toAddr32(mainnetRouterAddress),
          domain: mainnetBridger.domain,
        },
      },
    });
  });

  it("derives the production Arbitrum USDC router config when connected to the Dango mainnet", async () => {
    hookMocks.useConfig.mockReturnValue({
      chain: {
        id: "dango-1",
        name: "Mainnet",
      },
      coins: {
        byDenom: {
          [ethCoin.denom]: ethCoin,
          [usdcCoin.denom]: usdcCoin,
        },
      },
    });
    const controllers = {
      inputs: {},
      reset: vi.fn(),
      setValue: vi.fn(),
    };

    const { result } = renderHook(
      () =>
        useBridgeState({
          action: "deposit",
          config: {
            evm: {
              "1": mainnetBridger,
              "42161": arbitrumMainnetBridger,
            },
          },
          controllers,
        }),
      { wrapper: createQueryClientWrapper() },
    );

    expect(result.current.networks).toEqual([
      { id: "1", name: "Ethereum Network", time: "6 blocks | 1-3 mins" },
      { id: "42161", name: "Arbitrum Network", time: "1 block | <1 second" },
    ]);

    act(() => result.current.changeCoin(usdcCoin.denom));
    act(() => result.current.setNetwork("42161"));

    await waitFor(() => expect(result.current.config?.router).toBeDefined());

    expect(result.current.config?.chain).toEqual(expect.objectContaining({ id: 42161 }));
    expect(result.current.config?.bridger).toEqual(arbitrumMainnetBridger);
    expect(result.current.config?.router).toEqual({
      address: arbitrumMainnetRouterAddress,
      coin: arbitrumMainnetTokenAddress,
      domain: arbitrumMainnetBridger.domain,
      remote: {
        warp: {
          contract: toAddr32(arbitrumMainnetRouterAddress),
          domain: arbitrumMainnetBridger.domain,
        },
      },
    });
  });

  it("keeps unsupported bridge coin routes explicit when backend config has no matching warp route", async () => {
    hookMocks.useConfig.mockReturnValue({
      chain: {
        id: "dango-1",
        name: "Mainnet",
      },
      coins: {
        byDenom: {
          [ethCoin.denom]: ethCoin,
          [usdcCoin.denom]: usdcCoin,
        },
      },
    });
    const controllers = {
      inputs: {},
      reset: vi.fn(),
      setValue: vi.fn(),
    };

    const { result } = renderHook(
      () =>
        useBridgeState({
          action: "deposit",
          config: {
            evm: {
              "1": mainnetBridger,
            },
          },
          controllers,
        }),
      { wrapper: createQueryClientWrapper() },
    );

    act(() => result.current.changeCoin(ethCoin.denom));
    act(() => result.current.setNetwork("1"));

    await waitFor(() => expect(result.current.config?.bridger).toBe(mainnetBridger));

    expect(result.current.coin).toEqual(ethCoin);
    expect(result.current.config?.chain).toEqual(expect.objectContaining({ id: 1 }));
    expect(result.current.config?.router).toBeUndefined();
  });

  it("keeps selected EVM networks explicit when backend bridge config is missing", async () => {
    const controllers = {
      inputs: {},
      reset: vi.fn(),
      setValue: vi.fn(),
    };

    const { result } = renderHook(
      () =>
        useBridgeState({
          action: "deposit",
          config: {
            evm: {},
          },
          controllers,
        }),
      { wrapper: createQueryClientWrapper() },
    );

    act(() => result.current.changeCoin(usdcCoin.denom));
    act(() => result.current.setNetwork("11155111"));

    await waitFor(() => expect(result.current.config?.chain).toBeDefined());

    expect(result.current.coin).toEqual(usdcCoin);
    expect(result.current.config?.chain).toEqual(expect.objectContaining({ id: 11155111 }));
    expect(result.current.config?.bridger).toBeUndefined();
    expect(result.current.config?.router).toBeUndefined();
  });

  it("tracks the selected EVM connector and resets bridge form state when action or connection changes", async () => {
    const metamask = {
      id: "metamask",
      name: "MetaMask",
    };
    hookMocks.useConnectors.mockReturnValue([metamask]);
    const controllers = {
      inputs: {},
      reset: vi.fn(),
      setValue: vi.fn(),
    };

    const { rerender, result } = renderHook(
      ({ action }: { action: "deposit" | "withdraw" }) =>
        useBridgeState({
          action,
          config: bridgeEnvConfig,
          controllers,
        }),
      {
        initialProps: { action: "deposit" },
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(() => expect(controllers.reset).toHaveBeenCalledOnce());

    act(() => result.current.setConnectorId("metamask"));
    await waitFor(() => expect(result.current.connector).toBe(metamask));

    act(() => result.current.changeCoin(usdcCoin.denom));
    act(() => result.current.setNetwork("11155111"));
    await waitFor(() => expect(result.current.config?.router).toBeDefined());

    rerender({ action: "withdraw" });

    await waitFor(() => expect(controllers.reset).toHaveBeenCalledTimes(2));
    expect(result.current.connector).toBeUndefined();
    expect(result.current.coin).toBeUndefined();
    expect(result.current.network).toBeUndefined();
    expect(result.current.config).toBeUndefined();

    act(() => result.current.setConnectorId("metamask"));
    await waitFor(() => expect(result.current.connector).toBe(metamask));

    hookMocks.useAccount.mockReturnValue({
      account: undefined,
      isConnected: false,
    });
    rerender({ action: "withdraw" });

    await waitFor(() => expect(controllers.reset).toHaveBeenCalledTimes(3));
    expect(result.current.connector).toBeUndefined();
  });

  it("quotes withdrawal fees and submits Dango transferRemote with parsed funds", async () => {
    const reset = vi.fn();
    hookMocks.getWithdrawalFee.mockResolvedValue("2500000");

    const { result } = renderHook(
      () =>
        useBridgeWithdraw({
          amount: "3.25",
          coin: usdcCoin,
          config: evmBridgeConfig,
          recipient: evmRecipient,
          reset,
        }),
      { wrapper: createQueryClientWrapper() },
    );

    await waitFor(() => expect(result.current.withdrawFee.data).toBe("2.5"));

    expect(hookMocks.getWithdrawalFee).toHaveBeenCalledWith(dangoPublicClient, {
      denom: usdcCoin.denom,
      remote: evmBridgeConfig.router.remote,
    });

    await act(async () => {
      await result.current.withdraw.mutateAsync();
    });

    expect(hookMocks.transferRemote).toHaveBeenCalledWith(signingClient, {
      sender: evmAccount,
      recipient: toAddr32(evmRecipient),
      remote: {
        warp: {
          contract: toAddr32(routerAddress),
          domain: bridger.domain,
        },
      },
      funds: {
        [usdcCoin.denom]: "3250000",
      },
    });
    expect(reset).toHaveBeenCalledOnce();
  });

  it("keeps withdrawal form state intact when the backend transfer fails", async () => {
    const reset = vi.fn();
    hookMocks.getWithdrawalFee.mockResolvedValue("2500000");
    hookMocks.transferRemote.mockRejectedValueOnce(new Error("remote transfer rejected"));

    const { result } = renderHook(
      () =>
        useBridgeWithdraw({
          amount: "3.25",
          coin: usdcCoin,
          config: evmBridgeConfig,
          recipient: evmRecipient,
          reset,
        }),
      { wrapper: createQueryClientWrapper() },
    );

    await waitFor(() => expect(result.current.withdrawFee.data).toBe("2.5"));

    await expect(result.current.withdraw.mutateAsync()).rejects.toThrow("remote transfer rejected");

    expect(hookMocks.transferRemote).toHaveBeenCalledWith(signingClient, {
      sender: evmAccount,
      recipient: toAddr32(evmRecipient),
      remote: {
        warp: {
          contract: toAddr32(routerAddress),
          domain: bridger.domain,
        },
      },
      funds: {
        [usdcCoin.denom]: "3250000",
      },
    });
    expect(reset).not.toHaveBeenCalled();
  });

  it("does not re-quote withdrawal fees for amount or recipient edits and submits the latest transfer", async () => {
    const reset = vi.fn();
    hookMocks.getWithdrawalFee.mockResolvedValue("2500000");
    const nextRecipient = "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    const { rerender, result } = renderHook(
      ({ amount, recipient }: { amount: string; recipient: string }) =>
        useBridgeWithdraw({
          amount,
          coin: usdcCoin,
          config: evmBridgeConfig,
          recipient,
          reset,
        }),
      {
        initialProps: {
          amount: "3.25",
          recipient: evmRecipient,
        },
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(() => expect(result.current.withdrawFee.data).toBe("2.5"));

    rerender({
      amount: "4.5",
      recipient: nextRecipient,
    });

    expect(result.current.withdrawFee.data).toBe("2.5");
    expect(hookMocks.getWithdrawalFee).toHaveBeenCalledOnce();

    await act(async () => {
      await result.current.withdraw.mutateAsync();
    });

    expect(hookMocks.transferRemote).toHaveBeenCalledWith(signingClient, {
      sender: evmAccount,
      recipient: toAddr32(nextRecipient),
      remote: {
        warp: {
          contract: toAddr32(routerAddress),
          domain: bridger.domain,
        },
      },
      funds: {
        [usdcCoin.denom]: "4500000",
      },
    });
    expect(reset).toHaveBeenCalledOnce();
  });

  it("submits native Dango withdrawals with 18-decimal parsed funds and the native warp route", async () => {
    const reset = vi.fn();
    hookMocks.getWithdrawalFee.mockResolvedValue("100000000000000000");

    const { result } = renderHook(
      () =>
        useBridgeWithdraw({
          amount: "0.125",
          coin: ethCoin,
          config: nativeEvmBridgeConfig,
          recipient: evmRecipient,
          reset,
        }),
      { wrapper: createQueryClientWrapper() },
    );

    await waitFor(() => expect(result.current.withdrawFee.data).toBe("0.1"));

    expect(hookMocks.getWithdrawalFee).toHaveBeenCalledWith(dangoPublicClient, {
      denom: ethCoin.denom,
      remote: nativeEvmBridgeConfig.router.remote,
    });

    await act(async () => {
      await result.current.withdraw.mutateAsync();
    });

    expect(hookMocks.transferRemote).toHaveBeenCalledWith(signingClient, {
      sender: evmAccount,
      recipient: toAddr32(evmRecipient),
      remote: {
        warp: {
          contract: toAddr32(nativeRouterAddress),
          domain: bridger.domain,
        },
      },
      funds: {
        [ethCoin.denom]: "125000000000000000",
      },
    });
    expect(reset).toHaveBeenCalledOnce();
  });

  it("re-quotes withdrawal fees when the selected bridge route changes", async () => {
    const reset = vi.fn();
    hookMocks.getWithdrawalFee
      .mockResolvedValueOnce("2500000")
      .mockResolvedValueOnce("100000000000000000");

    const { rerender, result } = renderHook(
      ({ coin, config }: { coin: typeof usdcCoin; config: typeof evmBridgeConfig }) =>
        useBridgeWithdraw({
          amount: "3.25",
          coin,
          config,
          recipient: evmRecipient,
          reset,
        }),
      {
        initialProps: {
          coin: usdcCoin,
          config: evmBridgeConfig,
        },
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(() => expect(result.current.withdrawFee.data).toBe("2.5"));

    rerender({
      coin: ethCoin,
      config: nativeEvmBridgeConfig,
    });

    await waitFor(() => expect(result.current.withdrawFee.data).toBe("0.1"));
    expect(hookMocks.getWithdrawalFee).toHaveBeenNthCalledWith(1, dangoPublicClient, {
      denom: usdcCoin.denom,
      remote: evmBridgeConfig.router.remote,
    });
    expect(hookMocks.getWithdrawalFee).toHaveBeenNthCalledWith(2, dangoPublicClient, {
      denom: ethCoin.denom,
      remote: nativeEvmBridgeConfig.router.remote,
    });
    expect(hookMocks.transferRemote).not.toHaveBeenCalled();
    expect(reset).not.toHaveBeenCalled();
  });

  it("treats empty withdrawal fee responses from the backend as zero", async () => {
    const reset = vi.fn();
    hookMocks.getWithdrawalFee.mockResolvedValue(undefined);

    const { result } = renderHook(
      () =>
        useBridgeWithdraw({
          amount: "3.25",
          coin: usdcCoin,
          config: evmBridgeConfig,
          recipient: evmRecipient,
          reset,
        }),
      { wrapper: createQueryClientWrapper() },
    );

    await waitFor(() => expect(hookMocks.getWithdrawalFee).toHaveBeenCalledOnce());

    expect(result.current.withdrawFee.data).toBe("0");
    expect(hookMocks.getWithdrawalFee).toHaveBeenCalledWith(dangoPublicClient, {
      denom: usdcCoin.denom,
      remote: evmBridgeConfig.router.remote,
    });
    expect(hookMocks.transferRemote).not.toHaveBeenCalled();
    expect(reset).not.toHaveBeenCalled();
  });

  it("surfaces withdrawal fee quote failures while keeping the initial zero fee", async () => {
    const reset = vi.fn();
    const quoteError = new Error("withdrawal fee unavailable");
    hookMocks.getWithdrawalFee.mockRejectedValueOnce(quoteError);

    const { result } = renderHook(
      () =>
        useBridgeWithdraw({
          amount: "3.25",
          coin: usdcCoin,
          config: evmBridgeConfig,
          recipient: evmRecipient,
          reset,
        }),
      { wrapper: createQueryClientWrapper() },
    );

    await waitFor(() => expect(result.current.withdrawFee.isError).toBe(true));

    expect(hookMocks.getWithdrawalFee).toHaveBeenCalledWith(dangoPublicClient, {
      denom: usdcCoin.denom,
      remote: evmBridgeConfig.router.remote,
    });
    expect(result.current.withdrawFee.error).toBe(quoteError);
    expect(result.current.withdrawFee.data).toBe("0");
    expect(hookMocks.transferRemote).not.toHaveBeenCalled();
    expect(reset).not.toHaveBeenCalled();
  });

  it("does not submit Dango withdrawals without a connected account", async () => {
    const reset = vi.fn();
    hookMocks.getWithdrawalFee.mockResolvedValue("2500000");
    hookMocks.useAccount.mockReturnValue({
      account: undefined,
      isConnected: false,
    });

    const { result } = renderHook(
      () =>
        useBridgeWithdraw({
          amount: "3.25",
          coin: usdcCoin,
          config: evmBridgeConfig,
          recipient: evmRecipient,
          reset,
        }),
      { wrapper: createQueryClientWrapper() },
    );

    await waitFor(() => expect(result.current.withdrawFee.data).toBe("2.5"));

    await expect(result.current.withdraw.mutateAsync()).rejects.toThrow("Account not connected");

    expect(hookMocks.transferRemote).not.toHaveBeenCalled();
    expect(reset).not.toHaveBeenCalled();
  });

  it("does not submit Dango withdrawals without a signing client", async () => {
    const reset = vi.fn();
    hookMocks.getWithdrawalFee.mockResolvedValue("2500000");
    hookMocks.useSigningClient.mockReturnValue({
      data: undefined,
    });

    const { result } = renderHook(
      () =>
        useBridgeWithdraw({
          amount: "3.25",
          coin: usdcCoin,
          config: evmBridgeConfig,
          recipient: evmRecipient,
          reset,
        }),
      { wrapper: createQueryClientWrapper() },
    );

    await waitFor(() => expect(result.current.withdrawFee.data).toBe("2.5"));

    expect(hookMocks.getWithdrawalFee).toHaveBeenCalledWith(dangoPublicClient, {
      denom: usdcCoin.denom,
      remote: evmBridgeConfig.router.remote,
    });

    await expect(result.current.withdraw.mutateAsync()).rejects.toThrow(
      "Signing client not initialized",
    );

    expect(hookMocks.transferRemote).not.toHaveBeenCalled();
    expect(reset).not.toHaveBeenCalled();
  });

  it("does not quote or submit withdrawals when the selected bridge config has no router", async () => {
    const reset = vi.fn();
    hookMocks.getWithdrawalFee.mockResolvedValue("2500000");

    const { result } = renderHook(
      () =>
        useBridgeWithdraw({
          amount: "3.25",
          coin: usdcCoin,
          config: unsupportedEvmBridgeConfig,
          recipient: evmRecipient,
          reset,
        }),
      { wrapper: createQueryClientWrapper() },
    );

    expect(result.current.withdrawFee.data).toBe("0");
    expect(hookMocks.getWithdrawalFee).not.toHaveBeenCalled();

    await expect(result.current.withdraw.mutateAsync()).rejects.toThrow(
      "Bridge config not available",
    );

    expect(hookMocks.transferRemote).not.toHaveBeenCalled();
    expect(reset).not.toHaveBeenCalled();
  });

  it("requests an EVM wallet, checks ERC20 allowance, and submits approval transactions", async () => {
    const provider = {
      request: vi.fn().mockResolvedValue([evmRecipient]),
    };
    const connector = {
      getProvider: vi.fn().mockResolvedValue(provider),
      id: "metamask",
    };

    const { result } = renderHook(
      () =>
        useBridgeEvmDeposit({
          amount: "1.5",
          coin: usdcCoin,
          config: evmBridgeConfig,
          connector,
        }),
      { wrapper: createQueryClientWrapper() },
    );

    await waitFor(() =>
      expect(provider.request).toHaveBeenCalledWith({ method: "eth_requestAccounts" }),
    );
    await waitFor(() =>
      expect(hookMocks.publicClientReadContract).toHaveBeenCalledWith(
        expect.objectContaining({
          address: tokenAddress,
          args: [evmRecipient, routerAddress],
          functionName: "allowance",
        }),
      ),
    );

    await act(async () => {
      await result.current.allowanceMutation.mutateAsync();
    });

    expect(hookMocks.createPublicClient).toHaveBeenCalledWith({
      chain: evmBridgeConfig.chain,
      transport: { type: "http", url: expect.any(String) },
    });
    expect(hookMocks.createWalletClient).toHaveBeenCalledWith({
      account: evmRecipient,
      chain: evmBridgeConfig.chain,
      transport: { provider, type: "custom" },
    });
    expect(hookMocks.walletSwitchChain).toHaveBeenCalledWith({ id: 11155111 });
    expect(hookMocks.walletWriteContract).toHaveBeenCalledWith(
      expect.objectContaining({
        address: tokenAddress,
        args: [routerAddress, 1500000n],
        functionName: "approve",
      }),
    );
    expect(hookMocks.publicClientWaitForTransactionReceipt).toHaveBeenCalledWith({
      hash: "0xtransaction",
    });
  });

  it("refreshes the ERC20 allowance from the EVM client after approval confirms", async () => {
    const provider = {
      request: vi.fn().mockResolvedValue([evmRecipient]),
    };
    const connector = {
      getProvider: vi.fn().mockResolvedValue(provider),
      id: "metamask",
    };
    hookMocks.publicClientReadContract.mockResolvedValueOnce(100n).mockResolvedValueOnce(1500000n);

    const { result } = renderHook(
      () =>
        useBridgeEvmDeposit({
          amount: "1.5",
          coin: usdcCoin,
          config: evmBridgeConfig,
          connector,
        }),
      { wrapper: createQueryClientWrapper() },
    );

    await waitFor(() => expect(result.current.allowanceQuery.data).toBe(100n));

    await act(async () => {
      await result.current.allowanceMutation.mutateAsync();
    });

    await waitFor(() => expect(result.current.allowanceQuery.data).toBe(1500000n));
    expect(hookMocks.publicClientReadContract).toHaveBeenCalledTimes(2);
    expect(hookMocks.publicClientReadContract).toHaveBeenNthCalledWith(
      1,
      expect.objectContaining({
        address: tokenAddress,
        args: [evmRecipient, routerAddress],
        functionName: "allowance",
      }),
    );
    expect(hookMocks.publicClientReadContract).toHaveBeenNthCalledWith(
      2,
      expect.objectContaining({
        address: tokenAddress,
        args: [evmRecipient, routerAddress],
        functionName: "allowance",
      }),
    );
    expect(hookMocks.walletWriteContract).toHaveBeenCalledWith(
      expect.objectContaining({
        address: tokenAddress,
        args: [routerAddress, 1500000n],
        functionName: "approve",
      }),
    );
  });

  it("uses the latest edited amount for EVM approval and deposit transactions", async () => {
    const provider = {
      request: vi.fn().mockResolvedValue([evmRecipient]),
    };
    const connector = {
      getProvider: vi.fn().mockResolvedValue(provider),
      id: "metamask",
    };

    const { rerender, result } = renderHook(
      ({ amount }: { amount: string }) =>
        useBridgeEvmDeposit({
          amount,
          coin: usdcCoin,
          config: evmBridgeConfig,
          connector,
        }),
      {
        initialProps: {
          amount: "1.5",
        },
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(() => expect(result.current.wallet.data?.account.address).toBe(evmRecipient));

    rerender({ amount: "2.75" });

    await act(async () => {
      await result.current.allowanceMutation.mutateAsync();
    });

    expect(connector.getProvider).toHaveBeenCalledOnce();
    expect(provider.request).toHaveBeenCalledOnce();
    expect(hookMocks.walletWriteContract).toHaveBeenCalledWith(
      expect.objectContaining({
        address: tokenAddress,
        args: [routerAddress, 2750000n],
        functionName: "approve",
      }),
    );

    hookMocks.walletSwitchChain.mockClear();
    hookMocks.walletWriteContract.mockClear();
    hookMocks.publicClientWaitForTransactionReceipt.mockClear();

    await act(async () => {
      await result.current.deposit.mutateAsync();
    });

    expect(hookMocks.signingClientQueryWasmSmart).toHaveBeenCalledWith({
      contract: "0x6d61696c626f7800000000000000000000000000",
      msg: { config: {} },
    });
    expect(hookMocks.walletSwitchChain).toHaveBeenCalledWith({ id: 11155111 });
    expect(hookMocks.walletWriteContract).toHaveBeenCalledWith(
      expect.objectContaining({
        address: routerAddress,
        args: [999, `0x${toAddr32(evmAccount)}`, 2750000n],
        functionName: "transferRemote",
        value: 77n,
      }),
    );
    expect(hookMocks.publicClientWaitForTransactionReceipt).toHaveBeenCalledWith({
      hash: "0xtransaction",
    });
  });

  it("does not submit approval or deposit transactions without a connector wallet", async () => {
    const { result } = renderHook(
      () =>
        useBridgeEvmDeposit({
          amount: "1.5",
          coin: usdcCoin,
          config: evmBridgeConfig,
          connector: undefined,
        }),
      { wrapper: createQueryClientWrapper() },
    );

    expect(result.current.allowanceQuery.data).toBe(2n ** 256n - 1n);

    await expect(result.current.allowanceMutation.mutateAsync()).rejects.toThrow(
      "Wasn't able to approve",
    );
    await expect(result.current.deposit.mutateAsync()).rejects.toThrow("Wasn't able to deposit");

    expect(hookMocks.walletSwitchChain).not.toHaveBeenCalled();
    expect(hookMocks.walletWriteContract).not.toHaveBeenCalled();
    expect(hookMocks.publicClientWaitForTransactionReceipt).not.toHaveBeenCalled();
    expect(hookMocks.signingClientQueryWasmSmart).not.toHaveBeenCalled();
  });

  it("rejects EVM deposit hooks when the selected bridge config has no router", () => {
    expect(() =>
      renderHook(
        () =>
          useBridgeEvmDeposit({
            amount: "1.5",
            coin: usdcCoin,
            config: unsupportedEvmBridgeConfig,
            connector: undefined,
          }),
        { wrapper: createQueryClientWrapper() },
      ),
    ).toThrow(BridgeConfigError);

    expect(hookMocks.createPublicClient).not.toHaveBeenCalled();
    expect(hookMocks.createWalletClient).not.toHaveBeenCalled();
    expect(hookMocks.walletWriteContract).not.toHaveBeenCalled();
  });

  it("surfaces EVM wallet acquisition failures before approval or deposit transactions", async () => {
    const walletError = new Error("wallet provider unavailable");
    const connector = {
      getProvider: vi.fn().mockRejectedValue(walletError),
      id: "metamask",
    };

    const { result } = renderHook(
      () =>
        useBridgeEvmDeposit({
          amount: "1.5",
          coin: usdcCoin,
          config: evmBridgeConfig,
          connector,
        }),
      { wrapper: createQueryClientWrapper() },
    );

    await waitFor(() => expect(result.current.wallet.isError).toBe(true));

    expect(connector.getProvider).toHaveBeenCalledOnce();
    expect(result.current.wallet.error).toBe(walletError);
    expect(result.current.allowanceQuery.data).toBe(2n ** 256n - 1n);

    await expect(result.current.allowanceMutation.mutateAsync()).rejects.toThrow(
      "Wasn't able to approve",
    );
    await expect(result.current.deposit.mutateAsync()).rejects.toThrow("Wasn't able to deposit");

    expect(hookMocks.walletSwitchChain).not.toHaveBeenCalled();
    expect(hookMocks.walletWriteContract).not.toHaveBeenCalled();
    expect(hookMocks.publicClientWaitForTransactionReceipt).not.toHaveBeenCalled();
    expect(hookMocks.signingClientQueryWasmSmart).not.toHaveBeenCalled();
  });

  it("does not submit EVM deposits without a Dango signing client", async () => {
    const provider = {
      request: vi.fn().mockResolvedValue([evmRecipient]),
    };
    const connector = {
      getProvider: vi.fn().mockResolvedValue(provider),
      id: "metamask",
    };
    hookMocks.useSigningClient.mockReturnValue({
      data: undefined,
    });

    const { result } = renderHook(
      () =>
        useBridgeEvmDeposit({
          amount: "1.5",
          coin: usdcCoin,
          config: evmBridgeConfig,
          connector,
        }),
      { wrapper: createQueryClientWrapper() },
    );

    await waitFor(() => expect(result.current.wallet.data?.account.address).toBe(evmRecipient));

    await expect(result.current.deposit.mutateAsync()).rejects.toThrow("Wasn't able to deposit");

    expect(hookMocks.signingClientQueryWasmSmart).not.toHaveBeenCalled();
    expect(hookMocks.walletSwitchChain).not.toHaveBeenCalled();
    expect(hookMocks.walletWriteContract).not.toHaveBeenCalled();
    expect(hookMocks.publicClientWaitForTransactionReceipt).not.toHaveBeenCalled();
  });

  it("does not submit EVM deposits without a connected Dango account", async () => {
    const provider = {
      request: vi.fn().mockResolvedValue([evmRecipient]),
    };
    const connector = {
      getProvider: vi.fn().mockResolvedValue(provider),
      id: "metamask",
    };
    hookMocks.useAccount.mockReturnValue({
      account: undefined,
      isConnected: false,
    });

    const { result } = renderHook(
      () =>
        useBridgeEvmDeposit({
          amount: "1.5",
          coin: usdcCoin,
          config: evmBridgeConfig,
          connector,
        }),
      { wrapper: createQueryClientWrapper() },
    );

    await waitFor(() => expect(result.current.wallet.data?.account.address).toBe(evmRecipient));

    await expect(result.current.deposit.mutateAsync()).rejects.toThrow("Wasn't able to deposit");

    expect(hookMocks.signingClientQueryWasmSmart).not.toHaveBeenCalled();
    expect(hookMocks.walletSwitchChain).not.toHaveBeenCalled();
    expect(hookMocks.walletWriteContract).not.toHaveBeenCalled();
    expect(hookMocks.publicClientWaitForTransactionReceipt).not.toHaveBeenCalled();
  });

  it("does not switch chains or write EVM deposits when the Dango mailbox config query fails", async () => {
    const mailboxError = new Error("mailbox config unavailable");
    const provider = {
      request: vi.fn().mockResolvedValue([evmRecipient]),
    };
    const connector = {
      getProvider: vi.fn().mockResolvedValue(provider),
      id: "metamask",
    };
    hookMocks.signingClientQueryWasmSmart.mockRejectedValueOnce(mailboxError);

    const { result } = renderHook(
      () =>
        useBridgeEvmDeposit({
          amount: "1.5",
          coin: usdcCoin,
          config: evmBridgeConfig,
          connector,
        }),
      { wrapper: createQueryClientWrapper() },
    );

    await waitFor(() => expect(result.current.wallet.data?.account.address).toBe(evmRecipient));

    await expect(result.current.deposit.mutateAsync()).rejects.toThrow(
      "mailbox config unavailable",
    );

    expect(hookMocks.signingClientQueryWasmSmart).toHaveBeenCalledWith({
      contract: "0x6d61696c626f7800000000000000000000000000",
      msg: { config: {} },
    });
    expect(hookMocks.walletSwitchChain).not.toHaveBeenCalled();
    expect(hookMocks.walletWriteContract).not.toHaveBeenCalled();
    expect(hookMocks.publicClientWaitForTransactionReceipt).not.toHaveBeenCalled();
  });

  it("does not poll or refresh allowance when an EVM approval transaction is rejected", async () => {
    const approvalError = new Error("approval rejected");
    const provider = {
      request: vi.fn().mockResolvedValue([evmRecipient]),
    };
    const connector = {
      getProvider: vi.fn().mockResolvedValue(provider),
      id: "metamask",
    };
    hookMocks.walletWriteContract.mockRejectedValueOnce(approvalError);

    const { result } = renderHook(
      () =>
        useBridgeEvmDeposit({
          amount: "1.5",
          coin: usdcCoin,
          config: evmBridgeConfig,
          connector,
        }),
      { wrapper: createQueryClientWrapper() },
    );

    await waitFor(() =>
      expect(hookMocks.publicClientReadContract).toHaveBeenCalledWith(
        expect.objectContaining({
          address: tokenAddress,
          args: [evmRecipient, routerAddress],
          functionName: "allowance",
        }),
      ),
    );

    await expect(result.current.allowanceMutation.mutateAsync()).rejects.toThrow(
      "approval rejected",
    );

    expect(hookMocks.walletSwitchChain).toHaveBeenCalledWith({ id: 11155111 });
    expect(hookMocks.walletWriteContract).toHaveBeenCalledWith(
      expect.objectContaining({
        address: tokenAddress,
        args: [routerAddress, 1500000n],
        functionName: "approve",
      }),
    );
    expect(hookMocks.publicClientWaitForTransactionReceipt).not.toHaveBeenCalled();
    expect(hookMocks.publicClientReadContract).toHaveBeenCalledOnce();
  });

  it("does not poll for an EVM deposit receipt when the Hyperlane transfer is rejected", async () => {
    const depositError = new Error("deposit rejected");
    const provider = {
      request: vi.fn().mockResolvedValue([evmRecipient]),
    };
    const connector = {
      getProvider: vi.fn().mockResolvedValue(provider),
      id: "metamask",
    };
    hookMocks.walletWriteContract.mockRejectedValueOnce(depositError);

    const { result } = renderHook(
      () =>
        useBridgeEvmDeposit({
          amount: "1.5",
          coin: usdcCoin,
          config: evmBridgeConfig,
          connector,
        }),
      { wrapper: createQueryClientWrapper() },
    );

    await waitFor(() => expect(result.current.wallet.data?.account.address).toBe(evmRecipient));

    await expect(result.current.deposit.mutateAsync()).rejects.toThrow("deposit rejected");

    expect(hookMocks.signingClientQueryWasmSmart).toHaveBeenCalledWith({
      contract: "0x6d61696c626f7800000000000000000000000000",
      msg: { config: {} },
    });
    expect(hookMocks.walletSwitchChain).toHaveBeenCalledWith({ id: 11155111 });
    expect(hookMocks.walletWriteContract).toHaveBeenCalledWith(
      expect.objectContaining({
        address: routerAddress,
        args: [999, `0x${toAddr32(evmAccount)}`, 1500000n],
        functionName: "transferRemote",
        value: 77n,
      }),
    );
    expect(hookMocks.publicClientWaitForTransactionReceipt).not.toHaveBeenCalled();
  });

  it("submits EVM deposits through the Hyperlane router with mailbox domain and protocol fee", async () => {
    const provider = {
      request: vi.fn().mockResolvedValue([evmRecipient]),
    };
    const connector = {
      getProvider: vi.fn().mockResolvedValue(provider),
      id: "metamask",
    };

    const { result } = renderHook(
      () =>
        useBridgeEvmDeposit({
          amount: "1.5",
          coin: usdcCoin,
          config: evmBridgeConfig,
          connector,
        }),
      { wrapper: createQueryClientWrapper() },
    );

    await waitFor(() => expect(result.current.wallet.data?.account.address).toBe(evmRecipient));

    await act(async () => {
      await result.current.deposit.mutateAsync();
    });

    expect(hookMocks.signingClientQueryWasmSmart).toHaveBeenCalledWith({
      contract: "0x6d61696c626f7800000000000000000000000000",
      msg: { config: {} },
    });
    expect(hookMocks.walletSwitchChain).toHaveBeenCalledWith({ id: 11155111 });
    expect(hookMocks.walletWriteContract).toHaveBeenCalledWith(
      expect.objectContaining({
        address: routerAddress,
        args: [999, `0x${toAddr32(evmAccount)}`, 1500000n],
        functionName: "transferRemote",
        value: 77n,
      }),
    );
    expect(hookMocks.publicClientWaitForTransactionReceipt).toHaveBeenCalledWith({
      hash: "0xtransaction",
    });
  });

  it("preserves zero protocol fees from the backend bridge config in EVM deposits", async () => {
    const provider = {
      request: vi.fn().mockResolvedValue([evmRecipient]),
    };
    const connector = {
      getProvider: vi.fn().mockResolvedValue(provider),
      id: "metamask",
    };
    const zeroFeeBridgeConfig = {
      ...evmBridgeConfig,
      bridger: {
        ...evmBridgeConfig.bridger,
        protocolFee: 0,
      },
    };

    const { result } = renderHook(
      () =>
        useBridgeEvmDeposit({
          amount: "1.5",
          coin: usdcCoin,
          config: zeroFeeBridgeConfig,
          connector,
        }),
      { wrapper: createQueryClientWrapper() },
    );

    await waitFor(() => expect(result.current.wallet.data?.account.address).toBe(evmRecipient));

    await act(async () => {
      await result.current.deposit.mutateAsync();
    });

    expect(hookMocks.walletWriteContract).toHaveBeenCalledWith(
      expect.objectContaining({
        address: routerAddress,
        args: [999, `0x${toAddr32(evmAccount)}`, 1500000n],
        functionName: "transferRemote",
        value: 0n,
      }),
    );
    expect(hookMocks.publicClientWaitForTransactionReceipt).toHaveBeenCalledWith({
      hash: "0xtransaction",
    });
  });

  it("submits native EVM deposits with amount plus protocol fee and skips ERC20 allowance reads", async () => {
    const provider = {
      request: vi.fn().mockResolvedValue([evmRecipient]),
    };
    const connector = {
      getProvider: vi.fn().mockResolvedValue(provider),
      id: "metamask",
    };

    const { result } = renderHook(
      () =>
        useBridgeEvmDeposit({
          amount: "0.25",
          coin: ethCoin,
          config: nativeEvmBridgeConfig,
          connector,
        }),
      { wrapper: createQueryClientWrapper() },
    );

    await waitFor(() => expect(result.current.wallet.data?.account.address).toBe(evmRecipient));
    await waitFor(() => expect(result.current.allowanceQuery.data).toBe(2n ** 256n - 1n));

    expect(hookMocks.publicClientReadContract).not.toHaveBeenCalled();

    await expect(result.current.allowanceMutation.mutateAsync()).rejects.toThrow(
      "Wasn't able to approve",
    );
    expect(hookMocks.walletSwitchChain).not.toHaveBeenCalled();
    expect(hookMocks.walletWriteContract).not.toHaveBeenCalled();
    expect(hookMocks.publicClientWaitForTransactionReceipt).not.toHaveBeenCalled();

    await act(async () => {
      await result.current.deposit.mutateAsync();
    });

    expect(hookMocks.walletSwitchChain).toHaveBeenCalledWith({ id: 11155111 });
    expect(hookMocks.walletWriteContract).toHaveBeenCalledWith(
      expect.objectContaining({
        address: nativeRouterAddress,
        args: [999, `0x${toAddr32(evmAccount)}`, 250000000000000000n],
        functionName: "transferRemote",
        value: 250000000000000077n,
      }),
    );
    expect(hookMocks.publicClientWaitForTransactionReceipt).toHaveBeenCalledWith({
      hash: "0xtransaction",
    });
  });
});
