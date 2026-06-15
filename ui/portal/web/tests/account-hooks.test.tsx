import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { useAccount } from "../../../store/src/hooks/useAccount";
import { ConnectionStatus } from "../../../store/src/types/store";
import { createTestConfig, createTestConnector } from "./mocks/store-config";

import type { Account } from "@left-curve/types";

const firstAccount = {
  address: "0x6669727374000000000000000000000000000000",
  index: 0,
  owner: 7,
} as Account;

const secondAccount = {
  address: "0x7365636f6e640000000000000000000000000000",
  index: 1,
  owner: 7,
} as Account;

function setConnectionAccounts(
  config: ReturnType<typeof createTestConfig>,
  connectorUid: string,
  accounts: Account[],
) {
  const connection = config.state.connectors.get(connectorUid);
  if (!connection) throw new Error(`Missing ${connectorUid} connection`);

  config.state.connectors.set(connectorUid, {
    ...connection,
    account: accounts[0],
    accounts,
    keyHash: `${connectorUid}-key-hash`,
  });
}

describe("useAccount", () => {
  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("projects connected account state and live status transitions from config", async () => {
    const connector = createTestConnector("wallet");
    const config = createTestConfig({
      connectors: [connector],
      current: "wallet",
      status: ConnectionStatus.Connected,
      user: {
        index: 7,
        status: "active",
        username: "alice",
      },
    });
    setConnectionAccounts(config, "wallet", [firstAccount, secondAccount]);

    const { result } = renderHook(() => useAccount({ config }));

    expect(result.current).toMatchObject({
      account: firstAccount,
      accounts: [firstAccount, secondAccount],
      chain: {
        id: "dango-dev-1",
      },
      chainId: "dango-dev-1",
      connector,
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
    expect(result.current.changeAccount).toEqual(expect.any(Function));
    expect(result.current.refreshAccounts).toEqual(expect.any(Function));
    expect(result.current.refreshUserStatus).toEqual(expect.any(Function));

    act(() => {
      config.setState((state) => ({
        ...state,
        status: ConnectionStatus.Reconnecting,
      }));
    });

    await waitFor(() =>
      expect(result.current).toMatchObject({
        account: firstAccount,
        isConnected: false,
        isReconnecting: true,
        status: "reconnecting",
      }),
    );
    expect(result.current.changeAccount).toBeUndefined();

    act(() => {
      config.setState((state) => ({
        ...state,
        status: ConnectionStatus.Connecting,
      }));
    });

    await waitFor(() =>
      expect(result.current).toMatchObject({
        account: firstAccount,
        isConnecting: true,
        isReconnecting: false,
        status: "connecting",
      }),
    );

    act(() => {
      config.setState((state) => ({
        ...state,
        status: ConnectionStatus.Disconnected,
      }));
    });

    await waitFor(() =>
      expect(result.current).toMatchObject({
        account: undefined,
        isConnected: false,
        isDisconnected: true,
        status: "disconnected",
        username: undefined,
      }),
    );
  });

  it("updates the selected account when changeAccount is called with a known account address", async () => {
    const connector = createTestConnector("wallet");
    const config = createTestConfig({
      connectors: [connector],
      current: "wallet",
      status: ConnectionStatus.Connected,
      user: {
        index: 7,
        status: "active",
        username: "alice",
      },
    });
    setConnectionAccounts(config, "wallet", [firstAccount, secondAccount]);

    const { result } = renderHook(() => useAccount({ config }));

    expect(result.current.account).toEqual(firstAccount);

    act(() => {
      result.current.changeAccount?.("0x7365636f6e640000000000000000000000000000");
    });

    await waitFor(() => expect(result.current.account).toEqual(secondAccount));

    act(() => {
      result.current.changeAccount?.("0x6d697373696e6700000000000000000000000000");
    });

    expect(result.current.account).toEqual(secondAccount);
  });

  it("refreshes user status from the currently selected account after account changes", async () => {
    const getAccountStatus = vi.fn().mockResolvedValue("inactive");
    const connector = createTestConnector("wallet", {
      getClient: vi.fn().mockResolvedValue({
        getAccountStatus,
      }),
    });
    const config = createTestConfig({
      connectors: [connector],
      current: "wallet",
      status: ConnectionStatus.Connected,
      user: {
        index: 7,
        status: "active",
        username: "alice",
      },
    });
    setConnectionAccounts(config, "wallet", [firstAccount, secondAccount]);

    const { result } = renderHook(() => useAccount({ config }));

    expect(result.current.account).toEqual(firstAccount);

    act(() => {
      result.current.changeAccount?.(secondAccount.address);
    });

    await waitFor(() => expect(result.current.account).toEqual(secondAccount));

    act(() => {
      result.current.refreshUserStatus?.();
    });

    await waitFor(() => expect(result.current.userStatus).toBe("inactive"));
    expect(getAccountStatus).toHaveBeenCalledWith({
      address: secondAccount.address,
    });
  });

  it("refreshes accounts and user status through the active connector client", async () => {
    const getUser = vi.fn().mockResolvedValue({
      accounts: {
        0: "0x6669727374000000000000000000000000000000",
        1: "0x7468697264000000000000000000000000000000",
      },
      index: 7,
      name: "alice-renamed",
    });
    const getAccountStatus = vi.fn().mockResolvedValue("inactive");
    const connector = createTestConnector("wallet", {
      getClient: vi.fn().mockResolvedValue({
        getAccountStatus,
        getUser,
      }),
    });
    const config = createTestConfig({
      connectors: [connector],
      current: "wallet",
      status: ConnectionStatus.Connected,
      user: {
        index: 7,
        status: "active",
        username: "alice",
      },
    });
    setConnectionAccounts(config, "wallet", [firstAccount, secondAccount]);

    const { result } = renderHook(() => useAccount({ config }));

    expect(result.current.accounts?.map((account) => account.address)).toEqual([
      "0x6669727374000000000000000000000000000000",
      "0x7365636f6e640000000000000000000000000000",
    ]);

    await act(async () => {
      await result.current.refreshAccounts?.();
    });

    await waitFor(() =>
      expect(result.current.accounts?.map((account) => account.address)).toEqual([
        "0x6669727374000000000000000000000000000000",
        "0x7468697264000000000000000000000000000000",
      ]),
    );
    expect(getUser).toHaveBeenCalledWith({
      userIndexOrName: {
        index: 7,
      },
    });
    expect(result.current).toMatchObject({
      account: {
        address: "0x6669727374000000000000000000000000000000",
        index: 0,
        owner: 7,
      },
      username: "alice-renamed",
    });
    expect(result.current.userStatus).toBe("active");

    act(() => {
      result.current.refreshUserStatus?.();
    });

    await waitFor(() => expect(result.current.userStatus).toBe("inactive"));
    expect(getAccountStatus).toHaveBeenCalledWith({
      address: "0x6669727374000000000000000000000000000000",
    });
    expect(result.current.isUserActive).toBe(false);
  });

  it("refreshes backend accounts when the active user index is zero", async () => {
    const zeroAccountAddress = "0x7a65726f2d757365722d6163636f756e74000000";
    const getUser = vi.fn().mockResolvedValue({
      accounts: {
        0: zeroAccountAddress,
      },
      index: 0,
      name: "zero-renamed",
    });
    const connector = createTestConnector("wallet", {
      getClient: vi.fn().mockResolvedValue({
        getUser,
      }),
    });
    const config = createTestConfig({
      connectors: [connector],
      current: "wallet",
      status: ConnectionStatus.Connected,
      user: {
        index: 0,
        status: "active",
        username: "zero",
      },
    });
    setConnectionAccounts(config, "wallet", [
      {
        address: zeroAccountAddress,
        index: 0,
        owner: 0,
      } as Account,
    ]);

    const { result } = renderHook(() => useAccount({ config }));

    await act(async () => {
      await result.current.refreshAccounts?.();
    });

    await waitFor(() =>
      expect(getUser).toHaveBeenCalledWith({
        userIndexOrName: {
          index: 0,
        },
      }),
    );
    expect(result.current).toMatchObject({
      account: {
        address: zeroAccountAddress,
        index: 0,
        owner: 0,
      },
      userIndex: 0,
    });
  });

  it("does not refresh backend accounts before a user index is selected", async () => {
    const getClient = vi.fn().mockResolvedValue({
      getUser: vi.fn(),
    });
    const connector = createTestConnector("wallet", { getClient });
    const config = createTestConfig({
      connectors: [connector],
      current: "wallet",
      status: ConnectionStatus.Connected,
      user: undefined,
    });
    setConnectionAccounts(config, "wallet", [firstAccount]);

    const { result } = renderHook(() => useAccount({ config }));

    await act(async () => {
      await result.current.refreshAccounts?.();
    });

    expect(getClient).not.toHaveBeenCalled();
    expect(result.current.accounts).toEqual([firstAccount]);
  });

  it("preserves the selected account across backend account refreshes when it still exists", async () => {
    const refreshedFirstAccount = {
      address: "0x6669727374000000000000000000000000000000",
      index: 0,
      owner: 7,
    } as Account;
    const refreshedSecondAccount = {
      address: "0x7365636f6e640000000000000000000000000000",
      index: 3,
      owner: 7,
    } as Account;
    const getUser = vi.fn().mockResolvedValue({
      accounts: {
        0: refreshedFirstAccount.address,
        3: refreshedSecondAccount.address,
      },
      index: 7,
      name: "alice-updated",
    });
    const connector = createTestConnector("wallet", {
      getClient: vi.fn().mockResolvedValue({
        getUser,
      }),
    });
    const config = createTestConfig({
      connectors: [connector],
      current: "wallet",
      status: ConnectionStatus.Connected,
      user: {
        index: 7,
        status: "active",
        username: "alice",
      },
    });
    setConnectionAccounts(config, "wallet", [firstAccount, secondAccount]);
    const connection = config.state.connectors.get("wallet");
    if (!connection) throw new Error("Missing wallet connection");
    config.state.connectors.set("wallet", {
      ...connection,
      account: secondAccount,
    });

    const { result } = renderHook(() => useAccount({ config }));

    expect(result.current.account).toEqual(secondAccount);

    await act(async () => {
      await result.current.refreshAccounts?.();
    });

    await waitFor(() =>
      expect(result.current.accounts).toEqual([refreshedFirstAccount, refreshedSecondAccount]),
    );
    expect(result.current).toMatchObject({
      account: refreshedSecondAccount,
      username: "alice-updated",
    });
    expect(getUser).toHaveBeenCalledWith({
      userIndexOrName: {
        index: 7,
      },
    });
  });

  it("returns disconnected state when the current connector has no connection", () => {
    const connector = createTestConnector("wallet");
    const config = createTestConfig({
      connectors: [connector],
      current: "missing",
      status: ConnectionStatus.Connected,
      user: {
        index: 7,
        status: "active",
        username: "alice",
      },
    });

    const { result } = renderHook(() => useAccount({ config }));

    expect(result.current).toMatchObject({
      account: undefined,
      isConnected: false,
      isDisconnected: true,
      status: "disconnected",
      userIndex: undefined,
      username: undefined,
    });
  });
});
