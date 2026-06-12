import { waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { publicActions } from "@left-curve/sdk";

import { changeAccount } from "../../../store/src/actions/changeAccount";
import { connect } from "../../../store/src/actions/connect";
import { disconnect } from "../../../store/src/actions/disconnect";
import { getAccount } from "../../../store/src/actions/getAccount";
import { getAccountInfo } from "../../../store/src/actions/getAccountInfo";
import { getAppConfig } from "../../../store/src/actions/getAppConfig";
import { getBalances } from "../../../store/src/actions/getBalances";
import { getBlock } from "../../../store/src/actions/getBlock";
import { getChainId } from "../../../store/src/actions/getChainId";
import { getConnector } from "../../../store/src/actions/getConnector";
import { getConnectorClient } from "../../../store/src/actions/getConnectorClient";
import { getConnectors } from "../../../store/src/actions/getConnectors";
import { getPublicClient } from "../../../store/src/actions/getPublicClient";
import { refreshAccounts } from "../../../store/src/actions/refreshAccounts";
import { refreshUserStatus } from "../../../store/src/actions/refreshUserStatus";
import { watchAccount } from "../../../store/src/actions/watchAccount";
import { watchChainId } from "../../../store/src/actions/watchChainId";
import { watchPublicClient } from "../../../store/src/actions/watchPublicClient";
import { ConnectionStatus } from "../../../store/src/types/store";
import { createTestConfig, createTestConnector } from "./mocks/store-config";

import type { Account } from "@left-curve/types";

const sdkActionMocks = vi.hoisted(() => ({
  getAccountInfo: vi.fn(),
  getBalances: vi.fn(),
  queryBlock: vi.fn(),
}));

vi.mock("@left-curve/sdk/actions", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@left-curve/sdk/actions")>();

  return {
    ...actual,
    getAccountInfo: sdkActionMocks.getAccountInfo,
    getBalances: sdkActionMocks.getBalances,
    queryBlock: sdkActionMocks.queryBlock,
  };
});

const firstAccount = {
  address: "0x73746f72652d616374696f6e2d6669727374",
  index: 0,
  owner: 7,
} as Account;

const secondAccount = {
  address: "0x73746f72652d616374696f6e2d7365636f6e64",
  index: 1,
  owner: 7,
} as Account;

const thirdAccount = {
  address: "0x73746f72652d616374696f6e2d7468697264",
  index: 2,
  owner: 7,
} as Account;

const otherAccount = {
  address: "0x73746f72652d616374696f6e2d6f74686572",
  index: 0,
  owner: 9,
} as Account;

function setConnectionAccounts(
  config: ReturnType<typeof createTestConfig>,
  connectorUid: string,
  accounts: Account[],
  account = accounts[0],
) {
  const connection = config.state.connectors.get(connectorUid);
  if (!connection) throw new Error(`Missing ${connectorUid} connection`);

  config.state.connectors.set(connectorUid, {
    ...connection,
    account,
    accounts,
    keyHash: `${connectorUid}-key-hash`,
  });
}

describe("store actions", () => {
  afterEach(() => {
    vi.clearAllMocks();
  });

  it("derives the connected account aggregate and exposes connected-only helpers", () => {
    const wallet = createTestConnector("wallet");
    const config = createTestConfig({
      chainId: "dango-dev-1",
      connectors: [wallet],
      current: "wallet",
      status: ConnectionStatus.Connected,
      user: {
        index: 7,
        status: "active",
        username: "alice",
      },
    });
    setConnectionAccounts(config, "wallet", [firstAccount, secondAccount], firstAccount);

    const account = getAccount(config);

    expect(account).toMatchObject({
      account: firstAccount,
      accounts: [firstAccount, secondAccount],
      chain: {
        id: "dango-dev-1",
      },
      chainId: "dango-dev-1",
      connector: wallet,
      isConnected: true,
      isConnecting: false,
      isDisconnected: false,
      isReconnecting: false,
      isUserActive: true,
      keyHash: "wallet-key-hash",
      status: "connected",
      userIndex: 7,
      username: "alice",
      userStatus: "active",
    });

    if (account.status !== "connected") throw new Error("Expected connected account state");

    account.changeAccount(secondAccount.address);

    expect(config.state.connectors.get("wallet")?.account).toEqual(secondAccount);
    expect(account.refreshAccounts).toEqual(expect.any(Function));
    expect(account.refreshUserStatus).toEqual(expect.any(Function));
  });

  it("refreshes backend accounts and user status through connected account helpers", async () => {
    const getUser = vi.fn().mockResolvedValue({
      accounts: {
        1: secondAccount.address,
        2: thirdAccount.address,
      },
      index: 7,
      name: "alice-renamed",
    });
    const getAccountStatus = vi.fn().mockResolvedValue("inactive");
    const wallet = createTestConnector("wallet", {
      getClient: vi.fn().mockResolvedValue({
        getAccountStatus,
        getUser,
      }),
    });
    const config = createTestConfig({
      chainId: "dango-dev-1",
      connectors: [wallet],
      current: "wallet",
      status: ConnectionStatus.Connected,
      user: {
        index: 7,
        status: "active",
        username: "alice",
      },
    });
    setConnectionAccounts(config, "wallet", [firstAccount, secondAccount], secondAccount);

    const account = getAccount(config);
    if (account.status !== "connected") throw new Error("Expected connected account state");

    await account.refreshAccounts();
    account.refreshUserStatus();

    await waitFor(() => {
      expect(getUser).toHaveBeenCalledWith({
        userIndexOrName: {
          index: 7,
        },
      });
      expect(getAccountStatus).toHaveBeenCalledWith({
        address: secondAccount.address,
      });
    });
    expect(wallet.getClient).toHaveBeenCalledTimes(2);
    expect(config.state.user).toEqual({
      index: 7,
      status: "inactive",
      username: "alice-renamed",
    });
    expect(config.state.connectors.get("wallet")?.accounts).toEqual([
      {
        address: secondAccount.address,
        index: 1,
        owner: 7,
      },
      {
        address: thirdAccount.address,
        index: 2,
        owner: 7,
      },
    ]);
    expect(config.state.connectors.get("wallet")?.account).toEqual(secondAccount);
  });

  it("keeps backend user index zero connected and refreshes accounts with index zero", async () => {
    const zeroAccount = {
      address: "0x73746f72652d616374696f6e2d7a65726f",
      index: 0,
      owner: 0,
    } as Account;
    const getUser = vi.fn().mockResolvedValue({
      accounts: {
        0: zeroAccount.address,
      },
      index: 0,
      name: "zero-renamed",
    });
    const wallet = createTestConnector("wallet", {
      getClient: vi.fn().mockResolvedValue({
        getUser,
      }),
    });
    const config = createTestConfig({
      chainId: "dango-dev-1",
      connectors: [wallet],
      current: "wallet",
      status: ConnectionStatus.Connected,
      user: {
        index: 0,
        status: "active",
        username: "zero",
      },
    });
    setConnectionAccounts(config, "wallet", [zeroAccount], zeroAccount);

    const account = getAccount(config);
    expect(account).toMatchObject({
      account: zeroAccount,
      isConnected: true,
      isUserActive: true,
      status: "connected",
      userIndex: 0,
      username: "zero",
    });
    if (account.status !== "connected") throw new Error("Expected connected account state");

    await account.refreshAccounts();

    await waitFor(() =>
      expect(getUser).toHaveBeenCalledWith({
        userIndexOrName: {
          index: 0,
        },
      }),
    );
    expect(config.state.user).toEqual({
      index: 0,
      status: "active",
      username: "zero-renamed",
    });
    expect(config.state.connectors.get("wallet")?.accounts).toEqual([zeroAccount]);
    expect(config.state.connectors.get("wallet")?.account).toEqual(zeroAccount);
  });

  it("preserves transitional account snapshots without connected-only helpers", () => {
    const wallet = createTestConnector("wallet");
    const config = createTestConfig({
      connectors: [wallet],
      current: "wallet",
      status: ConnectionStatus.Reconnecting,
      user: {
        index: 7,
        status: "inactive",
        username: "alice",
      },
    });
    setConnectionAccounts(config, "wallet", [firstAccount], firstAccount);

    const reconnecting = getAccount(config);

    expect(reconnecting).toMatchObject({
      account: firstAccount,
      chainId: "dango-dev-1",
      connector: wallet,
      isConnected: false,
      isConnecting: false,
      isDisconnected: false,
      isReconnecting: true,
      isUserActive: false,
      status: "reconnecting",
      userIndex: 7,
      username: "alice",
      userStatus: "inactive",
    });
    expect(reconnecting.changeAccount).toBeUndefined();
    expect(reconnecting.refreshAccounts).toBeUndefined();
    expect(reconnecting.refreshUserStatus).toBeUndefined();

    config.state = {
      ...config.state,
      status: ConnectionStatus.Connecting,
    };

    const connecting = getAccount(config);

    expect(connecting).toMatchObject({
      account: firstAccount,
      isConnecting: true,
      isDisconnected: false,
      isReconnecting: false,
      status: "connecting",
    });
    expect(connecting.changeAccount).toBeUndefined();
    expect(connecting.refreshAccounts).toBeUndefined();
    expect(connecting.refreshUserStatus).toBeUndefined();
  });

  it("returns the disconnected aggregate when no current connection exists", () => {
    const wallet = createTestConnector("wallet");
    const config = createTestConfig({
      connectors: [wallet],
      current: "missing-wallet",
      status: ConnectionStatus.Connected,
      user: {
        index: 7,
        status: "active",
        username: "alice",
      },
    });

    expect(getAccount(config)).toEqual({
      account: undefined,
      accounts: undefined,
      chain: undefined,
      chainId: undefined,
      changeAccount: undefined,
      connector: undefined,
      isConnected: false,
      isConnecting: false,
      isDisconnected: true,
      isReconnecting: false,
      isUserActive: false,
      keyHash: undefined,
      refreshAccounts: undefined,
      refreshUserStatus: undefined,
      status: "disconnected",
      userIndex: undefined,
      username: undefined,
      userStatus: undefined,
    });
  });

  it("changes the selected account only when the target connection contains the address", () => {
    const wallet = createTestConnector("wallet");
    const other = createTestConnector("other");
    const config = createTestConfig({
      connectors: [wallet, other],
      current: "wallet",
      status: ConnectionStatus.Connected,
      user: {
        index: 7,
        status: "active",
        username: "alice",
      },
    });
    setConnectionAccounts(config, "wallet", [firstAccount, secondAccount]);
    setConnectionAccounts(config, "other", [otherAccount]);

    changeAccount(config, {
      address: secondAccount.address,
    });

    expect(config.state.connectors.get("wallet")?.account).toEqual(secondAccount);
    expect(config.state.connectors.get("other")?.account).toEqual(otherAccount);

    changeAccount(config, {
      address: "0x73746f72652d616374696f6e2d6d697373",
    });

    expect(config.state.connectors.get("wallet")?.account).toEqual(secondAccount);

    changeAccount(config, {
      address: otherAccount.address,
      connectorUId: "other",
    });

    expect(config.state.connectors.get("other")?.account).toEqual(otherAccount);
  });

  it("rolls back a failed connector connect without creating a connection", async () => {
    const connectError = new Error("backend authorization failed");
    const wallet = createTestConnector("wallet", {
      connect: vi.fn().mockRejectedValue(connectError),
    });
    const config = createTestConfig();

    await expect(
      connect(config, {
        challenge: "sign-in",
        chainId: "dango-dev-1",
        connector: wallet,
        userIndex: 7,
      }),
    ).rejects.toBe(connectError);

    expect(wallet.emitter.emit).toHaveBeenCalledWith("message", { type: "connecting" });
    expect(wallet.connect).toHaveBeenCalledWith({
      challenge: "sign-in",
      chainId: "dango-dev-1",
      userIndex: 7,
    });
    expect(config.state).toMatchObject({
      connectors: new Map(),
      status: ConnectionStatus.Disconnected,
    });
  });

  it("preserves an active connection when another connector login fails", async () => {
    const connectError = new Error("secondary authorization failed");
    const wallet = createTestConnector("wallet");
    const secondary = createTestConnector("secondary", {
      connect: vi.fn().mockRejectedValue(connectError),
    });
    const config = createTestConfig({
      connectors: [wallet],
      current: "wallet",
      status: ConnectionStatus.Connected,
      user: {
        index: 7,
        status: "active",
        username: "alice",
      },
    });
    setConnectionAccounts(config, "wallet", [firstAccount], firstAccount);

    await expect(
      connect(config, {
        challenge: "sign-in",
        chainId: "dango-dev-1",
        connector: secondary,
        userIndex: 7,
      }),
    ).rejects.toBe(connectError);

    expect(secondary.emitter.emit).toHaveBeenCalledWith("message", { type: "connecting" });
    expect(secondary.connect).toHaveBeenCalledWith({
      challenge: "sign-in",
      chainId: "dango-dev-1",
      userIndex: 7,
    });
    expect(wallet.connect).not.toHaveBeenCalled();
    expect(config.state).toMatchObject({
      current: "wallet",
      status: ConnectionStatus.Connected,
      user: {
        index: 7,
        status: "active",
        username: "alice",
      },
    });
    expect(config.state.connectors.get("wallet")?.account).toEqual(firstAccount);
    expect(config.state.connectors.has("secondary")).toBe(false);
  });

  it("disconnects an explicit non-current connector without dropping the active connection", async () => {
    const wallet = createTestConnector("wallet");
    const backup = createTestConnector("backup");
    const config = createTestConfig({
      connectors: [wallet, backup],
      current: "wallet",
      status: ConnectionStatus.Connected,
      user: {
        index: 7,
        status: "active",
        username: "alice",
      },
    });

    await disconnect(config, {
      connectorUId: "backup",
    });

    expect(backup.disconnect).toHaveBeenCalledOnce();
    expect(wallet.disconnect).not.toHaveBeenCalled();
    expect(config.state).toMatchObject({
      current: "wallet",
      status: ConnectionStatus.Connected,
      user: {
        index: 7,
        status: "active",
        username: "alice",
      },
    });
    expect(config.state.connectors.has("wallet")).toBe(true);
    expect(config.state.connectors.has("backup")).toBe(false);
    expect(config.state.connectors.get("wallet")?.connector).toBe(wallet);
  });

  it("leaves active connection state unchanged when disconnecting an unknown connector", async () => {
    const wallet = createTestConnector("wallet");
    const config = createTestConfig({
      connectors: [wallet],
      current: "wallet",
      status: ConnectionStatus.Connected,
      user: {
        index: 7,
        status: "active",
        username: "alice",
      },
    });
    const beforeConnectors = config.state.connectors;

    await disconnect(config, {
      connectorUId: "missing",
    });

    expect(wallet.disconnect).not.toHaveBeenCalled();
    expect(config.state).toMatchObject({
      current: "wallet",
      status: ConnectionStatus.Connected,
      user: {
        index: 7,
        status: "active",
        username: "alice",
      },
    });
    expect(config.state.connectors).not.toBe(beforeConnectors);
    expect(config.state.connectors.get("wallet")?.connector).toBe(wallet);
  });

  it("refreshes accounts through the connector client and preserves the selected address", async () => {
    const getUser = vi.fn().mockResolvedValue({
      accounts: {
        1: secondAccount.address,
        2: thirdAccount.address,
      },
      index: 7,
      name: "alice-renamed",
    });
    const wallet = createTestConnector("wallet", {
      getClient: vi.fn().mockResolvedValue({
        getUser,
      }),
    });
    const config = createTestConfig({
      connectors: [wallet],
      current: "wallet",
      status: ConnectionStatus.Connected,
      user: {
        index: 7,
        status: "active",
        username: "alice",
      },
    });
    setConnectionAccounts(config, "wallet", [firstAccount, secondAccount], secondAccount);

    await refreshAccounts(config, {
      userIndex: 7,
    });

    expect(wallet.getClient).toHaveBeenCalledOnce();
    expect(getUser).toHaveBeenCalledWith({
      userIndexOrName: {
        index: 7,
      },
    });
    expect(config.state.user).toEqual({
      index: 7,
      status: "active",
      username: "alice-renamed",
    });
    expect(config.state.connectors.get("wallet")?.accounts).toEqual([
      {
        address: secondAccount.address,
        index: 1,
        owner: 7,
      },
      {
        address: thirdAccount.address,
        index: 2,
        owner: 7,
      },
    ]);
    expect(config.state.connectors.get("wallet")?.account).toEqual(secondAccount);
  });

  it("selects the first backend account when refreshed connection state has no selected account", async () => {
    const getUser = vi.fn().mockResolvedValue({
      accounts: {
        1: secondAccount.address,
        2: thirdAccount.address,
      },
      index: 7,
      name: "alice-restored",
    });
    const wallet = createTestConnector("wallet", {
      getClient: vi.fn().mockResolvedValue({
        getUser,
      }),
    });
    const config = createTestConfig({
      connectors: [wallet],
      current: "wallet",
      status: ConnectionStatus.Connected,
      user: {
        index: 7,
        status: "active",
        username: "alice",
      },
    });
    config.state.connectors.set("wallet", {
      ...config.state.connectors.get("wallet")!,
      account: undefined,
      accounts: [],
    });

    await refreshAccounts(config, {
      userIndex: 7,
    });

    expect(wallet.getClient).toHaveBeenCalledOnce();
    expect(getUser).toHaveBeenCalledWith({
      userIndexOrName: {
        index: 7,
      },
    });
    expect(config.state.user).toEqual({
      index: 7,
      status: "active",
      username: "alice-restored",
    });
    expect(config.state.connectors.get("wallet")?.accounts).toEqual([
      {
        address: secondAccount.address,
        index: 1,
        owner: 7,
      },
      {
        address: thirdAccount.address,
        index: 2,
        owner: 7,
      },
    ]);
    expect(config.state.connectors.get("wallet")?.account).toEqual({
      address: secondAccount.address,
      index: 1,
      owner: 7,
    });
  });

  it("leaves account state unchanged when backend account refresh fails", async () => {
    const refreshError = new Error("backend user lookup failed");
    const getUser = vi.fn().mockRejectedValue(refreshError);
    const wallet = createTestConnector("wallet", {
      getClient: vi.fn().mockResolvedValue({
        getUser,
      }),
    });
    const config = createTestConfig({
      connectors: [wallet],
      current: "wallet",
      status: ConnectionStatus.Connected,
      user: {
        index: 7,
        status: "active",
        username: "alice",
      },
    });
    setConnectionAccounts(config, "wallet", [firstAccount, secondAccount], secondAccount);
    const beforeUser = config.state.user;
    const beforeConnection = config.state.connectors.get("wallet");

    await expect(
      refreshAccounts(config, {
        userIndex: 7,
      }),
    ).rejects.toBe(refreshError);

    expect(wallet.getClient).toHaveBeenCalledOnce();
    expect(getUser).toHaveBeenCalledWith({
      userIndexOrName: {
        index: 7,
      },
    });
    expect(config.setState).not.toHaveBeenCalled();
    expect(config.state.user).toBe(beforeUser);
    expect(config.state.connectors.get("wallet")).toBe(beforeConnection);
  });

  it("refreshes accounts for an explicit connector without touching the current connection", async () => {
    const walletGetUser = vi.fn().mockResolvedValue({
      accounts: {
        0: firstAccount.address,
      },
      index: 7,
      name: "alice",
    });
    const otherGetUser = vi.fn().mockResolvedValue({
      accounts: {
        0: otherAccount.address,
        2: thirdAccount.address,
      },
      index: 9,
      name: "bob-renamed",
    });
    const wallet = createTestConnector("wallet", {
      getClient: vi.fn().mockResolvedValue({
        getUser: walletGetUser,
      }),
    });
    const other = createTestConnector("other", {
      getClient: vi.fn().mockResolvedValue({
        getUser: otherGetUser,
      }),
    });
    const config = createTestConfig({
      connectors: [wallet, other],
      current: "wallet",
      status: ConnectionStatus.Connected,
      user: {
        index: 7,
        status: "active",
        username: "alice",
      },
    });
    setConnectionAccounts(config, "wallet", [firstAccount, secondAccount], firstAccount);
    setConnectionAccounts(config, "other", [otherAccount], otherAccount);

    await refreshAccounts(config, {
      connectorUId: "other",
      userIndex: 9,
    });

    expect(wallet.getClient).not.toHaveBeenCalled();
    expect(walletGetUser).not.toHaveBeenCalled();
    expect(other.getClient).toHaveBeenCalledOnce();
    expect(otherGetUser).toHaveBeenCalledWith({
      userIndexOrName: {
        index: 9,
      },
    });
    expect(config.state.current).toBe("wallet");
    expect(config.state.user).toEqual({
      index: 7,
      status: "active",
      username: "bob-renamed",
    });
    expect(config.state.connectors.get("wallet")?.account).toEqual(firstAccount);
    expect(config.state.connectors.get("wallet")?.accounts).toEqual([firstAccount, secondAccount]);
    expect(config.state.connectors.get("other")?.accounts).toEqual([
      {
        address: otherAccount.address,
        index: 0,
        owner: 9,
      },
      {
        address: thirdAccount.address,
        index: 2,
        owner: 9,
      },
    ]);
    expect(config.state.connectors.get("other")?.account).toEqual(otherAccount);
  });

  it("refreshes user status from the selected account or explicit address", async () => {
    const getAccountStatus = vi.fn().mockResolvedValue("inactive");
    const wallet = createTestConnector("wallet", {
      getClient: vi.fn().mockResolvedValue({
        getAccountStatus,
      }),
    });
    const config = createTestConfig({
      connectors: [wallet],
      current: "wallet",
      status: ConnectionStatus.Connected,
      user: {
        index: 7,
        status: "active",
        username: "alice",
      },
    });
    setConnectionAccounts(config, "wallet", [firstAccount, secondAccount], firstAccount);

    await refreshUserStatus(config);

    expect(getAccountStatus).toHaveBeenCalledWith({
      address: firstAccount.address,
    });
    expect(config.state.user?.status).toBe("inactive");

    getAccountStatus.mockResolvedValueOnce("active");
    await refreshUserStatus(config, {
      address: secondAccount.address,
    });

    expect(getAccountStatus).toHaveBeenLastCalledWith({
      address: secondAccount.address,
    });
    expect(config.state.user?.status).toBe("active");
  });

  it("refreshes user status through an explicit non-current connector", async () => {
    const walletGetAccountStatus = vi.fn().mockResolvedValue("active");
    const otherGetAccountStatus = vi.fn().mockResolvedValue("inactive");
    const wallet = createTestConnector("wallet", {
      getClient: vi.fn().mockResolvedValue({
        getAccountStatus: walletGetAccountStatus,
      }),
    });
    const other = createTestConnector("other", {
      getClient: vi.fn().mockResolvedValue({
        getAccountStatus: otherGetAccountStatus,
      }),
    });
    const config = createTestConfig({
      connectors: [wallet, other],
      current: "wallet",
      status: ConnectionStatus.Connected,
      user: {
        index: 7,
        status: "active",
        username: "alice",
      },
    });
    setConnectionAccounts(config, "wallet", [firstAccount], firstAccount);
    setConnectionAccounts(config, "other", [otherAccount], otherAccount);

    await refreshUserStatus(config, {
      connectorUId: "other",
    });

    expect(wallet.getClient).not.toHaveBeenCalled();
    expect(walletGetAccountStatus).not.toHaveBeenCalled();
    expect(other.getClient).toHaveBeenCalledOnce();
    expect(otherGetAccountStatus).toHaveBeenCalledWith({
      address: otherAccount.address,
    });
    expect(config.state.user?.status).toBe("inactive");
    expect(config.state.current).toBe("wallet");
  });

  it("leaves user status unchanged when backend status refresh fails", async () => {
    const statusError = new Error("backend account status failed");
    const getAccountStatus = vi.fn().mockRejectedValue(statusError);
    const wallet = createTestConnector("wallet", {
      getClient: vi.fn().mockResolvedValue({
        getAccountStatus,
      }),
    });
    const config = createTestConfig({
      connectors: [wallet],
      current: "wallet",
      status: ConnectionStatus.Connected,
      user: {
        index: 7,
        status: "active",
        username: "alice",
      },
    });
    setConnectionAccounts(config, "wallet", [firstAccount, secondAccount], secondAccount);
    const beforeUser = config.state.user;

    await expect(refreshUserStatus(config)).rejects.toBe(statusError);

    expect(wallet.getClient).toHaveBeenCalledOnce();
    expect(getAccountStatus).toHaveBeenCalledWith({
      address: secondAccount.address,
    });
    expect(config.setState).not.toHaveBeenCalled();
    expect(config.state.user).toBe(beforeUser);
  });

  it("clears user status without querying the backend when no account address is available", async () => {
    const getAccountStatus = vi.fn().mockResolvedValue("active");
    const wallet = createTestConnector("wallet", {
      getClient: vi.fn().mockResolvedValue({
        getAccountStatus,
      }),
    });
    const config = createTestConfig({
      connectors: [wallet],
      current: "wallet",
      status: ConnectionStatus.Connected,
      user: {
        index: 7,
        status: "inactive",
        username: "alice",
      },
    });
    config.state.connectors.set("wallet", {
      ...config.state.connectors.get("wallet")!,
      account: undefined,
      accounts: [],
    });

    await refreshUserStatus(config);

    expect(wallet.getClient).toHaveBeenCalledOnce();
    expect(getAccountStatus).not.toHaveBeenCalled();
    expect(config.state.user).toEqual({
      index: 7,
      status: undefined,
      username: "alice",
    });
  });

  it("delegates balance, account-info, and block lookups to public-client SDK actions", async () => {
    const config = createTestConfig();
    const blockHash = "0x626c6f636b2d616374696f6e00000000000000000000000000000000000000";
    sdkActionMocks.getBalances.mockResolvedValue({
      "bridge/usdc": "42000000",
    });
    sdkActionMocks.getAccountInfo.mockResolvedValue(firstAccount);
    sdkActionMocks.queryBlock.mockResolvedValue({
      blockHeight: 123,
      createdAt: "2026-06-09T08:00:00.000Z",
      hash: blockHash,
    });

    await expect(
      getBalances(config, {
        address: firstAccount.address,
      }),
    ).resolves.toEqual({
      "bridge/usdc": "42000000",
    });
    await expect(
      getAccountInfo(config, {
        address: firstAccount.address,
        height: 99,
      }),
    ).resolves.toBe(firstAccount);
    await expect(
      getBlock(config, {
        height: 123,
      }),
    ).resolves.toEqual({
      hash: blockHash,
      height: "123",
      timestamp: "2026-06-09T08:00:00.000Z",
    });

    expect(sdkActionMocks.getBalances).toHaveBeenCalledWith(
      expect.objectContaining({
        actions: publicActions,
        uid: "public-client",
      }),
      {
        address: firstAccount.address,
      },
    );
    expect(sdkActionMocks.getAccountInfo).toHaveBeenCalledWith(
      expect.objectContaining({
        actions: publicActions,
        uid: "public-client",
      }),
      {
        address: firstAccount.address,
        height: 99,
      },
    );
    expect(sdkActionMocks.queryBlock).toHaveBeenCalledWith(
      expect.objectContaining({
        actions: publicActions,
        uid: "public-client",
      }),
      {
        height: 123,
      },
    );
  });

  it("preserves zero block heights when delegating block lookups", async () => {
    const config = createTestConfig();
    const blockHash = "0x67656e657369732d626c6f636b0000000000000000000000000000000000";
    sdkActionMocks.queryBlock.mockResolvedValue({
      blockHeight: 0,
      createdAt: "2026-06-09T00:00:00.000Z",
      hash: blockHash,
    });

    await expect(
      getBlock(config, {
        height: 0,
      }),
    ).resolves.toEqual({
      hash: blockHash,
      height: "0",
      timestamp: "2026-06-09T00:00:00.000Z",
    });

    expect(sdkActionMocks.queryBlock).toHaveBeenCalledWith(
      expect.objectContaining({
        actions: publicActions,
        uid: "public-client",
      }),
      {
        height: 0,
      },
    );
  });

  it("delegates account-info chain id to the public-client SDK action", async () => {
    const config = createTestConfig();
    sdkActionMocks.getAccountInfo.mockResolvedValue(firstAccount);

    await expect(
      getAccountInfo(config, {
        address: firstAccount.address,
        chainId: "dango-dev-1",
        height: 128,
      }),
    ).resolves.toBe(firstAccount);

    expect(sdkActionMocks.getAccountInfo).toHaveBeenCalledWith(
      expect.objectContaining({
        actions: publicActions,
        uid: "public-client",
      }),
      {
        address: firstAccount.address,
        chainId: "dango-dev-1",
        height: 128,
      },
    );
  });

  it("delegates paginated balance lookups to the public-client SDK action", async () => {
    const config = createTestConfig();
    sdkActionMocks.getBalances.mockResolvedValue({
      "bridge/atom": "100",
    });

    await expect(
      getBalances(config, {
        address: firstAccount.address,
        height: 64,
        limit: 25,
        startAfter: "bridge/usdc",
      }),
    ).resolves.toEqual({
      "bridge/atom": "100",
    });

    expect(sdkActionMocks.getBalances).toHaveBeenCalledWith(
      expect.objectContaining({
        actions: publicActions,
        uid: "public-client",
      }),
      {
        address: firstAccount.address,
        height: 64,
        limit: 25,
        startAfter: "bridge/usdc",
      },
    );
  });

  it("normalizes app config addresses and preserves backend perps parameters", async () => {
    const appConfigClient = {
      getAppConfig: vi.fn().mockResolvedValue({
        addresses: {
          accountFactory: "0x6163636f756e742d666163746f72790000000000",
          dex: {
            pairFactory: "0x706169722d666163746f72790000000000000000",
          },
        },
        owner: "0x6f776e6572000000000000000000000000000000",
      }),
      getCodeHash: vi
        .fn()
        .mockResolvedValue("0x636f64652d68617368000000000000000000000000000000000000000000"),
      getPerpsPairParams: vi.fn().mockResolvedValue({
        "perp/btcusd": {
          enabled: true,
        },
      }),
      getPerpsParam: vi.fn().mockResolvedValue({
        tradingFeeRate: "0.001",
      }),
      uid: "app-config-client",
    };
    const config = createTestConfig();
    config.getClient.mockReturnValue({
      extend: vi.fn(() => appConfigClient),
      uid: "base-client",
    });

    await expect(getAppConfig(config)).resolves.toEqual({
      accountFactory: {
        codeHash: "0x636f64652d68617368000000000000000000000000000000000000000000",
      },
      addresses: {
        "0x6163636f756e742d666163746f72790000000000": "accountFactory",
        "0x706169722d666163746f72790000000000000000": "pairFactory",
        accountFactory: "0x6163636f756e742d666163746f72790000000000",
        pairFactory: "0x706169722d666163746f72790000000000000000",
      },
      owner: "0x6f776e6572000000000000000000000000000000",
      perpsPairs: {
        "perp/btcusd": {
          enabled: true,
        },
      },
      perpsParam: {
        tradingFeeRate: "0.001",
      },
    });
    expect(appConfigClient.getAppConfig).toHaveBeenCalledOnce();
    expect(appConfigClient.getCodeHash).toHaveBeenCalledOnce();
    expect(appConfigClient.getPerpsPairParams).toHaveBeenCalledOnce();
    expect(appConfigClient.getPerpsParam).toHaveBeenCalledOnce();
  });

  it("rejects app config loading when any backend config lookup fails", async () => {
    const queryError = new Error("perps pair params unavailable");
    const appConfigClient = {
      getAppConfig: vi.fn().mockResolvedValue({
        addresses: {
          accountFactory: "0x6163636f756e742d666163746f72790000000000",
        },
        owner: "0x6f776e6572000000000000000000000000000000",
      }),
      getCodeHash: vi
        .fn()
        .mockResolvedValue("0x636f64652d68617368000000000000000000000000000000000000000000"),
      getPerpsPairParams: vi.fn().mockRejectedValue(queryError),
      getPerpsParam: vi.fn().mockResolvedValue({
        tradingFeeRate: "0.001",
      }),
      uid: "app-config-client",
    };
    const config = createTestConfig();
    config.getClient.mockReturnValue({
      extend: vi.fn(() => appConfigClient),
      uid: "base-client",
    });

    await expect(getAppConfig(config)).rejects.toBe(queryError);

    expect(appConfigClient.getAppConfig).toHaveBeenCalledOnce();
    expect(appConfigClient.getCodeHash).toHaveBeenCalledOnce();
    expect(appConfigClient.getPerpsPairParams).toHaveBeenCalledOnce();
    expect(appConfigClient.getPerpsParam).toHaveBeenCalledOnce();
  });

  it("resolves connectors and signer clients from current or explicit connection ids", async () => {
    const walletClient = {
      uid: "wallet-signer-client",
    };
    const otherClient = {
      uid: "other-signer-client",
    };
    const wallet = createTestConnector("wallet", {
      getClient: vi.fn().mockResolvedValue(walletClient),
    });
    const other = createTestConnector("other", {
      getClient: vi.fn().mockResolvedValue(otherClient),
    });
    const config = createTestConfig({
      connectors: [wallet, other],
      current: "wallet",
      status: ConnectionStatus.Connected,
    });

    expect(getConnector(config)).toBe(wallet);
    expect(getConnector(config, { connectorUId: "other" })).toBe(other);
    await expect(getConnectorClient(config)).resolves.toBe(walletClient);
    await expect(getConnectorClient(config, { connectorUId: "other" })).resolves.toBe(otherClient);

    expect(wallet.getClient).toHaveBeenCalledOnce();
    expect(other.getClient).toHaveBeenCalledOnce();
  });

  it("resolves explicit signer clients without relying on the current connector", async () => {
    const walletClient = {
      uid: "wallet-signer-client",
    };
    const wallet = createTestConnector("wallet", {
      getClient: vi.fn().mockResolvedValue(walletClient),
    });
    const config = createTestConfig({
      connectors: [wallet],
      current: null,
      status: ConnectionStatus.Disconnected,
    });

    expect(getConnector(config, { connectorUId: "wallet" })).toBe(wallet);
    await expect(getConnectorClient(config, { connectorUId: "wallet" })).resolves.toBe(
      walletClient,
    );

    expect(wallet.getClient).toHaveBeenCalledOnce();
  });

  it("propagates signer client failures without falling back to the current connector", async () => {
    const signerError = new Error("wallet provider unavailable");
    const wallet = createTestConnector("wallet", {
      getClient: vi.fn().mockResolvedValue({
        uid: "wallet-signer-client",
      }),
    });
    const session = createTestConnector("session", {
      getClient: vi.fn().mockRejectedValue(signerError),
    });
    const config = createTestConfig({
      connectors: [wallet, session],
      current: "wallet",
      status: ConnectionStatus.Connected,
    });
    const beforeState = config.state;

    await expect(
      getConnectorClient(config, {
        connectorUId: "session",
      }),
    ).rejects.toBe(signerError);

    expect(session.getClient).toHaveBeenCalledOnce();
    expect(wallet.getClient).not.toHaveBeenCalled();
    expect(config.state).toBe(beforeState);
  });

  it("surfaces connector lookup errors for missing current or explicit connections", async () => {
    const disconnectedConfig = createTestConfig();

    expect(() => getConnector(disconnectedConfig)).toThrow("No connector found for current chain");
    await expect(getConnectorClient(disconnectedConfig)).rejects.toThrow(
      "No connector found for current chain",
    );

    const missingConnectionConfig = createTestConfig({
      current: "wallet",
      status: ConnectionStatus.Connected,
    });

    expect(() => getConnector(missingConnectionConfig)).toThrow("No connection found");
    await expect(
      getConnectorClient(missingConnectionConfig, {
        connectorUId: "wallet",
      }),
    ).rejects.toThrow("No connection found");
  });

  it("reads chain id, stabilizes connector snapshots, and extends the public client", () => {
    const wallet = createTestConnector("wallet");
    const other = createTestConnector("other");
    const config = createTestConfig({
      chainId: "dango-dev-1",
      connectors: [wallet, other],
    });

    expect(getChainId(config)).toBe("dango-dev-1");

    const connectors = getConnectors(config);
    expect(connectors).toEqual([wallet, other]);
    expect(getConnectors(config)).toBe(connectors);
    expect(
      getConnectors(
        createTestConfig({
          chainId: "dango-dev-1",
          connectors: [wallet, other],
        }),
      ),
    ).toBe(connectors);

    const reorderedConfig = createTestConfig({
      chainId: "dango-dev-1",
      connectors: [other, wallet],
    });
    expect(getConnectors(reorderedConfig)).toEqual([other, wallet]);
    expect(getConnectors(reorderedConfig)).not.toBe(connectors);

    const publicClient = getPublicClient(config) as { actions: unknown; uid: string };

    expect(config.getClient).toHaveBeenCalledOnce();
    expect(publicClient.uid).toBe("public-client");
    expect(publicClient.actions).toBe(publicActions);
  });

  it("watches selected account changes until unsubscribed", () => {
    const wallet = createTestConnector("wallet");
    const config = createTestConfig({
      connectors: [wallet],
      current: "wallet",
      status: ConnectionStatus.Connected,
      user: {
        index: 7,
        status: "active",
        username: "alice",
      },
    });
    setConnectionAccounts(config, "wallet", [firstAccount, secondAccount], firstAccount);

    const onChange = vi.fn();
    const unsubscribe = watchAccount(config, {
      onChange,
    });

    changeAccount(config, {
      address: secondAccount.address,
    });

    expect(onChange).toHaveBeenCalledWith(
      expect.objectContaining({
        account: secondAccount,
        connector: expect.objectContaining({
          uid: "wallet",
        }),
      }),
      expect.objectContaining({
        account: firstAccount,
        connector: expect.objectContaining({
          uid: "wallet",
        }),
      }),
    );

    unsubscribe();
    changeAccount(config, {
      address: firstAccount.address,
    });

    expect(onChange).toHaveBeenCalledTimes(1);
  });

  it("watches backend user status changes without requiring an account switch", () => {
    const wallet = createTestConnector("wallet");
    const config = createTestConfig({
      connectors: [wallet],
      current: "wallet",
      status: ConnectionStatus.Connected,
      user: {
        index: 7,
        status: "active",
        username: "alice",
      },
    });
    setConnectionAccounts(config, "wallet", [firstAccount, secondAccount], firstAccount);

    const onChange = vi.fn();
    const unsubscribe = watchAccount(config, {
      onChange,
    });

    config.setState((state) => ({
      ...state,
      user: {
        ...state.user!,
        status: "inactive",
      },
    }));

    expect(onChange).toHaveBeenCalledWith(
      expect.objectContaining({
        account: firstAccount,
        isUserActive: false,
        userStatus: "inactive",
      }),
      expect.objectContaining({
        account: firstAccount,
        isUserActive: true,
        userStatus: "active",
      }),
    );

    unsubscribe();
    config.setState((state) => ({
      ...state,
      user: {
        ...state.user!,
        status: "active",
      },
    }));

    expect(onChange).toHaveBeenCalledTimes(1);
  });

  it("watches chain id changes until unsubscribed", () => {
    const config = createTestConfig({
      chainId: "dango-dev-1",
    });
    const onChange = vi.fn();
    const unsubscribe = watchChainId(config, {
      onChange,
    });

    config.setState((state) => ({
      ...state,
      chainId: "dango-test-2",
    }));

    expect(onChange).toHaveBeenCalledWith("dango-test-2", "dango-dev-1");

    unsubscribe();
    config.setState((state) => ({
      ...state,
      chainId: "dango-test-3",
    }));

    expect(onChange).toHaveBeenCalledTimes(1);
  });

  it("watches public client changes by derived client uid", () => {
    const config = createTestConfig();
    const onChange = vi.fn();
    const unsubscribe = watchPublicClient(config, {
      onChange,
    });

    config.setState((state) => ({
      ...state,
      chainId: "dango-dev-same-client",
    }));

    expect(onChange).not.toHaveBeenCalled();

    config.getClient.mockImplementation(() => ({
      extend: vi.fn(() => ({
        uid: "next-public-client",
      })),
      uid: "next-base-client",
    }));
    config.setState((state) => ({
      ...state,
      chainId: "dango-dev-next-client",
    }));

    expect(onChange).toHaveBeenCalledWith(
      {
        uid: "next-public-client",
      },
      expect.objectContaining({
        uid: "public-client",
      }),
    );

    unsubscribe();
    config.getClient.mockImplementation(() => ({
      extend: vi.fn(() => ({
        uid: "ignored-public-client",
      })),
      uid: "ignored-base-client",
    }));
    config.setState((state) => ({
      ...state,
      chainId: "dango-dev-ignored-client",
    }));

    expect(onChange).toHaveBeenCalledTimes(1);
  });
});
