import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { debug } from "../../../store/src/connectors/debug";

const connectorMocks = vi.hoisted(() => ({
  createSignerClient: vi.fn(),
  getAccountStatus: vi.fn(),
  getUser: vi.fn(),
  toAccount: vi.fn(),
}));

vi.mock("@left-curve/sdk", () => ({
  createSignerClient: connectorMocks.createSignerClient,
  toAccount: connectorMocks.toAccount,
}));

vi.mock("@left-curve/sdk/actions", () => ({
  getUser: connectorMocks.getUser,
}));

const debugKeyHash = "0x64656275672d6b657900000000000000000000000000000000000000000000";
const secondaryKeyHash = "0x7365636f6e646172792d6b6579000000000000000000000000000000000000";
const userAccountAddress = "0x64656275672d6163636f756e742d3000000000";
const secondUserAccountAddress = "0x64656275672d6163636f756e742d3100000000";

function createDebugConnector({
  getUserIndex = () => 7,
}: {
  getUserIndex?: () => number | undefined;
} = {}) {
  const emitter = {
    emit: vi.fn(),
  };
  const chain = {
    id: "dango-dev-1",
    name: "Devnet",
  };
  const transport = {
    type: "http",
  };
  const connector = debug()({
    chain,
    emitter,
    getUserIndex,
    transport,
  } as never);

  return {
    chain,
    connector,
    emitter,
    transport,
  };
}

describe("debug connector", () => {
  beforeEach(() => {
    connectorMocks.createSignerClient.mockReturnValue({
      getAccountStatus: connectorMocks.getAccountStatus,
      uid: "debug-client",
    });
    connectorMocks.getAccountStatus.mockResolvedValue("active");
    connectorMocks.getUser.mockResolvedValue({
      accounts: {
        0: userAccountAddress,
        1: secondUserAccountAddress,
      },
      keys: {
        [debugKeyHash]: {
          debug: true,
        },
        [secondaryKeyHash]: {
          debug: true,
        },
      },
      name: "debug-user",
    });
    connectorMocks.toAccount.mockImplementation(({ accountIndex, address, user }) => ({
      accountIndex,
      address,
      username: user.name,
    }));
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it("connects as a backend user, chooses a registered key, and emits account status", async () => {
    const { chain, connector, emitter, transport } = createDebugConnector();

    await connector.connect({
      chainId: chain.id,
      challenge: "debug",
      userIndex: 7,
    });

    expect(connectorMocks.createSignerClient).toHaveBeenCalledWith({
      chain,
      signer: connector,
      transport,
      type: "debug",
    });
    expect(connectorMocks.getUser).toHaveBeenCalledWith(
      expect.objectContaining({
        getAccountStatus: connectorMocks.getAccountStatus,
        uid: "debug-client",
      }),
      {
        userIndexOrName: {
          index: 7,
        },
      },
    );
    expect(connectorMocks.getAccountStatus).toHaveBeenCalledWith({
      address: userAccountAddress,
    });
    expect(emitter.emit).toHaveBeenCalledWith("connect", {
      accounts: [
        {
          accountIndex: 0,
          address: userAccountAddress,
          username: "debug-user",
        },
        {
          accountIndex: 1,
          address: secondUserAccountAddress,
          username: "debug-user",
        },
      ],
      chainId: chain.id,
      keyHash: debugKeyHash,
      userIndex: 7,
      userStatus: "active",
      username: "debug-user",
    });
  });

  it("rejects backend users without registered keys before mapping accounts or emitting connect", async () => {
    connectorMocks.getUser.mockResolvedValueOnce({
      accounts: {
        0: userAccountAddress,
      },
      keys: {},
      name: "debug-user",
    });
    const { chain, connector, emitter } = createDebugConnector();

    await expect(
      connector.connect({
        chainId: chain.id,
        challenge: "debug",
        userIndex: 7,
      }),
    ).rejects.toThrow("debug: user has no registered keys");

    expect(connectorMocks.toAccount).not.toHaveBeenCalled();
    expect(connectorMocks.getAccountStatus).not.toHaveBeenCalled();
    expect(emitter.emit).not.toHaveBeenCalled();
  });

  it("tracks authorization with the selected key and clears it on disconnect", async () => {
    const { chain, connector, emitter } = createDebugConnector();

    await expect(connector.isAuthorized()).resolves.toBe(false);
    await expect(connector.getKeyHash()).rejects.toThrow("debug: not connected");

    await connector.connect({
      chainId: chain.id,
      challenge: "debug",
      userIndex: 7,
    });

    await expect(connector.isAuthorized()).resolves.toBe(true);
    await expect(connector.getKeyHash()).resolves.toBe(debugKeyHash);

    await connector.disconnect();

    expect(emitter.emit).toHaveBeenLastCalledWith("disconnect");
    await expect(connector.isAuthorized()).resolves.toBe(false);
    await expect(connector.getKeyHash()).rejects.toThrow("debug: not connected");
  });

  it("refetches accounts for the selected user index and rejects missing user state", async () => {
    const missingUserIndex = createDebugConnector({
      getUserIndex: () => undefined,
    });

    await expect(missingUserIndex.connector.getAccounts()).rejects.toThrow(
      "debug: user index not found",
    );
    expect(connectorMocks.getUser).not.toHaveBeenCalled();

    const { connector } = createDebugConnector();

    await expect(connector.getAccounts()).resolves.toEqual([
      {
        accountIndex: 0,
        address: userAccountAddress,
        username: "debug-user",
      },
      {
        accountIndex: 1,
        address: secondUserAccountAddress,
        username: "debug-user",
      },
    ]);
    expect(connectorMocks.getUser).toHaveBeenCalledWith(
      expect.objectContaining({
        getAccountStatus: connectorMocks.getAccountStatus,
        uid: "debug-client",
      }),
      {
        userIndexOrName: {
          index: 7,
        },
      },
    );
  });

  it("keeps arbitrary and transaction signing disabled", async () => {
    const { connector } = createDebugConnector();

    await expect(connector.signArbitrary({} as never)).rejects.toThrow(
      "Debug connector: signing is disabled",
    );
    await expect(connector.signTx({} as never)).rejects.toThrow(
      "Debug connector: signing is disabled",
    );
  });
});
