import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import type React from "react";

import { Bridge } from "../src/components/bridge/Bridge";

const bridgeUiMocks = vi.hoisted(() => ({
  changeCoin: vi.fn(),
  changeAction: vi.fn(),
  accountAddress: "0x6272696467657573657200000000000000000000",
  chainId: "dango-dev-1",
  getPrice: vi.fn(),
  hasRouter: true,
  isConnected: true,
  network: "11155111" as "11155111" | "bitcoin",
  refreshBalances: vi.fn(),
  refreshUserStatus: vi.fn(),
  reset: vi.fn(),
  setNetwork: vi.fn(),
  showModal: vi.fn(),
  swapperOptions: [] as Array<{
    container: HTMLElement;
    depositWalletAddress: string;
    dstChainId: string;
    dstTokenAddr: string;
    integratorId: string;
    iframeAttributes?: {
      borderRadius?: string;
      height?: string;
      minWidth?: string;
      title?: string;
      width?: string;
    };
    onEvent?: (event: { type: string; data?: unknown }) => void;
    styles?: unknown;
    supportedDepositOptions?: string[];
  }>,
  theme: "light" as "light" | "dark",
  userStatus: "active" as "active" | "inactive",
  withdraw: {
    isPending: false,
    mutate: vi.fn(),
    mutateAsync: vi.fn(),
  },
  withdrawFeeData: "0.25",
}));

const usdcCoin = {
  decimals: 6,
  denom: "bridge/usdc",
  logoURI: "/usdc.png",
  name: "USD Coin",
  symbol: "USDC",
  type: "native",
};

const ethCoin = {
  decimals: 18,
  denom: "bridge/eth",
  logoURI: "/eth.png",
  name: "Ether",
  symbol: "ETH",
  type: "native",
};

const bridgeConfig = {
  chain: {
    id: 11155111,
    name: "Sepolia",
  },
  router: {
    address: "0x1111111111111111111111111111111111111111",
    coin: "0x2222222222222222222222222222222222222222",
    domain: 17,
    remote: {
      warp: {
        contract: "0x3333333333333333333333333333333333333333",
        domain: 17,
      },
    },
  },
};

vi.mock("@swapper-finance/deposit-sdk", () => {
  class SwapperIframeMock {
    private readonly iframe: HTMLIFrameElement;

    constructor(options: (typeof bridgeUiMocks.swapperOptions)[number]) {
      bridgeUiMocks.swapperOptions.push(options);

      const src = new URL("https://deposit.swapper.finance/");
      src.searchParams.set("integratorId", options.integratorId);
      src.searchParams.set("dstChainId", options.dstChainId);
      src.searchParams.set("dstTokenAddr", options.dstTokenAddr);
      src.searchParams.set("depositWalletAddress", options.depositWalletAddress);
      src.searchParams.set(
        "supportedDepositOptions",
        JSON.stringify(options.supportedDepositOptions),
      );
      src.searchParams.set("styles", JSON.stringify(options.styles));

      this.iframe = document.createElement("iframe");
      this.iframe.title = options.iframeAttributes?.title || "Swapper Deposit Widget";
      this.iframe.src = src.toString();
      options.container.appendChild(this.iframe);
    }

    destroy() {
      this.iframe.remove();
    }
  }

  return {
    SwapperIframe: SwapperIframeMock,
    WidgetEventName: {
      TRANSACTION_SUCCESS: "transaction_success",
    },
  };
});

vi.mock("@tanstack/react-router", () => ({
  Link: ({ children }: React.PropsWithChildren<{ to: string }>) => <>{children}</>,
}));

vi.mock("@left-curve/applets-kit", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@left-curve/applets-kit")>();
  const React = await import("react");

  return {
    ...actual,
    AssetInputWithRange: ({
      asset,
      bottomComponent,
      controllers,
      extendValidation,
      label,
      name,
      shouldValidate,
    }: {
      asset: typeof usdcCoin;
      balances: Record<string, string>;
      bottomComponent?: React.ReactNode;
      controllers: ReturnType<typeof actual.useInputs>;
      extendValidation?: (value: string) => boolean | string;
      label: React.ReactNode;
      name: string;
      shouldValidate?: boolean;
      showRange?: boolean;
    }) => (
      <label>
        <span>{label}</span>
        <input
          aria-label={typeof label === "string" ? label : name}
          onChange={(event) => {
            const value = event.currentTarget.value;
            controllers.setValue(name, value);

            if (!shouldValidate || !extendValidation) return;

            const validationResult = extendValidation(value);
            controllers.setError(
              name,
              validationResult === true ? undefined : validationResult || "Value is not valid",
            );
          }}
          value={controllers.inputs[name]?.value ?? ""}
        />
        <span>{asset.symbol}</span>
        {controllers.errors[name] ? <p role="alert">{controllers.errors[name]}</p> : null}
        {bottomComponent}
      </label>
    ),
    AuthenticatedButton: ({ children }: { children: React.ReactElement }) =>
      bridgeUiMocks.isConnected
        ? children
        : React.cloneElement(children, { children: m["common.signin"]() }),
    CoinSelector: ({
      coins,
      isDisabled,
      label,
      onChange,
      value,
    }: {
      coins: Array<typeof usdcCoin>;
      isDisabled?: boolean;
      label: string;
      onChange: (denom: string) => void;
      value?: string;
    }) => (
      <select
        aria-label={label}
        disabled={isDisabled}
        onChange={(event) => onChange(event.currentTarget.value)}
        value={value}
      >
        {coins.map((coin) => (
          <option key={coin.denom} value={coin.denom}>
            {coin.symbol}
          </option>
        ))}
      </select>
    ),
    FormattedNumber: ({
      as: Component = "span",
      number,
    }: {
      as?: React.ElementType;
      formatOptions?: unknown;
      number: string | number;
    }) => <Component>{String(number)}</Component>,
    IconDisconnect: ({ onClick }: { className?: string; onClick?: () => void }) => (
      <button aria-label="disconnect wallet" onClick={onClick} type="button" />
    ),
    Input: ({
      bottomComponent,
      insideBottomComponent,
      label,
      startContent,
      value,
    }: {
      bottomComponent?: React.ReactNode;
      classNames?: unknown;
      insideBottomComponent?: React.ReactNode;
      label?: React.ReactNode;
      placeholder?: string;
      readOnly?: boolean;
      startContent?: React.ReactNode;
      startText?: string;
      value?: string;
    }) => (
      <label>
        <span>{label}</span>
        {startContent}
        <input aria-label={typeof label === "string" ? label : "input"} readOnly value={value} />
        {insideBottomComponent}
        {bottomComponent}
      </label>
    ),
    Modals: {
      ...actual.Modals,
      BridgeWithdraw: "BridgeWithdraw",
      DestinationWallet: "DestinationWallet",
    },
    NetworkSelector: ({
      isDisabled,
      label,
      networks,
      onNetworkChange,
      value,
    }: {
      isDisabled?: boolean;
      label: string;
      networks: Array<{ id: string; name: string }>;
      onNetworkChange: (network: { id: string }) => void;
      value?: string;
    }) => (
      <select
        aria-label={label}
        disabled={isDisabled}
        onChange={(event) => onNetworkChange({ id: event.currentTarget.value })}
        value={value}
      >
        {networks.map((network) => (
          <option key={network.id} value={network.id}>
            {network.name}
          </option>
        ))}
      </select>
    ),
    useApp: () => ({
      showModal: bridgeUiMocks.showModal,
    }),
    useTheme: () => ({
      theme: bridgeUiMocks.theme,
    }),
  };
});

vi.mock("@left-curve/store", () => ({
  useAccount: () => ({
    account: bridgeUiMocks.accountAddress ? { address: bridgeUiMocks.accountAddress } : undefined,
    isConnected: bridgeUiMocks.isConnected,
    refreshUserStatus: bridgeUiMocks.refreshUserStatus,
    userStatus: bridgeUiMocks.userStatus,
  }),
  useBalances: () => ({
    data: {
      "bridge/usdc": "10000000",
    },
    refetch: bridgeUiMocks.refreshBalances,
  }),
  useBridgeState: ({ action }: { action: "deposit" | "withdraw" }) => ({
    action,
    changeCoin: bridgeUiMocks.changeCoin,
    coin: usdcCoin,
    coins: [usdcCoin, ethCoin],
    config: bridgeUiMocks.hasRouter ? bridgeConfig : { chain: bridgeConfig.chain },
    network: bridgeUiMocks.network,
    networks: [
      {
        id: "11155111",
        name: "Sepolia",
      },
      {
        id: "bitcoin",
        name: "Bitcoin",
      },
    ],
    reset: bridgeUiMocks.reset,
    setNetwork: bridgeUiMocks.setNetwork,
  }),
  useBridgeWithdraw: () => ({
    withdraw: bridgeUiMocks.withdraw,
    withdrawFee: {
      data: bridgeUiMocks.withdrawFeeData,
    },
  }),
  useConfig: () => ({
    chain: {
      id: bridgeUiMocks.chainId,
    },
  }),
  usePrices: () => ({
    getPrice: bridgeUiMocks.getPrice,
  }),
}));

function renderBridgeWithdraw() {
  return render(
    <Bridge action="withdraw" changeAction={bridgeUiMocks.changeAction}>
      <Bridge.Withdraw />
    </Bridge>,
  );
}

function renderBridgeDeposit() {
  return render(
    <Bridge action="deposit" changeAction={bridgeUiMocks.changeAction}>
      <Bridge.Deposit />
    </Bridge>,
  );
}

describe("bridge UI", () => {
  beforeEach(() => {
    class ResizeObserverMock {
      disconnect = vi.fn();
      observe = vi.fn();
      unobserve = vi.fn();
    }

    Object.defineProperty(globalThis, "ResizeObserver", {
      configurable: true,
      value: ResizeObserverMock,
    });

    bridgeUiMocks.accountAddress = "0x6272696467657573657200000000000000000000";
    bridgeUiMocks.chainId = "dango-dev-1";
    bridgeUiMocks.hasRouter = true;
    bridgeUiMocks.isConnected = true;
    bridgeUiMocks.network = "11155111";
    bridgeUiMocks.swapperOptions = [];
    bridgeUiMocks.theme = "light";
    bridgeUiMocks.userStatus = "active";
    bridgeUiMocks.withdrawFeeData = "0.25";
    vi.stubEnv("PUBLIC_SWAPPER_INTEGRATOR_ID", "test-swapper-integrator");
    bridgeUiMocks.getPrice.mockImplementation((amount: string, denom: string) => {
      return `${amount}:${denom}`;
    });
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
    vi.unstubAllEnvs();
  });

  it("uses Dango dark colors for Swapper deposits in dark mode", () => {
    bridgeUiMocks.theme = "dark";

    renderBridgeDeposit();

    const iframe = screen.getByTitle("Swapper Deposit Widget") as HTMLIFrameElement;
    const src = new URL(iframe.src);

    expect(src.origin).toBe("https://deposit.swapper.finance");
    expect(src.searchParams.get("integratorId")).toBe("test-swapper-integrator");
    expect(src.searchParams.get("dstChainId")).toBe("dango");
    expect(src.searchParams.get("dstTokenAddr")).toBe("usdc");
    expect(src.searchParams.get("depositWalletAddress")).toBe(bridgeUiMocks.accountAddress);
    expect(JSON.parse(src.searchParams.get("supportedDepositOptions") || "[]")).toEqual([
      "transferCrypto",
      "depositWithCash",
      "walletDeposit",
    ]);
    expect(JSON.parse(src.searchParams.get("styles") || "{}")).toEqual({
      themeMode: "dark",
      componentStyles: {
        primaryColor: "#F57589",
        primaryButtonTextColor: "#2D2C2A",
        accentColor: "#F57589",
        sphereColor: "#F57589",
        backgroundColor: "#2D2C2A",
        surfaceColor: "#363432",
        surfaceAltColor: "#4D4B48",
        textColor: "#FFFCF6",
      },
    });
  });

  it("keeps the POC Swapper colors in light mode", () => {
    bridgeUiMocks.theme = "light";

    renderBridgeDeposit();

    const iframe = screen.getByTitle("Swapper Deposit Widget") as HTMLIFrameElement;
    const src = new URL(iframe.src);

    expect(JSON.parse(src.searchParams.get("styles") || "{}")).toEqual({
      themeMode: "light",
      componentStyles: {
        primaryColor: "#F57589",
        primaryButtonTextColor: "#FFFCF6",
        accentColor: "#F57589",
        sphereColor: "#F57589",
        backgroundColor: "#fffcf6",
        surfaceColor: "#f5efdf",
        surfaceAltColor: "#fffaed",
        textColor: "#292929",
      },
    });
  });

  it("does not render Swapper when the integrator ID is not configured", () => {
    vi.stubEnv("PUBLIC_SWAPPER_INTEGRATOR_ID", "");

    renderBridgeDeposit();

    expect(screen.getByText(m["common.failedToLoad"]())).toBeInTheDocument();
    expect(screen.queryByTitle("Swapper Deposit Widget")).not.toBeInTheDocument();
    expect(bridgeUiMocks.swapperOptions).toEqual([]);
  });

  it("prompts for a Dango connection instead of rendering Swapper without an account address", () => {
    bridgeUiMocks.accountAddress = "";
    bridgeUiMocks.isConnected = false;

    renderBridgeDeposit();

    expect(screen.getByRole("button", { name: m["common.signin"]() })).toBeInTheDocument();
    expect(screen.queryByTitle("Swapper Deposit Widget")).not.toBeInTheDocument();
  });

  it("refreshes balances and user status after a Swapper transaction succeeds", () => {
    renderBridgeDeposit();

    act(() => {
      bridgeUiMocks.swapperOptions[0]?.onEvent?.({
        type: "transaction_success",
        data: {
          depositOption: "transferCrypto",
          txHash: "0xabc",
        },
      });
    });

    expect(bridgeUiMocks.refreshBalances).toHaveBeenCalledOnce();
    expect(bridgeUiMocks.refreshUserStatus).toHaveBeenCalledOnce();
  });

  it("keeps bridgeable coins available for withdrawals", () => {
    renderBridgeWithdraw();

    const withdrawCoinSelector = screen.getByRole("combobox", {
      name: m["bridge.selectCoin"](),
    }) as HTMLSelectElement;

    expect(Array.from(withdrawCoinSelector.options).map((option) => option.textContent)).toEqual([
      "USDC",
      "ETH",
    ]);
  });

  it("routes coin and network selector changes through the bridge state callbacks", () => {
    renderBridgeWithdraw();

    fireEvent.change(screen.getByRole("combobox", { name: m["bridge.selectCoin"]() }), {
      target: {
        value: "bridge/eth",
      },
    });
    fireEvent.change(screen.getByRole("combobox", { name: m["bridge.selectNetwork"]() }), {
      target: {
        value: "bitcoin",
      },
    });

    expect(bridgeUiMocks.changeCoin).toHaveBeenCalledWith("bridge/eth");
    expect(bridgeUiMocks.setNetwork).toHaveBeenCalledWith("bitcoin");
  });

  it("sets a destination address through the modal callback, subtracts fees, and opens withdraw confirmation", async () => {
    renderBridgeWithdraw();

    expect(screen.getByText(m["bridge.rateLimitWarning"]())).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: m["bridge.setDestinationAddress"]() }));

    expect(bridgeUiMocks.showModal).toHaveBeenCalledWith(
      "DestinationWallet",
      expect.objectContaining({
        network: "11155111",
      }),
    );

    const [, destinationProps] = bridgeUiMocks.showModal.mock.calls[0];

    await act(async () => {
      destinationProps.onAddressSet(
        "0x4444444444444444444444444444444444444444",
        "Browser Wallet",
        "/wallet.svg",
      );
    });

    expect(screen.getByText("0x4444444444444444444444444444444444444444")).toBeInTheDocument();
    expect(screen.getByText("Browser Wallet")).toBeInTheDocument();

    fireEvent.change(screen.getByRole("textbox", { name: m["bridge.youWithdraw"]() }), {
      target: {
        value: "3.25",
      },
    });

    expect(screen.getByRole("textbox", { name: m["bridge.youGet"]() })).toHaveValue("3");
    expect(screen.getByText("0.25")).toBeInTheDocument();
    expect(screen.getByText("3:bridge/usdc")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: m["bridge.withdraw.title"]() }));

    expect(bridgeUiMocks.showModal).toHaveBeenLastCalledWith("BridgeWithdraw", {
      amount: "3.25",
      coin: usdcCoin,
      config: bridgeConfig,
      fee: "0.25",
      recipient: "0x4444444444444444444444444444444444444444",
      withdraw: bridgeUiMocks.withdraw,
    });
  });

  it("clamps the withdrawal receive preview to zero when backend fees exceed the amount", async () => {
    bridgeUiMocks.withdrawFeeData = "0.25";

    renderBridgeWithdraw();

    fireEvent.click(screen.getByRole("button", { name: m["bridge.setDestinationAddress"]() }));
    const [, destinationProps] = bridgeUiMocks.showModal.mock.calls[0];

    await act(async () => {
      destinationProps.onAddressSet("0x4444444444444444444444444444444444444444");
    });

    fireEvent.change(screen.getByRole("textbox", { name: m["bridge.youWithdraw"]() }), {
      target: {
        value: "0.1",
      },
    });

    expect(screen.getByRole("textbox", { name: m["bridge.youGet"]() })).toHaveValue("0");
    expect(screen.getByText("0.25")).toBeInTheDocument();
    expect(screen.getByText("0:bridge/usdc")).toBeInTheDocument();
  });

  it("preserves a backend zero withdrawal fee through preview and confirmation", async () => {
    bridgeUiMocks.withdrawFeeData = "0";

    renderBridgeWithdraw();

    fireEvent.click(screen.getByRole("button", { name: m["bridge.setDestinationAddress"]() }));
    const [, destinationProps] = bridgeUiMocks.showModal.mock.calls[0];

    await act(async () => {
      destinationProps.onAddressSet("0x4444444444444444444444444444444444444444");
    });

    fireEvent.change(screen.getByRole("textbox", { name: m["bridge.youWithdraw"]() }), {
      target: {
        value: "3.25",
      },
    });

    expect(screen.getByRole("textbox", { name: m["bridge.youGet"]() })).toHaveValue("3.25");
    expect(screen.getByText("> 0")).toBeInTheDocument();
    expect(screen.getByText("3.25:bridge/usdc")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: m["bridge.withdraw.title"]() }));

    expect(bridgeUiMocks.showModal).toHaveBeenLastCalledWith("BridgeWithdraw", {
      amount: "3.25",
      coin: usdcCoin,
      config: bridgeConfig,
      fee: "0",
      recipient: "0x4444444444444444444444444444444444444444",
      withdraw: bridgeUiMocks.withdraw,
    });
  });

  it("clears the selected destination address and recipient when disconnected", async () => {
    renderBridgeWithdraw();

    fireEvent.click(screen.getByRole("button", { name: m["bridge.setDestinationAddress"]() }));
    const [, destinationProps] = bridgeUiMocks.showModal.mock.calls[0];

    await act(async () => {
      destinationProps.onAddressSet("0x5555555555555555555555555555555555555555");
    });

    expect(screen.getByText("0x5555555555555555555555555555555555555555")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "disconnect wallet" }));

    expect(
      screen.queryByText("0x5555555555555555555555555555555555555555"),
    ).not.toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: m["bridge.setDestinationAddress"]() }),
    ).toBeInTheDocument();
  });

  it("routes tab changes through the bridge container action callback", () => {
    renderBridgeWithdraw();

    fireEvent.click(screen.getByRole("button", { name: "deposit" }));

    expect(bridgeUiMocks.changeAction).toHaveBeenCalledWith("deposit");
  });
});
