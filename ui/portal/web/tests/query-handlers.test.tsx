import { afterEach, describe, expect, it, vi } from "vitest";

import { connectMutationOptions } from "../../../store/src/handlers/connect";
import { disconnectMutationOptions } from "../../../store/src/handlers/disconnect";
import {
  getAccountInfoQueryKey,
  getAccountInfoQueryOptions,
} from "../../../store/src/handlers/getAccountInfo";
import {
  getAppConfigQueryKey,
  getAppConfigQueryOptions,
} from "../../../store/src/handlers/getAppConfig";
import {
  getBalancesQueryKey,
  getBalancesQueryOptions,
} from "../../../store/src/handlers/getBalances";
import { getBlockQueryKey, getBlockQueryOptions } from "../../../store/src/handlers/getBlock";
import {
  getConnectorClientQueryKey,
  getConnectorClientQueryOptions,
} from "../../../store/src/handlers/getConnectorClient";
import { withPagination } from "../../../store/src/handlers/pagination";
import { filterQueryOptions } from "../../../store/src/handlers/query";

const handlerMocks = vi.hoisted(() => ({
  connect: vi.fn(),
  disconnect: vi.fn(),
  getAccountInfo: vi.fn(),
  getAppConfig: vi.fn(),
  getBalances: vi.fn(),
  getBlock: vi.fn(),
  getConnectorClient: vi.fn(),
}));

vi.mock("../../../store/src/actions/connect.js", () => ({
  connect: handlerMocks.connect,
}));

vi.mock("../../../store/src/actions/disconnect.js", () => ({
  disconnect: handlerMocks.disconnect,
}));

vi.mock("../../../store/src/actions/getBalances.js", () => ({
  getBalances: handlerMocks.getBalances,
}));

vi.mock("../../../store/src/actions/getBlock.js", () => ({
  getBlock: handlerMocks.getBlock,
}));

vi.mock("../../../store/src/actions/getAccountInfo.js", () => ({
  getAccountInfo: handlerMocks.getAccountInfo,
}));

vi.mock("../../../store/src/actions/getAppConfig.js", () => ({
  getAppConfig: handlerMocks.getAppConfig,
}));

vi.mock("../../../store/src/actions/getConnectorClient.js", () => ({
  getConnectorClient: handlerMocks.getConnectorClient,
}));

const accountAddress = "0x71756572792d68616e646c65722d616363740000";

function createConfig() {
  return {
    state: {
      chainId: "dango-dev-1",
      connectors: new Map(),
      current: null,
      isMipdLoaded: false,
      status: "disconnected",
      user: undefined,
    },
  };
}

describe("store query and mutation handlers", () => {
  afterEach(() => {
    vi.clearAllMocks();
  });

  it("filters React Query, store, and connector-only options out of stable query keys", () => {
    expect(
      filterQueryOptions({
        address: accountAddress,
        config: { kind: "config" },
        connector: { id: "passkey" },
        enabled: false,
        gcTime: 1_000,
        getNextPageParam: () => undefined,
        initialData: { cached: true },
        meta: { reason: "test" },
        query: { staleTime: 10 },
        queryFn: () => undefined,
        retry: false,
        scopeKey: "balances-panel",
        staleTime: 5_000,
      }),
    ).toEqual({
      address: accountAddress,
      scopeKey: "balances-panel",
    });
  });

  it("keeps observer-only query controls out of backend query keys", () => {
    expect(
      filterQueryOptions({
        _defaulted: true,
        _optimisticResults: "optimistic",
        address: accountAddress,
        behavior: { kind: "observer-behavior" },
        height: 12,
        initialDataUpdatedAt: 1_717_930_000,
        initialPageParam: "first-page",
        maxPages: 3,
        networkMode: "always",
        notifyOnChangeProps: ["data"],
        placeholderData: { cached: true },
        queryHash: "hashed-query",
        queryKey: ["client-side-key"],
        queryKeyHashFn: () => "client-side-hash",
        refetchInterval: 5_000,
        refetchIntervalInBackground: true,
        refetchOnMount: "always",
        refetchOnReconnect: false,
        refetchOnWindowFocus: false,
        retryDelay: 100,
        retryOnMount: false,
        select: (value: unknown) => value,
        structuralSharing: false,
        suspense: false,
        throwOnError: true,
      }),
    ).toEqual({
      address: accountAddress,
      height: 12,
    });
  });

  it("keeps falsy backend parameters in stable query keys", () => {
    expect(
      filterQueryOptions({
        address: accountAddress,
        enabled: false,
        height: 0,
        includeArchived: false,
        maybeCursor: null,
        query: { staleTime: 10 },
        retry: false,
        scopeKey: "falsy-backend-params",
        staleTime: 5_000,
      }),
    ).toEqual({
      address: accountAddress,
      height: 0,
      includeArchived: false,
      maybeCursor: null,
      scopeKey: "falsy-backend-params",
    });
  });

  it("builds balances query keys from backend parameters and scope only", async () => {
    handlerMocks.getBalances.mockResolvedValue({
      "bridge/usdc": "42",
    });
    const config = createConfig();
    const options = getBalancesQueryOptions(
      config as never,
      {
        address: accountAddress,
        enabled: false,
        query: { staleTime: 10 },
        retry: false,
        scopeKey: "account-balances",
        staleTime: 5_000,
      } as never,
    );

    expect(options.queryKey).toEqual([
      "getBalances",
      {
        address: accountAddress,
        scopeKey: "account-balances",
      },
    ]);
    expect(
      getBalancesQueryKey({
        address: accountAddress,
        enabled: false,
        scopeKey: "account-balances",
      } as never),
    ).toEqual(options.queryKey);

    await expect(
      options.queryFn({
        queryKey: options.queryKey,
      } as never),
    ).resolves.toEqual({
      "bridge/usdc": "42",
    });

    expect(handlerMocks.getBalances).toHaveBeenCalledWith(config, {
      address: accountAddress,
    });
  });

  it("preserves paginated balance query parameters in keys and delegated backend parameters", async () => {
    handlerMocks.getBalances.mockResolvedValue({
      "bridge/atom": "100",
    });
    const config = createConfig();
    const options = getBalancesQueryOptions(
      config as never,
      {
        address: accountAddress,
        enabled: false,
        height: 64,
        limit: 25,
        query: { staleTime: 10 },
        retry: false,
        scopeKey: "paginated-balances",
        startAfter: "bridge/usdc",
      } as never,
    );

    expect(options.queryKey).toEqual([
      "getBalances",
      {
        address: accountAddress,
        height: 64,
        limit: 25,
        scopeKey: "paginated-balances",
        startAfter: "bridge/usdc",
      },
    ]);
    expect(
      getBalancesQueryKey({
        address: accountAddress,
        height: 64,
        limit: 25,
        scopeKey: "paginated-balances",
        startAfter: "bridge/usdc",
      } as never),
    ).toEqual(options.queryKey);

    await expect(options.queryFn({ queryKey: options.queryKey } as never)).resolves.toEqual({
      "bridge/atom": "100",
    });
    expect(handlerMocks.getBalances).toHaveBeenCalledWith(config, {
      address: accountAddress,
      height: 64,
      limit: 25,
      startAfter: "bridge/usdc",
    });
  });

  it("preserves zero-valued heights in query keys and delegated backend parameters", async () => {
    const config = createConfig();
    handlerMocks.getBalances.mockResolvedValue({
      "bridge/usdc": "0",
    });
    handlerMocks.getAccountInfo.mockResolvedValue({
      address: accountAddress,
      params: {
        sequence: 0,
      },
    });
    handlerMocks.getBlock.mockResolvedValue({
      hash: "0x67656e657369732d626c6f636b000000000000",
      height: "0",
      timestamp: "2026-06-09T00:00:00.000Z",
    });

    const balancesOptions = getBalancesQueryOptions(
      config as never,
      {
        address: accountAddress,
        enabled: false,
        height: 0,
        query: { staleTime: 10 },
        scopeKey: "genesis-balances",
      } as never,
    );
    const accountOptions = getAccountInfoQueryOptions(
      config as never,
      {
        address: accountAddress,
        height: 0,
        scopeKey: "genesis-account",
      } as never,
    );
    const blockOptions = getBlockQueryOptions(
      config as never,
      {
        height: 0,
        scopeKey: "genesis-block",
      } as never,
    );

    expect(balancesOptions.queryKey).toEqual([
      "getBalances",
      {
        address: accountAddress,
        height: 0,
        scopeKey: "genesis-balances",
      },
    ]);
    expect(accountOptions.queryKey).toEqual([
      "getAccountInfo",
      {
        address: accountAddress,
        height: 0,
        scopeKey: "genesis-account",
      },
    ]);
    expect(blockOptions.queryKey).toEqual([
      "GetBlock",
      {
        height: 0,
        scopeKey: "genesis-block",
      },
    ]);

    await expect(
      balancesOptions.queryFn({ queryKey: balancesOptions.queryKey } as never),
    ).resolves.toEqual({
      "bridge/usdc": "0",
    });
    await expect(
      accountOptions.queryFn({ queryKey: accountOptions.queryKey } as never),
    ).resolves.toMatchObject({
      address: accountAddress,
    });
    await expect(
      blockOptions.queryFn({ queryKey: blockOptions.queryKey } as never),
    ).resolves.toEqual({
      hash: "0x67656e657369732d626c6f636b000000000000",
      height: "0",
      timestamp: "2026-06-09T00:00:00.000Z",
    });

    expect(handlerMocks.getBalances).toHaveBeenCalledWith(config, {
      address: accountAddress,
      height: 0,
    });
    expect(handlerMocks.getAccountInfo).toHaveBeenCalledWith(config, {
      address: accountAddress,
      height: 0,
    });
    expect(handlerMocks.getBlock).toHaveBeenCalledWith(config, {
      height: 0,
    });
  });

  it("includes connector uid and current config state in connector-client query keys", async () => {
    const config = createConfig();
    const connectorClient = {
      uid: "connector-client",
    };
    handlerMocks.getConnectorClient.mockResolvedValue(connectorClient);

    const options = getConnectorClientQueryOptions(
      config as never,
      {
        connectorUId: "connector-1",
        enabled: false,
        retry: false,
        scopeKey: "signer-client",
      } as never,
    );

    expect(options.gcTime).toBe(0);
    expect(options.queryKey).toEqual([
      "connectorClient",
      {
        connectorUId: "connector-1",
        scopeKey: "signer-client",
        state: config.state,
      },
    ]);
    expect(
      getConnectorClientQueryKey(
        config as never,
        {
          connectorUId: "connector-1",
          enabled: false,
          retry: false,
          scopeKey: "signer-client",
        } as never,
      ),
    ).toEqual(options.queryKey);

    await expect(options.queryFn({ queryKey: options.queryKey } as never)).resolves.toBe(
      connectorClient,
    );
    expect(handlerMocks.getConnectorClient).toHaveBeenCalledWith(
      config,
      expect.objectContaining({
        connectorUId: "connector-1",
      }),
    );
  });

  it("preserves account-info chain id in query keys and delegated backend parameters", async () => {
    const config = createConfig();
    handlerMocks.getAccountInfo.mockResolvedValue({
      address: accountAddress,
      params: {
        sequence: 8,
      },
    });

    const options = getAccountInfoQueryOptions(
      config as never,
      {
        address: accountAddress,
        chainId: "dango-dev-1",
        height: 128,
        query: { staleTime: 10 },
        scopeKey: "account-chain-details",
      } as never,
    );

    expect(options.queryKey).toEqual([
      "getAccountInfo",
      {
        address: accountAddress,
        chainId: "dango-dev-1",
        height: 128,
        scopeKey: "account-chain-details",
      },
    ]);
    expect(
      getAccountInfoQueryKey({
        address: accountAddress,
        chainId: "dango-dev-1",
        height: 128,
        scopeKey: "account-chain-details",
      } as never),
    ).toEqual(options.queryKey);

    await expect(options.queryFn({ queryKey: options.queryKey } as never)).resolves.toEqual({
      address: accountAddress,
      params: {
        sequence: 8,
      },
    });
    expect(handlerMocks.getAccountInfo).toHaveBeenCalledWith(config, {
      address: accountAddress,
      chainId: "dango-dev-1",
      height: 128,
    });
  });

  it("builds no-cache connector-client queries for the current connector by default", async () => {
    const config = createConfig();
    const connectorClient = {
      uid: "current-connector-client",
    };
    handlerMocks.getConnectorClient.mockResolvedValue(connectorClient);

    const options = getConnectorClientQueryOptions(config as never);

    expect(options.gcTime).toBe(0);
    expect(options.queryKey).toEqual([
      "connectorClient",
      {
        connectorUId: undefined,
        state: config.state,
      },
    ]);

    await expect(options.queryFn({ queryKey: options.queryKey } as never)).resolves.toBe(
      connectorClient,
    );
    expect(handlerMocks.getConnectorClient).toHaveBeenCalledWith(
      config,
      expect.objectContaining({
        connectorUId: undefined,
      }),
    );
  });

  it("keeps block, account-info, and app-config query keys stable while delegating backend parameters", async () => {
    const config = createConfig();
    handlerMocks.getBlock.mockResolvedValue({
      hash: "0x626c6f636b000000000000000000000000000000",
      height: "99",
      timestamp: "2026-06-09T08:00:00.000Z",
    });
    handlerMocks.getAccountInfo.mockResolvedValue({
      address: accountAddress,
      params: {
        sequence: 4,
      },
    });
    handlerMocks.getAppConfig.mockResolvedValue({
      addresses: {
        accountFactory: "0x6163636f756e742d666163746f72790000000000",
      },
    });

    const blockOptions = getBlockQueryOptions(
      config as never,
      {
        enabled: false,
        height: 99,
        scopeKey: "explorer-block",
      } as never,
    );
    const accountOptions = getAccountInfoQueryOptions(
      config as never,
      {
        address: accountAddress,
        height: 99,
        query: {
          staleTime: 10,
        },
        scopeKey: "account-details",
      } as never,
    );
    const appOptions = getAppConfigQueryOptions(
      config as never,
      {
        retry: false,
        scopeKey: "root-loader",
      } as never,
    );

    expect(blockOptions.queryKey).toEqual([
      "GetBlock",
      {
        height: 99,
        scopeKey: "explorer-block",
      },
    ]);
    expect(getBlockQueryKey({ height: 99, scopeKey: "explorer-block" })).toEqual(
      blockOptions.queryKey,
    );

    expect(accountOptions.queryKey).toEqual([
      "getAccountInfo",
      {
        address: accountAddress,
        height: 99,
        scopeKey: "account-details",
      },
    ]);
    expect(
      getAccountInfoQueryKey({
        address: accountAddress,
        height: 99,
        scopeKey: "account-details",
      } as never),
    ).toEqual(accountOptions.queryKey);

    expect(appOptions.queryKey).toEqual([
      "getAppConfig",
      {
        scopeKey: "root-loader",
      },
    ]);
    expect(getAppConfigQueryKey({ scopeKey: "root-loader" })).toEqual(appOptions.queryKey);

    await expect(
      blockOptions.queryFn({ queryKey: blockOptions.queryKey } as never),
    ).resolves.toEqual({
      hash: "0x626c6f636b000000000000000000000000000000",
      height: "99",
      timestamp: "2026-06-09T08:00:00.000Z",
    });
    await expect(
      accountOptions.queryFn({ queryKey: accountOptions.queryKey } as never),
    ).resolves.toEqual({
      address: accountAddress,
      params: {
        sequence: 4,
      },
    });
    await expect(appOptions.queryFn({ queryKey: appOptions.queryKey } as never)).resolves.toEqual({
      addresses: {
        accountFactory: "0x6163636f756e742d666163746f72790000000000",
      },
    });

    expect(handlerMocks.getBlock).toHaveBeenCalledWith(config, {
      height: 99,
    });
    expect(handlerMocks.getAccountInfo).toHaveBeenCalledWith(config, {
      address: accountAddress,
      height: 99,
    });
    expect(handlerMocks.getAppConfig).toHaveBeenCalledWith(config);
  });

  it("delegates connect and disconnect mutation functions to store actions", async () => {
    handlerMocks.connect.mockResolvedValue(undefined);
    handlerMocks.disconnect.mockResolvedValue(undefined);
    const config = createConfig();
    const connector = {
      uid: "connector-uid",
    };

    const connectOptions = connectMutationOptions(config as never);
    const disconnectOptions = disconnectMutationOptions(config as never);

    expect(connectOptions.mutationKey).toEqual(["connect"]);
    expect(disconnectOptions.mutationKey).toEqual(["disconnect"]);

    await connectOptions.mutationFn({
      challenge: "sign-in",
      chainId: "dango-dev-1",
      connector,
      userIndex: 7,
    } as never);
    await disconnectOptions.mutationFn({
      connectorUId: "connector-uid",
    });

    expect(handlerMocks.connect).toHaveBeenCalledWith(config, {
      challenge: "sign-in",
      chainId: "dango-dev-1",
      connector,
      userIndex: 7,
    });
    expect(handlerMocks.disconnect).toHaveBeenCalledWith(config, {
      connectorUId: "connector-uid",
    });
  });

  it("delegates user index zero through connect mutation variables", async () => {
    handlerMocks.connect.mockResolvedValue(undefined);
    const config = createConfig();
    const connector = {
      uid: "genesis-user-connector",
    };
    const connectOptions = connectMutationOptions(config as never);

    await connectOptions.mutationFn({
      challenge: "sign-in",
      chainId: "dango-dev-1",
      connector,
      userIndex: 0,
    } as never);

    expect(handlerMocks.connect).toHaveBeenCalledWith(config, {
      challenge: "sign-in",
      chainId: "dango-dev-1",
      connector,
      userIndex: 0,
    });
  });

  it("propagates connect and disconnect mutation failures to React Query callers", async () => {
    const connectError = new Error("connect rejected by backend");
    const disconnectError = new Error("disconnect rejected by backend");
    handlerMocks.connect.mockRejectedValue(connectError);
    handlerMocks.disconnect.mockRejectedValue(disconnectError);
    const config = createConfig();
    const connector = {
      uid: "connector-uid",
    };

    const connectOptions = connectMutationOptions(config as never);
    const disconnectOptions = disconnectMutationOptions(config as never);

    await expect(
      connectOptions.mutationFn({
        challenge: "sign-in",
        chainId: "dango-dev-1",
        connector,
        userIndex: 7,
      } as never),
    ).rejects.toBe(connectError);
    await expect(
      disconnectOptions.mutationFn({
        connectorUId: "connector-uid",
      }),
    ).rejects.toBe(disconnectError);

    expect(handlerMocks.connect).toHaveBeenCalledWith(config, {
      challenge: "sign-in",
      chainId: "dango-dev-1",
      connector,
      userIndex: 7,
    });
    expect(handlerMocks.disconnect).toHaveBeenCalledWith(config, {
      connectorUId: "connector-uid",
    });
  });

  it("derives GraphQL cursor pagination parameters from backend page info", () => {
    const pagination = withPagination<{ id: string }>({
      limit: 25,
      sortBy: "BLOCK_HEIGHT_ASC",
    });

    expect(pagination.initialPageParam).toEqual({
      first: 25,
      sortBy: "BLOCK_HEIGHT_ASC",
    });
    expect(
      pagination.getNextPageParam({
        nodes: [{ id: "first" }],
        pageInfo: {
          endCursor: "next-cursor",
          hasNextPage: true,
          hasPreviousPage: false,
          startCursor: "first-cursor",
        },
      }),
    ).toEqual({
      after: "next-cursor",
      first: 25,
      sortBy: "BLOCK_HEIGHT_ASC",
    });
    expect(
      pagination.getPreviousPageParam({
        nodes: [{ id: "second" }],
        pageInfo: {
          endCursor: "second-cursor",
          hasNextPage: false,
          hasPreviousPage: true,
          startCursor: "previous-cursor",
        },
      }),
    ).toEqual({
      before: "previous-cursor",
      last: 25,
      sortBy: "BLOCK_HEIGHT_ASC",
    });

    expect(
      pagination.getNextPageParam({
        nodes: [],
        pageInfo: {
          hasNextPage: false,
          hasPreviousPage: false,
        },
      }),
    ).toBeUndefined();
    expect(
      pagination.getPreviousPageParam({
        nodes: [],
        pageInfo: {
          hasNextPage: false,
          hasPreviousPage: false,
        },
      }),
    ).toBeUndefined();
  });

  it("uses backend GraphQL pagination defaults when no client limit or sort is provided", () => {
    const pagination = withPagination<{ id: string }>({});

    expect(pagination.initialPageParam).toEqual({
      first: 10,
      sortBy: undefined,
    });
    expect(
      pagination.getNextPageParam({
        nodes: [{ id: "first" }],
        pageInfo: {
          endCursor: "backend-next-cursor",
          hasNextPage: true,
          hasPreviousPage: false,
          startCursor: "backend-start-cursor",
        },
      }),
    ).toEqual({
      after: "backend-next-cursor",
      first: 10,
      sortBy: undefined,
    });
    expect(
      pagination.getPreviousPageParam({
        nodes: [{ id: "second" }],
        pageInfo: {
          endCursor: "backend-end-cursor",
          hasNextPage: false,
          hasPreviousPage: true,
          startCursor: "backend-previous-cursor",
        },
      }),
    ).toEqual({
      before: "backend-previous-cursor",
      last: 10,
      sortBy: undefined,
    });
  });
});
