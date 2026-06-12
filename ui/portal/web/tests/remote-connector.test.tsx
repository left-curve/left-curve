import { cleanup, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { deserializeJson, serializeJson } from "@left-curve/encoding";

import { remote } from "../../../store/src/connectors/remote";
import { requestRemote } from "../../../store/src/remote";

const remoteMocks = vi.hoisted(() => ({
  createSignerClient: vi.fn(),
  getAccountStatus: vi.fn(),
  getUser: vi.fn(),
  toAccount: vi.fn(),
}));

vi.mock("@left-curve/sdk", () => ({
  createSignerClient: remoteMocks.createSignerClient,
  toAccount: remoteMocks.toAccount,
}));

vi.mock("@left-curve/sdk/actions", () => ({
  getUser: remoteMocks.getUser,
}));

type RemoteRequest = {
  id: string;
  method: string;
  args: unknown[];
};

const remoteKeyHash = "0x72656d6f74652d6b657900000000000000000000000000000000000000000000";
const remoteAccountAddress = "0x72656d6f74652d6163636f756e74000000000000";

function installNativeBridge() {
  const postMessage = vi.fn();

  Object.defineProperty(window, "ReactNativeWebView", {
    configurable: true,
    value: {
      postMessage,
    },
  });

  return postMessage;
}

async function waitForRemoteRequest(postMessage: ReturnType<typeof vi.fn>, callIndex = 0) {
  await waitFor(() => expect(postMessage.mock.calls.length).toBeGreaterThan(callIndex));

  const rawMessage = postMessage.mock.calls[callIndex][0];
  return deserializeJson<RemoteRequest>(rawMessage);
}

function resolveRemoteRequest(id: string, data: unknown) {
  window.dispatchEvent(
    new MessageEvent("message", {
      data: serializeJson({
        data,
        id,
        type: "dango-remote",
      }),
    }),
  );
}

function rejectRemoteRequest(id: string, error: unknown) {
  window.dispatchEvent(
    new MessageEvent("message", {
      data: serializeJson({
        error,
        id,
        type: "dango-remote",
      }),
    }),
  );
}

function createRemoteConnector({
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

  return {
    chain,
    connector: remote()({
      chain,
      emitter,
      getUserIndex,
      transport,
    } as never),
    emitter,
    transport,
  };
}

describe("remote connector bridge", () => {
  beforeEach(() => {
    remoteMocks.createSignerClient.mockReturnValue({
      getAccountStatus: remoteMocks.getAccountStatus,
    });
    remoteMocks.getAccountStatus.mockResolvedValue("active");
    remoteMocks.getUser.mockResolvedValue({
      accounts: {
        0: remoteAccountAddress,
      },
      keys: {
        [remoteKeyHash]: {
          ethereum: "0x72656d6f74652d657468657265756d0000000000",
        },
      },
      name: "remote-user",
    });
    remoteMocks.toAccount.mockImplementation(({ accountIndex, address, user }) => ({
      accountIndex,
      address,
      username: user.name,
    }));
  });

  afterEach(() => {
    cleanup();
    vi.useRealTimers();
    vi.clearAllMocks();
    Reflect.deleteProperty(window, "ReactNativeWebView");
  });

  it("posts native remote requests and resolves only matching dango responses", async () => {
    const postMessage = installNativeBridge();

    const response = requestRemote<{ keyHash: string }>("connector", "getKeyHash");
    const request = await waitForRemoteRequest(postMessage);

    expect(request).toEqual({
      args: ["getKeyHash"],
      id: expect.any(String),
      method: "connector",
    });

    window.dispatchEvent(
      new MessageEvent("message", {
        data: serializeJson({
          data: { keyHash: "ignored" },
          id: request.id,
          type: "not-dango-remote",
        }),
      }),
    );
    resolveRemoteRequest("other-request", { keyHash: "ignored" });
    resolveRemoteRequest(request.id, { keyHash: remoteKeyHash });

    await expect(response).resolves.toEqual({ keyHash: remoteKeyHash });
  });

  it("rejects native remote requests on bridge errors and timeouts", async () => {
    const postMessage = installNativeBridge();

    const rejected = requestRemote("connector", "getKeyHash");
    const rejectedRequest = await waitForRemoteRequest(postMessage, 0);
    rejectRemoteRequest(rejectedRequest.id, {
      message: "native wallet rejected",
    });

    await expect(rejected).rejects.toEqual({
      message: "native wallet rejected",
    });

    vi.useFakeTimers();
    const timedOut = requestRemote("connector", "getKeyHash");
    const timeoutRequest = deserializeJson<RemoteRequest>(postMessage.mock.calls[1][0]);

    expect(timeoutRequest).toMatchObject({
      args: ["getKeyHash"],
      method: "connector",
    });

    vi.advanceTimersByTime(30_000);

    await expect(timedOut).rejects.toThrow("Request timed out");
  });

  it("times out remote requests when the native bridge is unavailable", async () => {
    vi.useFakeTimers();

    const response = requestRemote("connector", "getKeyHash");

    vi.advanceTimersByTime(30_000);

    await expect(response).rejects.toThrow("Request timed out");
    expect(window.ReactNativeWebView).toBeUndefined();
  });

  it("connects remote accounts with the backend user, selected key hash, and account status", async () => {
    const postMessage = installNativeBridge();
    const { chain, connector, emitter, transport } = createRemoteConnector();

    const connect = connector.connect({
      chainId: chain.id,
      userIndex: 7,
    });
    const keyHashRequest = await waitForRemoteRequest(postMessage);

    expect(keyHashRequest).toMatchObject({
      args: ["getKeyHash"],
      method: "connector",
    });
    resolveRemoteRequest(keyHashRequest.id, remoteKeyHash);

    await connect;

    expect(remoteMocks.createSignerClient).toHaveBeenCalledWith({
      chain,
      signer: connector,
      transport,
      type: "remote",
    });
    expect(remoteMocks.getUser).toHaveBeenCalledWith(
      expect.objectContaining({
        getAccountStatus: remoteMocks.getAccountStatus,
      }),
      {
        userIndexOrName: {
          index: 7,
        },
      },
    );
    expect(remoteMocks.getAccountStatus).toHaveBeenCalledWith({
      address: remoteAccountAddress,
    });
    expect(emitter.emit).toHaveBeenCalledWith("connect", {
      accounts: [
        {
          accountIndex: 0,
          address: remoteAccountAddress,
          username: "remote-user",
        },
      ],
      chainId: chain.id,
      keyHash: remoteKeyHash,
      userIndex: 7,
      userStatus: "active",
      username: "remote-user",
    });
  });

  it("connects with an explicit key hash without requesting the native bridge key hash", async () => {
    const { chain, connector, emitter, transport } = createRemoteConnector();

    await connector.connect({
      chainId: chain.id,
      keyHash: remoteKeyHash,
      userIndex: 7,
    });

    expect(window.ReactNativeWebView).toBeUndefined();
    expect(remoteMocks.createSignerClient).toHaveBeenCalledWith({
      chain,
      signer: connector,
      transport,
      type: "remote",
    });
    expect(remoteMocks.getUser).toHaveBeenCalledWith(
      expect.objectContaining({
        getAccountStatus: remoteMocks.getAccountStatus,
      }),
      {
        userIndexOrName: {
          index: 7,
        },
      },
    );
    expect(remoteMocks.getAccountStatus).toHaveBeenCalledWith({
      address: remoteAccountAddress,
    });
    expect(emitter.emit).toHaveBeenCalledWith("connect", {
      accounts: [
        {
          accountIndex: 0,
          address: remoteAccountAddress,
          username: "remote-user",
        },
      ],
      chainId: chain.id,
      keyHash: remoteKeyHash,
      userIndex: 7,
      userStatus: "active",
      username: "remote-user",
    });
  });

  it("rejects remote connects when the backend user does not authorize the selected key hash", async () => {
    remoteMocks.getUser.mockResolvedValueOnce({
      accounts: {
        0: remoteAccountAddress,
      },
      keys: {},
      name: "remote-user",
    });
    const { chain, connector, emitter } = createRemoteConnector();

    await expect(
      connector.connect({
        chainId: chain.id,
        keyHash: remoteKeyHash,
        userIndex: 7,
      }),
    ).rejects.toThrow("Not authorized");

    expect(remoteMocks.getAccountStatus).not.toHaveBeenCalled();
    expect(emitter.emit).not.toHaveBeenCalled();
  });

  it("disconnects remote sessions through the connector event without native bridge calls", async () => {
    const { connector, emitter } = createRemoteConnector();

    await connector.disconnect();

    expect(emitter.emit).toHaveBeenCalledWith("disconnect");
    expect(window.ReactNativeWebView).toBeUndefined();
    expect(remoteMocks.createSignerClient).not.toHaveBeenCalled();
    expect(remoteMocks.getUser).not.toHaveBeenCalled();
  });

  it("forwards key creation and signing requests through the native remote bridge", async () => {
    const postMessage = installNativeBridge();
    const { connector } = createRemoteConnector();

    const createNewKey = connector.createNewKey();
    const createNewKeyRequest = await waitForRemoteRequest(postMessage, 0);
    expect(createNewKeyRequest).toMatchObject({
      args: ["createNewKey"],
      method: "connector",
    });
    resolveRemoteRequest(createNewKeyRequest.id, {
      key: {
        ethereum: "0x72656d6f74652d77616c6c657400000000000000",
      },
      keyHash: remoteKeyHash,
    });
    await expect(createNewKey).resolves.toEqual({
      key: {
        ethereum: "0x72656d6f74652d77616c6c657400000000000000",
      },
      keyHash: remoteKeyHash,
    });

    const signArbitrary = connector.signArbitrary({
      chainId: "dango-dev-1",
      message: "confirm remote session",
    });
    const signArbitraryRequest = await waitForRemoteRequest(postMessage, 1);
    expect(signArbitraryRequest).toMatchObject({
      args: [
        "signArbitrary",
        {
          chainId: "dango-dev-1",
          message: "confirm remote session",
        },
      ],
      method: "connector",
    });
    resolveRemoteRequest(signArbitraryRequest.id, {
      credential: {
        standard: {
          signature: "0x7369676e6174757265",
        },
      },
    });
    await expect(signArbitrary).resolves.toEqual({
      credential: {
        standard: {
          signature: "0x7369676e6174757265",
        },
      },
    });

    const signTx = connector.signTx({
      chainId: "dango-dev-1",
      messages: [{ transfer: {} }],
    } as never);
    const signTxRequest = await waitForRemoteRequest(postMessage, 2);
    expect(signTxRequest).toMatchObject({
      args: [
        "signTx",
        {
          chainId: "dango-dev-1",
          messages: [{ transfer: {} }],
        },
      ],
      method: "connector",
    });
    resolveRemoteRequest(signTxRequest.id, {
      credential: {
        standard: {
          signature: "0x74787369676e6174757265",
        },
      },
    });
    await expect(signTx).resolves.toEqual({
      credential: {
        standard: {
          signature: "0x74787369676e6174757265",
        },
      },
    });
  });

  it("reads remote accounts for the selected user index through the backend signer client", async () => {
    const secondAccountAddress = "0x72656d6f74652d7365636f6e6400000000000000";
    remoteMocks.getUser.mockResolvedValueOnce({
      accounts: {
        0: remoteAccountAddress,
        2: secondAccountAddress,
      },
      keys: {
        [remoteKeyHash]: {
          ethereum: "0x72656d6f74652d657468657265756d0000000000",
        },
      },
      name: "remote-user",
    });
    const { chain, connector, transport } = createRemoteConnector();

    await expect(connector.getAccounts()).resolves.toEqual([
      {
        accountIndex: 0,
        address: remoteAccountAddress,
        username: "remote-user",
      },
      {
        accountIndex: 2,
        address: secondAccountAddress,
        username: "remote-user",
      },
    ]);

    expect(remoteMocks.createSignerClient).toHaveBeenCalledWith({
      chain,
      signer: connector,
      transport,
      type: "remote",
    });
    expect(remoteMocks.getUser).toHaveBeenCalledWith(
      expect.objectContaining({
        getAccountStatus: remoteMocks.getAccountStatus,
      }),
      {
        userIndexOrName: {
          index: 7,
        },
      },
    );
    expect(remoteMocks.toAccount).toHaveBeenNthCalledWith(1, {
      accountIndex: 0,
      address: remoteAccountAddress,
      user: expect.objectContaining({
        name: "remote-user",
      }),
    });
    expect(remoteMocks.toAccount).toHaveBeenNthCalledWith(2, {
      accountIndex: 2,
      address: secondAccountAddress,
      user: expect.objectContaining({
        name: "remote-user",
      }),
    });
  });

  it("rejects remote account reads before a user index is selected", async () => {
    const { connector } = createRemoteConnector({
      getUserIndex: () => undefined,
    });

    await expect(connector.getAccounts()).rejects.toThrow("remote: user index not found");
    expect(remoteMocks.getUser).not.toHaveBeenCalled();
  });
});
