import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { toAddr32 } from "@left-curve/sdk/hyperlane";
import { useBridgeState } from "../../../store/src/hooks/useBridgeState";
import { useBridgeWithdraw } from "../../../store/src/hooks/useBridgeWithdraw";
import { useEvmBalances } from "../../../store/src/hooks/useEvmBalances";
import { createQueryClientWrapper } from "./utils/query-client";

const hookMocks = vi.hoisted(() => ({
  createPublicClient: vi.fn(),
  getWithdrawalFee: vi.fn(),
  http: vi.fn(),
  publicClientGetBalance: vi.fn(),
  publicClientReadContract: vi.fn(),
  transferRemote: vi.fn(),
  useAccount: vi.fn(),
  useConfig: vi.fn(),
  usePublicClient: vi.fn(),
  useSigningClient: vi.fn(),
}));

vi.mock("viem", () => ({
  createPublicClient: hookMocks.createPublicClient,
  http: hookMocks.http,
}));

vi.mock("@left-curve/sdk/actions", () => ({
  getWithdrawalFee: hookMocks.getWithdrawalFee,
  transferRemote: hookMocks.transferRemote,
}));

vi.mock("../../../store/src/hooks/useAccount.js", () => ({
  useAccount: hookMocks.useAccount,
}));

vi.mock("../../../store/src/hooks/useConfig.js", () => ({
  useConfig: hookMocks.useConfig,
}));

vi.mock("../../../store/src/hooks/usePublicClient.js", () => ({
  usePublicClient: hookMocks.usePublicClient,
}));

vi.mock("../../../store/src/hooks/useSigningClient.js", () => ({
  useSigningClient: hookMocks.useSigningClient,
}));

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
const evmAccount = "0x0000000000000000000000000000000000000abc";
const evmRecipient = "0x4444444444444444444444444444444444444444";

const bridger = {
  chain_id: 11155111,
  hyperlane_deployments: {
    mailbox: "0x5555555555555555555555555555555555555555",
    static_message_id_multisig_ism_factory: "0x6666666666666666666666666666666666666666",
  },
  hyperlane_domain: 17,
  hyperlane_protocol_fee: 77,
  infura_rpc_url: "https://sepolia.example",
  ism: {
    static_message_id_multisig_ism: {
      threshold: 1,
      validators: ["0xvalidator"],
    },
  },
  proxy_admin_address: "0x7777777777777777777777777777777777777777",
  warp_routes: [
    {
      proxy_address: routerAddress,
      symbol: "USDC",
      warp_route_type: {
        erc20_collateral: tokenAddress,
      },
    },
    {
      proxy_address: nativeRouterAddress,
      symbol: "ETH",
      warp_route_type: "native",
    },
  ],
};

const mainnetBridger = {
  ...bridger,
  chain_id: 1,
  hyperlane_domain: 1,
  infura_rpc_url: "https://mainnet.example",
  warp_routes: [
    {
      proxy_address: mainnetRouterAddress,
      symbol: "USDC",
      warp_route_type: {
        erc20_collateral: mainnetTokenAddress,
      },
    },
  ],
};

const bridgeEnvConfig = {
  evm: {
    "11155111": bridger,
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
    domain: bridger.hyperlane_domain,
    remote: {
      warp: {
        contract: toAddr32(routerAddress),
        domain: bridger.hyperlane_domain,
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
    domain: bridger.hyperlane_domain,
    remote: {
      warp: {
        contract: toAddr32(nativeRouterAddress),
        domain: bridger.hyperlane_domain,
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
  const signingClient = { id: "signing-client" };
  const evmPublicClient = {
    getBalance: hookMocks.publicClientGetBalance,
    readContract: hookMocks.publicClientReadContract,
  };

  beforeEach(() => {
    hookMocks.useAccount.mockReturnValue({
      account: { address: evmAccount },
      isConnected: true,
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
    hookMocks.usePublicClient.mockReturnValue(dangoPublicClient);
    hookMocks.useSigningClient.mockReturnValue({
      data: signingClient,
    });
    hookMocks.http.mockImplementation((url: string, options?: Record<string, unknown>) => ({
      ...options,
      type: "http",
      url,
    }));
    hookMocks.createPublicClient.mockReturnValue(evmPublicClient);
    hookMocks.publicClientGetBalance.mockResolvedValue(2000000000000000000n);
    hookMocks.publicClientReadContract.mockResolvedValue(100n);
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("exposes only bridgeable EVM coins from the configured coin store", () => {
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
          action: "withdraw",
          config: bridgeEnvConfig,
          controllers,
        }),
      { wrapper: createQueryClientWrapper() },
    );

    expect(result.current.coins).toEqual([ethCoin, usdcCoin]);

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
          action: "withdraw",
          config: bridgeEnvConfig,
          controllers,
        }),
      { wrapper: createQueryClientWrapper() },
    );

    expect(result.current.networks).toEqual([
      { id: "11155111", name: "Sepolia Network", time: "5-30 mins" },
    ]);

    act(() => result.current.changeCoin(usdcCoin.denom));
    act(() => result.current.setNetwork("11155111"));

    await waitFor(() => expect(result.current.config?.router).toBeDefined());

    expect(result.current.coin).toEqual(usdcCoin);
    expect(result.current.config?.router).toEqual({
      address: routerAddress,
      coin: tokenAddress,
      domain: bridger.hyperlane_domain,
      remote: {
        warp: {
          contract: toAddr32(routerAddress),
          domain: bridger.hyperlane_domain,
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
          action: "withdraw",
          config: bridgeEnvConfig,
          controllers,
        }),
      { wrapper: createQueryClientWrapper() },
    );

    expect(result.current.networks).toEqual([
      { id: "11155111", name: "Sepolia Network", time: "5-30 mins" },
    ]);

    act(() => result.current.changeCoin(usdcCoin.denom));
    act(() => result.current.setNetwork("11155111"));

    await waitFor(() => expect(result.current.config?.router).toBeDefined());

    expect(result.current.config?.chain).toEqual(expect.objectContaining({ id: 11155111 }));
    expect(result.current.config?.bridger).toEqual(bridger);
    expect(result.current.config?.router).toEqual({
      address: routerAddress,
      coin: tokenAddress,
      domain: bridger.hyperlane_domain,
      remote: {
        warp: {
          contract: toAddr32(routerAddress),
          domain: bridger.hyperlane_domain,
        },
      },
    });
  });

  it("derives native EVM router config from the selected ETH warp route", async () => {
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

    act(() => result.current.changeCoin(ethCoin.denom));
    act(() => result.current.setNetwork("11155111"));

    await waitFor(() => expect(result.current.config?.router).toBeDefined());

    expect(result.current.coin).toEqual(ethCoin);
    expect(result.current.config?.router).toEqual({
      address: nativeRouterAddress,
      coin: "native",
      domain: bridger.hyperlane_domain,
      remote: {
        warp: {
          contract: toAddr32(nativeRouterAddress),
          domain: bridger.hyperlane_domain,
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
          action: "withdraw",
          config: {
            evm: {
              "1": mainnetBridger,
            },
          },
          controllers,
        }),
      { wrapper: createQueryClientWrapper() },
    );

    expect(result.current.networks).toEqual([
      { id: "1", name: "Ethereum Network", time: "6 blocks | 1-3 mins" },
    ]);

    act(() => result.current.changeCoin(usdcCoin.denom));
    act(() => result.current.setNetwork("1"));

    await waitFor(() => expect(result.current.config?.router).toBeDefined());

    expect(result.current.config?.chain).toEqual(expect.objectContaining({ id: 1 }));
    expect(result.current.config?.bridger).toEqual(mainnetBridger);
    expect(result.current.config?.router).toEqual({
      address: mainnetRouterAddress,
      coin: mainnetTokenAddress,
      domain: mainnetBridger.hyperlane_domain,
      remote: {
        warp: {
          contract: toAddr32(mainnetRouterAddress),
          domain: mainnetBridger.hyperlane_domain,
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
          action: "withdraw",
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
          action: "withdraw",
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

  it("resets bridge form state when action or connection changes", async () => {
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

    act(() => result.current.changeCoin(usdcCoin.denom));
    act(() => result.current.setNetwork("11155111"));
    await waitFor(() => expect(result.current.config?.router).toBeDefined());

    rerender({ action: "withdraw" });

    await waitFor(() => expect(controllers.reset).toHaveBeenCalledTimes(2));
    expect(result.current.coin).toBeUndefined();
    expect(result.current.network).toBeUndefined();
    expect(result.current.config).toBeUndefined();

    hookMocks.useAccount.mockReturnValue({
      account: undefined,
      isConnected: false,
    });
    rerender({ action: "withdraw" });

    await waitFor(() => expect(controllers.reset).toHaveBeenCalledTimes(3));
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
          domain: bridger.hyperlane_domain,
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
          domain: bridger.hyperlane_domain,
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
          domain: bridger.hyperlane_domain,
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
          domain: bridger.hyperlane_domain,
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
});
