import { cleanup, renderHook, waitFor } from "@testing-library/react";
import { Suspense, type ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { useAccountInfo } from "../../../store/src/hooks/useAccountInfo";
import { useAppConfig } from "../../../store/src/hooks/useAppConfig";
import { useBalances } from "../../../store/src/hooks/useBalances";
import { useBlock } from "../../../store/src/hooks/useBlock";
import { useConnectorClient } from "../../../store/src/hooks/useConnectorClient";
import { useEvmBalances } from "../../../store/src/hooks/useEvmBalances";
import { usePrices } from "../../../store/src/hooks/usePrices";
import { createQueryClientWrapper } from "./utils/query-client";

const sdkActionMocks = vi.hoisted(() => ({
  getAccountInfo: vi.fn(),
  getBalances: vi.fn(),
  queryBlock: vi.fn(),
}));

const hookMocks = vi.hoisted(() => ({
  createPublicClient: vi.fn(),
  evmGetBalance: vi.fn(),
  evmReadContract: vi.fn(),
  http: vi.fn(),
  useConfig: vi.fn(),
  usePublicClient: vi.fn(),
}));

vi.mock("@left-curve/sdk/actions", () => ({
  getAccountInfo: sdkActionMocks.getAccountInfo,
  getBalances: sdkActionMocks.getBalances,
  queryBlock: sdkActionMocks.queryBlock,
}));

vi.mock("@left-curve/sdk/hyperlane", () => ({
  ERC20_ABI: [{ name: "balanceOf", type: "function" }],
  INFURA_URLS: {
    11155111: "https://sepolia.infura.test",
  },
}));

vi.mock("viem", () => ({
  createPublicClient: hookMocks.createPublicClient,
  http: hookMocks.http,
}));

vi.mock("../../../store/src/hooks/useConfig.js", () => ({
  useConfig: hookMocks.useConfig,
}));

vi.mock("../../../store/src/hooks/usePublicClient.js", () => ({
  usePublicClient: hookMocks.usePublicClient,
}));

const publicClient = {
  getAppConfig: vi.fn(),
  getCodeHash: vi.fn(),
  getPerpsPairParams: vi.fn(),
  getPerpsParam: vi.fn(),
  getPrices: vi.fn(),
  uid: "public-client",
};

const config = {
  chain: {
    id: "dango-dev-1",
  },
  coins: {
    byDenom: {
      "bridge/btc": {
        decimals: 8,
        denom: "bridge/btc",
        symbol: "BTC",
      },
      "bridge/usdc": {
        decimals: 6,
        denom: "bridge/usdc",
        symbol: "USDC",
      },
    },
  },
  getClient: vi.fn(() => ({
    extend: vi.fn(() => publicClient),
  })),
  state: {
    connectors: new Map(),
    current: undefined as string | undefined,
  },
};

function createSuspenseQueryClientWrapper() {
  const QueryClientWrapper = createQueryClientWrapper();

  return function SuspenseQueryClientWrapper({ children }: { children: ReactNode }) {
    return (
      <QueryClientWrapper>
        <Suspense fallback={null}>{children}</Suspense>
      </QueryClientWrapper>
    );
  };
}

describe("core client hooks", () => {
  beforeEach(() => {
    config.state.connectors = new Map();
    config.state.current = undefined;
    hookMocks.useConfig.mockReturnValue(config);
    hookMocks.usePublicClient.mockReturnValue(publicClient);
    hookMocks.http.mockImplementation((url: string, options: unknown) => ({ options, url }));
    hookMocks.createPublicClient.mockReturnValue({
      getBalance: hookMocks.evmGetBalance,
      readContract: hookMocks.evmReadContract,
    });
    publicClient.getAppConfig.mockResolvedValue({
      addresses: {
        accountFactory: "0x6163636f756e74666163746f7279000000000000",
        dex: "0x6465780000000000000000000000000000000000",
        gateway: "0x6761746577617900000000000000000000000000",
        hyperlane: {
          ism: "0x69736d0000000000000000000000000000000000",
          mailbox: "0x6d61696c626f7800000000000000000000000000",
          va: "0x7661000000000000000000000000000000000000",
        },
        lending: "0x6c656e64696e6700000000000000000000000000",
        oracle: "0x6f7261636c650000000000000000000000000000",
        perps: "0x7065727073000000000000000000000000000000",
        taxman: "0x7461786d616e0000000000000000000000000000",
        warp: "0x7761727000000000000000000000000000000000",
      },
      makerFeeRate: "0.001",
      maxLiquidationBonus: "0.05",
      minLiquidationBonus: "0.01",
      minimumDeposit: {
        "bridge/usdc": "1000000",
      },
      takerFeeRate: "0.002",
      targetUtilizationRate: "0.8",
    });
    publicClient.getCodeHash.mockResolvedValue("0x636f646568617368");
    publicClient.getPerpsPairParams.mockResolvedValue({
      "perp/btcusd": {
        enabled: true,
      },
    });
    publicClient.getPerpsParam.mockResolvedValue({
      maxPositionLimit: "1000000",
    });
    publicClient.getPrices.mockResolvedValue({
      "bridge/btc": {
        humanizedPrice: "50000",
        timestamp: "1",
      },
      "bridge/usdc": {
        humanizedPrice: "1",
        timestamp: "1",
      },
    });
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("queries balances only when an address is available and preserves the query key contract", async () => {
    sdkActionMocks.getBalances.mockResolvedValue({ "bridge/usdc": "42" });

    const disabled = renderHook(() => useBalances({ address: undefined }), {
      wrapper: createQueryClientWrapper(),
    });

    expect(disabled.result.current.queryKey).toEqual([
      "getBalances",
      {
        address: undefined,
      },
    ]);
    expect(sdkActionMocks.getBalances).not.toHaveBeenCalled();

    const { result } = renderHook(
      () =>
        useBalances({
          address: "0x6163636f756e7400000000000000000000000000",
          height: 12,
          query: {
            select: (balances) => balances["bridge/usdc"],
          },
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(() => expect(result.current.data).toBe("42"));

    expect(sdkActionMocks.getBalances).toHaveBeenCalledWith(publicClient, {
      address: "0x6163636f756e7400000000000000000000000000",
      height: 12,
    });
    expect(result.current.queryKey).toEqual([
      "getBalances",
      {
        address: "0x6163636f756e7400000000000000000000000000",
        height: 12,
      },
    ]);
  });

  it("preserves zero balance heights in backend balance queries", async () => {
    sdkActionMocks.getBalances.mockResolvedValue({ "bridge/usdc": "0" });

    const { result } = renderHook(
      () =>
        useBalances({
          address: "0x7a65726f2d62616c616e63657300000000000000",
          height: 0,
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(() => expect(result.current.data).toEqual({ "bridge/usdc": "0" }));

    expect(sdkActionMocks.getBalances).toHaveBeenCalledWith(publicClient, {
      address: "0x7a65726f2d62616c616e63657300000000000000",
      height: 0,
    });
    expect(result.current.queryKey).toEqual([
      "getBalances",
      {
        address: "0x7a65726f2d62616c616e63657300000000000000",
        height: 0,
      },
    ]);
  });

  it("preserves paginated balance parameters in backend balance queries", async () => {
    sdkActionMocks.getBalances.mockResolvedValue({ "bridge/atom": "100" });

    const { result } = renderHook(
      () =>
        useBalances({
          address: "0x706167696e617465642d62616c616e63657300",
          height: 64,
          limit: 25,
          startAfter: "bridge/usdc",
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(() => expect(result.current.data).toEqual({ "bridge/atom": "100" }));

    expect(sdkActionMocks.getBalances).toHaveBeenCalledWith(publicClient, {
      address: "0x706167696e617465642d62616c616e63657300",
      height: 64,
      limit: 25,
      startAfter: "bridge/usdc",
    });
    expect(result.current.queryKey).toEqual([
      "getBalances",
      {
        address: "0x706167696e617465642d62616c616e63657300",
        height: 64,
        limit: 25,
        startAfter: "bridge/usdc",
      },
    ]);
  });

  it("surfaces balance query failures from the SDK action", async () => {
    const queryError = new Error("balances unavailable");
    sdkActionMocks.getBalances.mockRejectedValueOnce(queryError);

    const { result } = renderHook(
      () =>
        useBalances({
          address: "0x62616c616e636573000000000000000000000000",
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(() => expect(result.current.isError).toBe(true));

    expect(sdkActionMocks.getBalances).toHaveBeenCalledWith(publicClient, {
      address: "0x62616c616e636573000000000000000000000000",
    });
    expect(result.current.error).toBe(queryError);
    expect(result.current.data).toBeUndefined();
  });

  it("queries account info and block data through SDK action adapters", async () => {
    sdkActionMocks.getAccountInfo.mockResolvedValue({
      address: "0x6163636f756e7400000000000000000000000000",
      params: { spot: {} },
    });
    sdkActionMocks.queryBlock.mockResolvedValue({
      blockHeight: 99,
      createdAt: "2026-06-08T12:00:00Z",
      hash: "block-hash",
    });

    const accountInfo = renderHook(
      () =>
        useAccountInfo({
          address: "0x6163636f756e7400000000000000000000000000",
          chainId: "dango-dev-1",
          height: 21,
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(() =>
      expect(accountInfo.result.current.data).toMatchObject({
        address: "0x6163636f756e7400000000000000000000000000",
      }),
    );

    expect(sdkActionMocks.getAccountInfo).toHaveBeenCalledWith(publicClient, {
      address: "0x6163636f756e7400000000000000000000000000",
      chainId: "dango-dev-1",
      height: 21,
    });
    expect(accountInfo.result.current.queryKey).toEqual([
      "getAccountInfo",
      {
        address: "0x6163636f756e7400000000000000000000000000",
        chainId: "dango-dev-1",
        height: 21,
      },
    ]);

    const block = renderHook(() => useBlock({ height: 99 }), {
      wrapper: createQueryClientWrapper(),
    });

    await waitFor(() =>
      expect(block.result.current.data).toEqual({
        hash: "block-hash",
        height: "99",
        timestamp: "2026-06-08T12:00:00Z",
      }),
    );

    expect(sdkActionMocks.queryBlock).toHaveBeenCalledWith(publicClient, {
      height: 99,
    });
    expect(block.result.current.queryKey).toEqual([
      "GetBlock",
      {
        height: 99,
      },
    ]);
  });

  it("surfaces account info and block query failures from SDK action adapters", async () => {
    const accountInfoError = new Error("account info unavailable");
    sdkActionMocks.getAccountInfo.mockRejectedValueOnce(accountInfoError);

    const accountInfo = renderHook(
      () =>
        useAccountInfo({
          address: "0x6163636f756e7400000000000000000000000000",
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(() => expect(accountInfo.result.current.isError).toBe(true));

    expect(sdkActionMocks.getAccountInfo).toHaveBeenCalledWith(publicClient, {
      address: "0x6163636f756e7400000000000000000000000000",
    });
    expect(accountInfo.result.current.error).toBe(accountInfoError);
    expect(accountInfo.result.current.data).toBeUndefined();

    const blockError = new Error("block unavailable");
    sdkActionMocks.queryBlock.mockRejectedValueOnce(blockError);

    const block = renderHook(() => useBlock({ height: 120 }), {
      wrapper: createQueryClientWrapper(),
    });

    await waitFor(() => expect(block.result.current.isError).toBe(true));

    expect(sdkActionMocks.queryBlock).toHaveBeenCalledWith(publicClient, {
      height: 120,
    });
    expect(block.result.current.error).toBe(blockError);
    expect(block.result.current.data).toBeUndefined();
  });

  it("preserves zero block heights in backend block queries", async () => {
    sdkActionMocks.queryBlock.mockResolvedValue({
      blockHeight: 0,
      createdAt: "2026-06-08T00:00:00Z",
      hash: "genesis-block-hash",
    });

    const { result } = renderHook(() => useBlock({ height: 0 }), {
      wrapper: createQueryClientWrapper(),
    });

    await waitFor(() =>
      expect(result.current.data).toEqual({
        hash: "genesis-block-hash",
        height: "0",
        timestamp: "2026-06-08T00:00:00Z",
      }),
    );

    expect(sdkActionMocks.queryBlock).toHaveBeenCalledWith(publicClient, {
      height: 0,
    });
    expect(result.current.queryKey).toEqual([
      "GetBlock",
      {
        height: 0,
      },
    ]);
  });

  it("preserves zero account-info heights in backend account queries", async () => {
    sdkActionMocks.getAccountInfo.mockResolvedValue({
      address: "0x7a65726f2d686569676874000000000000000000",
      params: { spot: {} },
    });

    const { result } = renderHook(
      () =>
        useAccountInfo({
          address: "0x7a65726f2d686569676874000000000000000000",
          height: 0,
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(() =>
      expect(result.current.data).toMatchObject({
        address: "0x7a65726f2d686569676874000000000000000000",
      }),
    );

    expect(sdkActionMocks.getAccountInfo).toHaveBeenCalledWith(publicClient, {
      address: "0x7a65726f2d686569676874000000000000000000",
      height: 0,
    });
    expect(result.current.queryKey).toEqual([
      "getAccountInfo",
      {
        address: "0x7a65726f2d686569676874000000000000000000",
        height: 0,
      },
    ]);
  });

  it("keeps account info and block queries disabled when the caller opts out", () => {
    const accountInfo = renderHook(
      () =>
        useAccountInfo({
          address: "0x6163636f756e7400000000000000000000000000",
          query: {
            enabled: false,
          },
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    expect(accountInfo.result.current.queryKey).toEqual([
      "getAccountInfo",
      {
        address: "0x6163636f756e7400000000000000000000000000",
      },
    ]);
    expect(sdkActionMocks.getAccountInfo).not.toHaveBeenCalled();

    const block = renderHook(
      () =>
        useBlock({
          height: 123,
          query: {
            enabled: false,
          },
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    expect(block.result.current.queryKey).toEqual([
      "GetBlock",
      {
        height: 123,
      },
    ]);
    expect(sdkActionMocks.queryBlock).not.toHaveBeenCalled();
  });

  it("loads app config through the public client and exposes flattened address aliases", async () => {
    const { result } = renderHook(
      () =>
        useAppConfig({
          scopeKey: "portal",
          query: {
            select: (appConfig) => ({
              accountFactoryCodeHash: appConfig.accountFactory.codeHash,
              addresses: appConfig.addresses,
              perpsPairs: appConfig.perpsPairs,
              perpsParam: appConfig.perpsParam,
            }),
          },
        }),
      {
        wrapper: createSuspenseQueryClientWrapper(),
      },
    );

    await waitFor(() =>
      expect(result.current.data).toMatchObject({
        accountFactoryCodeHash: "0x636f646568617368",
        addresses: {
          "0x6163636f756e74666163746f7279000000000000": "accountFactory",
          "0x6d61696c626f7800000000000000000000000000": "mailbox",
          accountFactory: "0x6163636f756e74666163746f7279000000000000",
          mailbox: "0x6d61696c626f7800000000000000000000000000",
        },
        perpsPairs: {
          "perp/btcusd": {
            enabled: true,
          },
        },
        perpsParam: {
          maxPositionLimit: "1000000",
        },
      }),
    );

    expect(config.getClient).toHaveBeenCalledOnce();
    expect(publicClient.getAppConfig).toHaveBeenCalledOnce();
    expect(publicClient.getCodeHash).toHaveBeenCalledOnce();
    expect(publicClient.getPerpsPairParams).toHaveBeenCalledOnce();
    expect(publicClient.getPerpsParam).toHaveBeenCalledOnce();
    expect(result.current.data?.addresses).not.toHaveProperty("hyperlane");
  });

  it("resolves connector clients from the current connector or an explicit connector UID", async () => {
    const currentClient = { id: "current-client" };
    const explicitClient = { id: "explicit-client" };
    config.state.current = "current";
    config.state.connectors = new Map([
      [
        "current",
        {
          connector: {
            getClient: vi.fn(async () => currentClient),
          },
        },
      ],
      [
        "explicit",
        {
          connector: {
            getClient: vi.fn(async () => explicitClient),
          },
        },
      ],
    ]);

    const current = renderHook(() => useConnectorClient(), {
      wrapper: createQueryClientWrapper(),
    });

    await waitFor(() => expect(current.result.current.data).toBe(currentClient));
    expect(current.result.current.queryKey).toEqual([
      "connectorClient",
      {
        connectorUId: undefined,
        state: config.state,
      },
    ]);

    const explicit = renderHook(() => useConnectorClient({ connectorUId: "explicit" }), {
      wrapper: createQueryClientWrapper(),
    });

    await waitFor(() => expect(explicit.result.current.data).toBe(explicitClient));
    expect(explicit.result.current.queryKey).toEqual([
      "connectorClient",
      {
        connectorUId: "explicit",
        state: config.state,
      },
    ]);
  });

  it("resolves an explicit connector client without a current connector", async () => {
    const walletClient = { id: "wallet-client" };
    const getClient = vi.fn(async () => walletClient);
    config.state.current = undefined;
    config.state.connectors = new Map([
      [
        "wallet",
        {
          connector: {
            getClient,
          },
        },
      ],
    ]);

    const { result } = renderHook(() => useConnectorClient({ connectorUId: "wallet" }), {
      wrapper: createQueryClientWrapper(),
    });

    await waitFor(() => expect(result.current.data).toBe(walletClient));
    expect(getClient).toHaveBeenCalledOnce();
    expect(result.current.queryKey).toEqual([
      "connectorClient",
      {
        connectorUId: "wallet",
        state: config.state,
      },
    ]);
  });

  it("keeps connector client resolution disabled when the caller opts out", () => {
    const getClient = vi.fn(async () => ({ id: "current-client" }));
    config.state.current = "current";
    config.state.connectors = new Map([
      [
        "current",
        {
          connector: {
            getClient,
          },
        },
      ],
    ]);

    const { result } = renderHook(
      () =>
        useConnectorClient({
          query: {
            enabled: false,
          },
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    expect(result.current.queryKey).toEqual([
      "connectorClient",
      {
        connectorUId: undefined,
        state: config.state,
      },
    ]);
    expect(getClient).not.toHaveBeenCalled();
  });

  it("surfaces explicit connector client failures without using the current connector", async () => {
    const connectorError = new Error("session connector unavailable");
    const currentGetClient = vi.fn(async () => ({ id: "current-client" }));
    const sessionGetClient = vi.fn(async () => {
      throw connectorError;
    });
    config.state.current = "current";
    config.state.connectors = new Map([
      [
        "current",
        {
          connector: {
            getClient: currentGetClient,
          },
        },
      ],
      [
        "session",
        {
          connector: {
            getClient: sessionGetClient,
          },
        },
      ],
    ]);

    const { result } = renderHook(() => useConnectorClient({ connectorUId: "session" }), {
      wrapper: createQueryClientWrapper(),
    });

    await waitFor(() => expect(result.current.isError).toBe(true));

    expect(result.current.error).toBe(connectorError);
    expect(sessionGetClient).toHaveBeenCalledOnce();
    expect(currentGetClient).not.toHaveBeenCalled();
    expect(result.current.queryKey).toEqual([
      "connectorClient",
      {
        connectorUId: "session",
        state: config.state,
      },
    ]);
  });

  it("surfaces connector client lookup errors for missing current or explicit connections", async () => {
    const missingCurrent = renderHook(() => useConnectorClient(), {
      wrapper: createQueryClientWrapper(),
    });

    await waitFor(() => expect(missingCurrent.result.current.isError).toBe(true));
    expect(missingCurrent.result.current.error).toEqual(
      new Error("No connector found for current chain"),
    );
    expect(missingCurrent.result.current.queryKey).toEqual([
      "connectorClient",
      {
        connectorUId: undefined,
        state: config.state,
      },
    ]);

    config.state.current = "wallet";
    config.state.connectors = new Map();

    const missingConnection = renderHook(() => useConnectorClient({ connectorUId: "wallet" }), {
      wrapper: createQueryClientWrapper(),
    });

    await waitFor(() => expect(missingConnection.result.current.isError).toBe(true));
    expect(missingConnection.result.current.error).toEqual(new Error("No connection found"));
    expect(missingConnection.result.current.queryKey).toEqual([
      "connectorClient",
      {
        connectorUId: "wallet",
        state: config.state,
      },
    ]);
  });

  it("fetches EVM native and ERC20 balances using the configured bridge chain", async () => {
    hookMocks.evmGetBalance.mockResolvedValue(123n);
    hookMocks.evmReadContract.mockResolvedValueOnce(456n).mockResolvedValueOnce(789n);

    const chain = {
      contracts: {
        erc20: [
          {
            address: "0x1111111111111111111111111111111111111111",
            targetDenom: "bridge/usdc",
          },
          {
            address: "0x2222222222222222222222222222222222222222",
            targetDenom: "bridge/wbtc",
          },
        ],
      },
      id: 11155111,
      name: "Sepolia",
    };

    const { result } = renderHook(
      () =>
        useEvmBalances({
          address: "0xabc0000000000000000000000000000000000000",
          chain,
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(() =>
      expect(result.current.data).toEqual({
        "bridge/eth": "123",
        "bridge/usdc": "456",
        "bridge/wbtc": "789",
      }),
    );

    expect(hookMocks.http).toHaveBeenCalledWith("https://sepolia.infura.test", {
      batch: true,
    });
    expect(hookMocks.createPublicClient).toHaveBeenCalledWith({
      chain,
      transport: {
        options: {
          batch: true,
        },
        url: "https://sepolia.infura.test",
      },
    });
    expect(hookMocks.evmGetBalance).toHaveBeenCalledWith({
      address: "0xabc0000000000000000000000000000000000000",
    });
    expect(hookMocks.evmReadContract).toHaveBeenCalledWith(
      expect.objectContaining({
        address: "0x1111111111111111111111111111111111111111",
        args: ["0xabc0000000000000000000000000000000000000"],
        functionName: "balanceOf",
      }),
    );

    cleanup();
    vi.clearAllMocks();

    renderHook(
      () =>
        useEvmBalances({
          address: undefined,
          chain,
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    expect(hookMocks.createPublicClient).not.toHaveBeenCalled();
  });

  it("surfaces EVM balance query failures before ERC20 balance reads", async () => {
    const chain = {
      contracts: {
        erc20: [
          {
            address: "0x1111111111111111111111111111111111111111",
            targetDenom: "bridge/usdc",
          },
        ],
      },
      id: 11155111,
      name: "Sepolia",
    };
    const queryError = new Error("native balance unavailable");
    hookMocks.evmGetBalance.mockRejectedValueOnce(queryError);

    const { result } = renderHook(
      () =>
        useEvmBalances({
          address: "0xabc0000000000000000000000000000000000000",
          chain,
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(() => expect(result.current.isError).toBe(true));

    expect(hookMocks.evmGetBalance).toHaveBeenCalledWith({
      address: "0xabc0000000000000000000000000000000000000",
    });
    expect(hookMocks.evmReadContract).not.toHaveBeenCalled();
    expect(result.current.error).toBe(queryError);
    expect(result.current.data).toBeUndefined();
  });

  it("fetches only the native EVM balance when the bridge chain has no ERC20 contracts", async () => {
    hookMocks.evmGetBalance.mockResolvedValue(987n);
    const chain = {
      contracts: {
        erc20: [],
      },
      id: 11155111,
      name: "Sepolia",
    };

    const { result } = renderHook(
      () =>
        useEvmBalances({
          address: "0xdef0000000000000000000000000000000000000",
          chain,
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(() =>
      expect(result.current.data).toEqual({
        "bridge/eth": "987",
      }),
    );

    expect(hookMocks.createPublicClient).toHaveBeenCalledWith({
      chain,
      transport: {
        options: {
          batch: true,
        },
        url: "https://sepolia.infura.test",
      },
    });
    expect(hookMocks.evmGetBalance).toHaveBeenCalledWith({
      address: "0xdef0000000000000000000000000000000000000",
    });
    expect(hookMocks.evmReadContract).not.toHaveBeenCalled();
  });

  it("surfaces ERC20 balance query failures after loading the native EVM balance", async () => {
    const queryError = new Error("erc20 balance unavailable");
    hookMocks.evmGetBalance.mockResolvedValue(123n);
    hookMocks.evmReadContract.mockRejectedValueOnce(queryError);
    const chain = {
      contracts: {
        erc20: [
          {
            address: "0x1111111111111111111111111111111111111111",
            targetDenom: "bridge/usdc",
          },
        ],
      },
      id: 11155111,
      name: "Sepolia",
    };

    const { result } = renderHook(
      () =>
        useEvmBalances({
          address: "0xabc0000000000000000000000000000000000000",
          chain,
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(() => expect(result.current.isError).toBe(true));

    expect(hookMocks.evmGetBalance).toHaveBeenCalledWith({
      address: "0xabc0000000000000000000000000000000000000",
    });
    expect(hookMocks.evmReadContract).toHaveBeenCalledWith(
      expect.objectContaining({
        address: "0x1111111111111111111111111111111111111111",
        args: ["0xabc0000000000000000000000000000000000000"],
        functionName: "balanceOf",
      }),
    );
    expect(result.current.error).toBe(queryError);
    expect(result.current.data).toBeUndefined();
  });

  it("loads prices and exposes USD price, conversion, and balance helpers", async () => {
    const formatter = vi.fn(
      (amount: number, options: { currency?: string }) =>
        `${options.currency ?? "value"}:${amount.toFixed(2)}`,
    );

    const { result } = renderHook(
      () =>
        usePrices({
          formatter,
          refetchInterval: false,
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(() => expect(result.current.prices).toBeDefined());

    expect(publicClient.getPrices).toHaveBeenCalledOnce();
    expect(result.current.getPrice(2, "bridge/btc")).toBe(100000);
    expect(
      result.current.getPrice(2, "bridge/btc", {
        format: true,
      }),
    ).toBe("usd:100000.00");
    expect(result.current.convertAmount("0.5", "bridge/btc", "bridge/usdc")).toBe("25000");
    expect(result.current.convertAmount("0.5", "bridge/btc", "bridge/usdc", true)).toBe(
      "25000000000",
    );
    expect(
      result.current.calculateBalance({
        "bridge/btc": "100000000",
        "bridge/usdc": "2500000",
        unknown: "999",
      }),
    ).toBe(50002.5);
    expect(
      result.current.calculateBalance(
        {
          "bridge/usdc": "2500000",
        },
        {
          format: true,
        },
      ),
    ).toBe("value:2.50");
  });

  it("uses caller-provided coin metadata for price conversions and balance totals", async () => {
    const formatter = vi.fn(
      (amount: number, options: { currency?: string }) =>
        `${options.currency ?? "value"}:${amount.toFixed(2)}`,
    );
    const coins = {
      "bridge/btc": {
        decimals: 8,
        denom: "bridge/btc",
        name: "Bitcoin",
        symbol: "BTC",
        type: "native",
      },
      "custom/quote": {
        decimals: 3,
        denom: "custom/quote",
        name: "Quote",
        symbol: "QUOTE",
        type: "native",
      },
    };
    publicClient.getPrices.mockResolvedValueOnce({
      "bridge/btc": {
        humanizedPrice: "50000",
        timestamp: "1",
      },
      "custom/quote": {
        humanizedPrice: "2",
        timestamp: "1",
      },
    });

    const { result } = renderHook(
      () =>
        usePrices({
          coins,
          formatter,
          refetchInterval: false,
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(() =>
      expect(result.current.prices).toEqual(
        expect.objectContaining({
          "custom/quote": {
            humanizedPrice: "2",
            timestamp: "1",
          },
        }),
      ),
    );

    expect(publicClient.getPrices).toHaveBeenCalledOnce();
    expect(result.current.convertAmount("0.1", "bridge/btc", "custom/quote")).toBe("2500");
    expect(result.current.convertAmount("0.1", "bridge/btc", "custom/quote", true)).toBe("2500000");
    expect(
      result.current.calculateBalance({
        "custom/quote": "1250",
      }),
    ).toBe(2.5);
    expect(
      result.current.calculateBalance(
        {
          "custom/quote": "1250",
        },
        {
          format: true,
        },
      ),
    ).toBe("value:2.50");
  });

  it("keeps price helpers finite when the backend omits a known coin price", async () => {
    publicClient.getPrices.mockResolvedValueOnce({
      "bridge/usdc": {
        humanizedPrice: "1",
        timestamp: "1",
      },
    });

    const { result } = renderHook(
      () =>
        usePrices({
          refetchInterval: false,
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(() =>
      expect(result.current.prices).toEqual({
        "bridge/usdc": {
          humanizedPrice: "1",
          timestamp: "1",
        },
      }),
    );

    expect(result.current.getPrice(2, "bridge/btc")).toBe(0);
    expect(result.current.convertAmount("0.5", "bridge/btc", "bridge/usdc")).toBe("0");
    expect(
      result.current.calculateBalance({
        "bridge/btc": "100000000",
        "bridge/usdc": "2500000",
      }),
    ).toBe(2.5);
  });

  it("keeps price cache entries isolated by caller-provided coin metadata", async () => {
    const customCoins = {
      "custom/quote": {
        decimals: 3,
        denom: "custom/quote",
        name: "Quote",
        symbol: "QUOTE",
        type: "native",
      },
    };
    publicClient.getPrices
      .mockResolvedValueOnce({
        "bridge/usdc": {
          humanizedPrice: "1",
          timestamp: "1",
        },
      })
      .mockResolvedValueOnce({
        "custom/quote": {
          humanizedPrice: "2",
          timestamp: "2",
        },
      });

    const wrapper = createQueryClientWrapper();
    const defaultPrices = renderHook(() => usePrices({ refetchInterval: false }), { wrapper });

    await waitFor(() =>
      expect(defaultPrices.result.current.prices).toEqual({
        "bridge/usdc": {
          humanizedPrice: "1",
          timestamp: "1",
        },
      }),
    );

    const customPrices = renderHook(
      () =>
        usePrices({
          coins: customCoins,
          refetchInterval: false,
        }),
      { wrapper },
    );

    await waitFor(() =>
      expect(customPrices.result.current.prices).toEqual({
        "custom/quote": {
          humanizedPrice: "2",
          timestamp: "2",
        },
      }),
    );

    expect(publicClient.getPrices).toHaveBeenCalledTimes(2);
    expect(
      customPrices.result.current.calculateBalance({
        "custom/quote": "1500",
      }),
    ).toBe(3);
  });

  it("surfaces price query failures while helpers fall back to zero values", async () => {
    const formatter = vi.fn(
      (amount: number, options: { currency?: string }) =>
        `${options.currency ?? "value"}:${amount.toFixed(2)}`,
    );
    const queryError = new Error("prices unavailable");
    publicClient.getPrices.mockRejectedValueOnce(queryError);

    const { result } = renderHook(
      () =>
        usePrices({
          formatter,
          refetchInterval: false,
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(() => expect(result.current.isError).toBe(true));

    expect(publicClient.getPrices).toHaveBeenCalledOnce();
    expect(result.current.error).toBe(queryError);
    expect(result.current.prices).toBeUndefined();
    expect(result.current.getPrice(2, "bridge/btc")).toBe(0);
    expect(
      result.current.calculateBalance({
        "bridge/btc": "100000000",
        unknown: "999",
      }),
    ).toBe(0);
    expect(
      result.current.calculateBalance(
        {
          "bridge/usdc": "2500000",
        },
        {
          format: true,
        },
      ),
    ).toBe("value:0.00");
  });
});
