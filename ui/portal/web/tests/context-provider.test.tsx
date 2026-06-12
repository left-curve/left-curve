import { cleanup, render, renderHook, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { CoinStore } from "../../../store/src/stores/coinStore";
import { DangoRemoteProvider, DangoStoreProvider, useConfig } from "../../../store/src";
import { ConnectionStatus } from "../../../store/src/types/store";
import { createTestConfig } from "./mocks/store-config";

import type { ReactNode } from "react";
import type { HydrateProps } from "../../../store/src/hydrate";
import type { Connection } from "../../../store/src/types/connector";
import type { NativeCoin } from "../../../store/src/types/coin";

const contextMocks = vi.hoisted(() => ({
  hydrateProps: [] as HydrateProps[],
}));

vi.mock("../../../store/src/hydrate.js", () => ({
  Hydrate: (props: HydrateProps & { children: ReactNode }) => {
    contextMocks.hydrateProps.push(props);
    return <>{props.children}</>;
  },
}));

const usdcCoin = {
  decimals: 6,
  denom: "bridge/usdc",
  name: "USD Coin",
  symbol: "USDC",
  type: "native",
} satisfies NativeCoin;

function ConfigConsumer() {
  const config = useConfig();

  return (
    <>
      <div data-testid="chain-id">{config.chain.id}</div>
      <div data-testid="connector-count">{config.connectors.length}</div>
    </>
  );
}

function installDangoRuntime(connection?: Omit<Connection, "connector">) {
  Object.defineProperty(window, "dango", {
    configurable: true,
    value: {
      chain: {
        id: "dango-dev-1",
        name: "Devnet",
        url: "https://rpc.dango.test",
      },
      coins: {
        [usdcCoin.denom]: usdcCoin,
      },
      connection,
    },
  });
}

describe("store context providers", () => {
  beforeEach(() => {
    contextMocks.hydrateProps = [];
    CoinStore.setState({
      byDenom: {},
      bySymbol: {},
    });
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
    Reflect.deleteProperty(window, "dango");
    CoinStore.setState({
      byDenom: {},
      bySymbol: {},
    });
  });

  it("throws a clear error when useConfig is rendered outside a provider", () => {
    expect(() => renderHook(() => useConfig())).toThrow("GrunnectProvider not found");
  });

  it("prefers an explicit config parameter over context", () => {
    const explicitConfig = createTestConfig({
      chainId: "dango-explicit-1",
    });

    const { result } = renderHook(() => useConfig({ config: explicitConfig }));

    expect(result.current).toBe(explicitConfig);
  });

  it("passes configured state through DangoStoreProvider and Hydrate", () => {
    const config = createTestConfig({
      chainId: "dango-provider-1",
    });
    const initialState = {
      ...config.state,
      chainId: "dango-provider-initial",
      status: ConnectionStatus.Reconnecting,
    };

    render(
      <DangoStoreProvider config={config} initialState={initialState} reconnectOnMount={false}>
        <ConfigConsumer />
      </DangoStoreProvider>,
    );

    expect(screen.getByTestId("chain-id")).toHaveTextContent("dango-provider-1");
    expect(contextMocks.hydrateProps).toHaveLength(1);
    expect(contextMocks.hydrateProps[0]).toMatchObject({
      config,
      initialState,
      reconnectOnMount: false,
    });
  });

  it("creates a connected remote config from the native Dango runtime", () => {
    const connection = {
      account: {
        address: "0x72656d6f74652d6163636f756e742d300000",
        index: 0,
        owner: 7,
      },
      accounts: [
        {
          address: "0x72656d6f74652d6163636f756e742d300000",
          index: 0,
          owner: 7,
        },
      ],
      chainId: "dango-dev-1",
      keyHash: "0x72656d6f74652d6b65792d6861736800000000000000000000000000000000",
    } satisfies Omit<Connection, "connector">;
    installDangoRuntime(connection);

    render(
      <DangoRemoteProvider>
        <ConfigConsumer />
      </DangoRemoteProvider>,
    );

    const hydrateProps = contextMocks.hydrateProps[0];
    const connector = hydrateProps.config.connectors[0];

    expect(screen.getByTestId("chain-id")).toHaveTextContent("dango-dev-1");
    expect(screen.getByTestId("connector-count")).toHaveTextContent("1");
    expect(connector).toMatchObject({
      id: "remote",
      type: "remote",
    });
    expect(hydrateProps).toMatchObject({
      reconnectOnMount: false,
    });
    expect(hydrateProps.initialState).toMatchObject({
      chainId: "dango-dev-1",
      current: connector.uid,
      status: ConnectionStatus.Connected,
      user: {
        index: 7,
        status: "active",
        username: "User #7",
      },
    });
    expect(hydrateProps.initialState?.connectors.get(connector.uid)).toMatchObject({
      ...connection,
      connector,
    });
    expect(CoinStore.getState().byDenom).toEqual({
      "bridge/usdc": usdcCoin,
    });
  });

  it("creates a disconnected remote config when native runtime has no connection", () => {
    installDangoRuntime();

    render(
      <DangoRemoteProvider>
        <ConfigConsumer />
      </DangoRemoteProvider>,
    );

    expect(contextMocks.hydrateProps[0].initialState).toMatchObject({
      chainId: "dango-dev-1",
      connectors: new Map(),
      current: null,
      isMipdLoaded: true,
      status: ConnectionStatus.Disconnected,
      user: undefined,
    });
  });
});
