import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { useChainId } from "../../../store/src/hooks/useChainId";
import { useConnect } from "../../../store/src/hooks/useConnect";
import { useConnectors } from "../../../store/src/hooks/useConnectors";
import { useDisconnect } from "../../../store/src/hooks/useDisconnect";
import { usePublicClient } from "../../../store/src/hooks/usePublicClient";
import { ConnectionStatus } from "../../../store/src/types/store";
import { createTestConfig, createTestConnector } from "./mocks/store-config";
import { createQueryClientWrapper } from "./utils/query-client";

const sdkMocks = vi.hoisted(() => ({
  publicActions: {
    name: "publicActions",
  },
}));

vi.mock("@left-curve/sdk", async (importOriginal) => {
  const actual = await importOriginal<object>();

  return {
    ...actual,
    publicActions: sdkMocks.publicActions,
  };
});

describe("connection hooks", () => {
  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("exposes stable connectors and live chain id updates from config state", async () => {
    const first = createTestConnector("first");
    const second = createTestConnector("second");
    const config = createTestConfig({
      chainId: "dango-dev-1",
      connectors: [first, second],
    });

    const connectors = renderHook(() => useConnectors({ config }));
    const chainId = renderHook(() => useChainId({ config }));

    expect(connectors.result.current).toEqual([first, second]);
    expect(chainId.result.current).toBe("dango-dev-1");

    act(() => {
      config.setState((state) => ({
        ...state,
        chainId: "dango-test-2",
      }));
    });

    await waitFor(() => expect(chainId.result.current).toBe("dango-test-2"));
  });

  it("connects through the selected connector and emits connecting state", async () => {
    const connector = createTestConnector("wallet");
    const config = createTestConfig({
      connectors: [connector],
    });

    const { result } = renderHook(() => useConnect({ config }), {
      wrapper: createQueryClientWrapper(),
    });

    expect(result.current.connectors).toEqual([connector]);

    await act(async () => {
      await result.current.connectAsync({
        chainId: "dango-dev-1",
        challenge: "signin-challenge",
        connector,
        userIndex: 7,
      });
    });

    expect(config.setState).toHaveBeenCalledWith(expect.any(Function));
    expect(config.state.status).toBe(ConnectionStatus.Connecting);
    expect(connector.emitter.emit).toHaveBeenCalledWith("message", { type: "connecting" });
    expect(connector.connect).toHaveBeenCalledWith({
      chainId: "dango-dev-1",
      challenge: "signin-challenge",
      userIndex: 7,
    });
  });

  it("passes backend user index zero through the selected connector", async () => {
    const connector = createTestConnector("wallet");
    const config = createTestConfig({
      connectors: [connector],
    });

    const { result } = renderHook(() => useConnect({ config }), {
      wrapper: createQueryClientWrapper(),
    });

    await act(async () => {
      await result.current.connectAsync({
        chainId: "dango-dev-1",
        challenge: "signin-challenge",
        connector,
        userIndex: 0,
      });
    });

    expect(connector.emitter.emit).toHaveBeenCalledWith("message", { type: "connecting" });
    expect(connector.connect).toHaveBeenCalledWith({
      chainId: "dango-dev-1",
      challenge: "signin-challenge",
      userIndex: 0,
    });
    expect(config.state.status).toBe(ConnectionStatus.Connecting);
  });

  it("restores the correct status when connector login fails", async () => {
    const connector = createTestConnector("wallet", {
      connect: vi.fn().mockRejectedValue(new Error("rejected")),
    });
    const connectedConfig = createTestConfig({
      connectors: [connector],
      current: "wallet",
      status: ConnectionStatus.Connected,
    });

    const connected = renderHook(() => useConnect({ config: connectedConfig }), {
      wrapper: createQueryClientWrapper(),
    });

    await expect(
      connected.result.current.connectAsync({
        chainId: "dango-dev-1",
        challenge: "signin-challenge",
        connector,
        userIndex: 7,
      }),
    ).rejects.toThrow("rejected");

    expect(connectedConfig.state.status).toBe(ConnectionStatus.Connected);

    const disconnectedConfig = createTestConfig({
      connectors: [],
      status: ConnectionStatus.Disconnected,
    });
    const rejectedConnector = createTestConnector("new-wallet", {
      connect: vi.fn().mockRejectedValue(new Error("missing account")),
    });
    const disconnected = renderHook(() => useConnect({ config: disconnectedConfig }), {
      wrapper: createQueryClientWrapper(),
    });

    await expect(
      disconnected.result.current.connectAsync({
        chainId: "dango-dev-1",
        challenge: "signin-challenge",
        connector: rejectedConnector,
        userIndex: 7,
      }),
    ).rejects.toThrow("missing account");

    expect(disconnectedConfig.state.status).toBe(ConnectionStatus.Disconnected);
  });

  it("resets the connect mutation when config transitions from connected to disconnected", async () => {
    const connector = createTestConnector("wallet", {
      connect: vi.fn().mockRejectedValue(new Error("rejected")),
    });
    const config = createTestConfig({
      connectors: [connector],
      current: "wallet",
      status: ConnectionStatus.Connected,
    });

    const { result } = renderHook(() => useConnect({ config }), {
      wrapper: createQueryClientWrapper(),
    });

    await expect(
      result.current.connectAsync({
        chainId: "dango-dev-1",
        challenge: "signin-challenge",
        connector,
        userIndex: 7,
      }),
    ).rejects.toThrow("rejected");
    await waitFor(() => expect(result.current.isError).toBe(true));

    act(() => {
      config.setState((state) => ({
        ...state,
        status: ConnectionStatus.Disconnected,
      }));
    });

    await waitFor(() => expect(result.current.isError).toBe(false));
  });

  it("disconnects the current connector and clears connection state when none remain", async () => {
    const connector = createTestConnector("wallet");
    const config = createTestConfig({
      connectors: [connector],
      current: "wallet",
      status: ConnectionStatus.Connected,
    });

    const { result } = renderHook(() => useDisconnect({ config }), {
      wrapper: createQueryClientWrapper(),
    });

    await act(async () => {
      await result.current.disconnectAsync({});
    });

    expect(connector.disconnect).toHaveBeenCalledOnce();
    expect(config.state.connectors.size).toBe(0);
    expect(config.state.status).toBe(ConnectionStatus.Disconnected);
  });

  it("disconnects a selected connector while preserving remaining connections", async () => {
    const first = createTestConnector("first");
    const second = createTestConnector("second");
    const config = createTestConfig({
      connectors: [first, second],
      current: "first",
      status: ConnectionStatus.Connected,
    });

    const { result } = renderHook(() => useDisconnect({ config }), {
      wrapper: createQueryClientWrapper(),
    });

    await act(async () => {
      await result.current.disconnectAsync({
        connectorUId: "second",
      });
    });

    expect(second.disconnect).toHaveBeenCalledOnce();
    expect(first.disconnect).not.toHaveBeenCalled();
    expect([...config.state.connectors.keys()]).toEqual(["first"]);
    expect(config.state.status).toBe(ConnectionStatus.Connected);
  });

  it("leaves the active connection intact when asked to disconnect an unknown connector", async () => {
    const connector = createTestConnector("wallet");
    const config = createTestConfig({
      connectors: [connector],
      current: "wallet",
      status: ConnectionStatus.Connected,
    });

    const { result } = renderHook(() => useDisconnect({ config }), {
      wrapper: createQueryClientWrapper(),
    });

    await act(async () => {
      await result.current.disconnectAsync({
        connectorUId: "missing-wallet",
      });
    });

    expect(connector.disconnect).not.toHaveBeenCalled();
    expect([...config.state.connectors.keys()]).toEqual(["wallet"]);
    expect(config.state.current).toBe("wallet");
    expect(config.state.status).toBe(ConnectionStatus.Connected);
  });

  it("derives a public client from the configured base client and updates on uid changes", async () => {
    const config = createTestConfig();
    const { result } = renderHook(() => usePublicClient({ config }));

    expect(result.current).toEqual({
      actions: sdkMocks.publicActions,
      uid: "public-client",
    });

    config.getClient.mockImplementation(() => ({
      extend: vi.fn((actions: unknown) => ({
        actions,
        uid: "next-public-client",
      })),
      uid: "next-base-client",
    }));

    act(() => {
      config.setState((state) => ({
        ...state,
        chainId: "dango-dev-2",
      }));
    });

    await waitFor(() =>
      expect(result.current).toEqual({
        actions: sdkMocks.publicActions,
        uid: "next-public-client",
      }),
    );
  });
});
