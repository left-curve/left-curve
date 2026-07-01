import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import type React from "react";

import { Bridge } from "../src/components/bridge/Bridge";

const swapperCssVariableColors = {
  "--color-ink-primary-900": "rgb(17, 17, 17)",
  "--color-surface-primary-rice": "rgb(255, 252, 246)",
  "--color-surface-secondary-rice": "rgb(255, 249, 240)",
  "--color-surface-tertiary-rice": "rgb(255, 243, 225)",
  "--color-surface-quaternary-rice": "rgb(243, 229, 191)",
};

const bridgeUiMocks = vi.hoisted(() => ({
  allowanceMutation: {
    isPending: false,
    mutate: vi.fn(),
    mutateAsync: vi.fn(),
  },
  allowanceQueryData: 0n,
  changeCoin: vi.fn(),
  changeAction: vi.fn(),
  chainId: "dango-dev-1",
  connector: null as { icon: string; id: string; name: string } | null,
  deposit: {
    isPending: false,
    mutate: vi.fn(),
    mutateAsync: vi.fn(),
  },
  evmBalances: {} as Record<string, string>,
  evmWalletAddress: "0x4444444444444444444444444444444444444444",
  getPrice: vi.fn(),
  hasRouter: true,
  isConnected: true,
  network: "11155111" as "11155111" | "bitcoin",
  refreshBridgeBalances: vi.fn(),
  refreshUserStatus: vi.fn(),
  refetchEvmBalances: vi.fn(),
  reset: vi.fn(),
  setConnectorId: vi.fn(),
  setNetwork: vi.fn(),
  showModal: vi.fn(),
  swapperDestroy: vi.fn(),
  swapperOptions: [] as Array<{
    depositWalletAddress?: string;
    dstChainId?: string;
    dstTokenAddr?: string;
    iframeAttributes?: Record<string, string | undefined>;
    integratorId?: string;
    onEvent?: (event: { type: string }) => void;
    styles?: { componentStyles?: Record<string, string | undefined>; themeMode?: string };
    supportedDepositOptions?: string[];
  }>,
  userStatus: "active" as "active" | "inactive",
  withdraw: {
    isPending: false,
    mutate: vi.fn(),
    mutateAsync: vi.fn(),
  },
  withdrawFeeData: "0.25",
}));

vi.mock("@swapper-finance/deposit-sdk", () => {
  class SwapperIframe {
    private readonly iframe: HTMLIFrameElement;

    constructor(options: {
      container: HTMLElement;
      iframeAttributes?: Record<string, string | undefined>;
      onEvent?: (event: { type: string }) => void;
    }) {
      bridgeUiMocks.swapperOptions.push(options);
      const wrapper = document.createElement("div");
      wrapper.style.height = options.iframeAttributes?.height ?? "560px";
      this.iframe = document.createElement("iframe");
      this.iframe.style.height = options.iframeAttributes?.height ?? "560px";
      this.iframe.title = "Swapper deposit";
      wrapper.appendChild(this.iframe);
      options.container.appendChild(wrapper);
    }

    getIframe() {
      return this.iframe;
    }

    destroy() {
      bridgeUiMocks.swapperDestroy();
      this.iframe.remove();
    }
  }

  return {
    SwapperIframe,
    WidgetEventName: {
      TRANSACTION_SUCCESS: "transaction_success",
    },
  };
});

const usdcCoin = {
  decimals: 6,
  denom: "bridge/usdc",
  logoURI: "/usdc.png",
  name: "USD Coin",
  symbol: "USDC",
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
    ConnectWalletWithModal: ({ onWalletSelected }: { onWalletSelected: (id: string) => void }) => (
      <button onClick={() => onWalletSelected("browser-wallet")} type="button">
        connect wallet
      </button>
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
      Authenticate: "authenticate",
      BridgeDeposit: "BridgeDeposit",
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
      networks: Array<{ id: string; name: string; withdrawLiquidity?: string }>;
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
            {[network.name, network.withdrawLiquidity].filter(Boolean).join(" ")}
          </option>
        ))}
      </select>
    ),
    useApp: () => ({
      showModal: bridgeUiMocks.showModal,
    }),
    useTheme: () => ({
      theme: "light",
    }),
  };
});

vi.mock("@left-curve/store", () => ({
  useAccount: () => ({
    account: bridgeUiMocks.isConnected
      ? {
          address: "0x6272696467657573657200000000000000000000",
        }
      : undefined,
    isConnected: bridgeUiMocks.isConnected,
    refreshUserStatus: bridgeUiMocks.refreshUserStatus,
    userStatus: bridgeUiMocks.userStatus,
  }),
  useAppConfig: () => ({
    data: {
      minimumDeposit: {
        "bridge/usdc": "1000000",
      },
    },
  }),
  useBalances: () => ({
    data: {
      "bridge/usdc": "10000000",
    },
    refetch: bridgeUiMocks.refreshBridgeBalances,
  }),
  useBridgeEvmDeposit: () => ({
    allowanceMutation: bridgeUiMocks.allowanceMutation,
    allowanceQuery: {
      data: bridgeUiMocks.allowanceQueryData,
    },
    deposit: bridgeUiMocks.deposit,
    wallet: {
      data: {
        account: {
          address: bridgeUiMocks.evmWalletAddress,
        },
      },
    },
  }),
  useBridgeState: ({ action }: { action: "deposit" | "withdraw" }) => ({
    action,
    changeCoin: bridgeUiMocks.changeCoin,
    coin: usdcCoin,
    coins: [usdcCoin],
    config: bridgeUiMocks.hasRouter ? bridgeConfig : { chain: bridgeConfig.chain },
    connector: bridgeUiMocks.connector,
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
    setConnectorId: bridgeUiMocks.setConnectorId,
    setNetwork: bridgeUiMocks.setNetwork,
  }),
  useBridgeWithdraw: () => ({
    withdraw: bridgeUiMocks.withdraw,
    withdrawFee: {
      data: bridgeUiMocks.withdrawFeeData,
    },
  }),
  useEvmBalances: () => ({
    data: bridgeUiMocks.evmBalances,
    refetch: bridgeUiMocks.refetchEvmBalances,
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

    bridgeUiMocks.allowanceQueryData = 0n;
    bridgeUiMocks.chainId = "dango-dev-1";
    bridgeUiMocks.connector = null;
    bridgeUiMocks.evmBalances = {
      "bridge/usdc": "10000000",
    };
    bridgeUiMocks.hasRouter = true;
    bridgeUiMocks.network = "11155111";
    bridgeUiMocks.swapperOptions = [];
    bridgeUiMocks.userStatus = "active";
    bridgeUiMocks.withdrawFeeData = "0.25";
    bridgeUiMocks.getPrice.mockImplementation((amount: string, denom: string) => {
      return `${amount}:${denom}`;
    });
    bridgeUiMocks.isConnected = true;
    for (const [variable, color] of Object.entries(swapperCssVariableColors)) {
      document.documentElement.style.setProperty(variable, color);
    }
    vi.stubEnv("PUBLIC_SWAPPER_INTEGRATOR_ID", "test-integrator");
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
    for (const variable of Object.keys(swapperCssVariableColors)) {
      document.documentElement.style.removeProperty(variable);
    }
    vi.unstubAllEnvs();
  });

  it("connects an EVM wallet before showing the deposit form", () => {
    renderBridgeDeposit();

    fireEvent.click(screen.getByRole("button", { name: "connect wallet" }));

    expect(bridgeUiMocks.setConnectorId).toHaveBeenCalledWith("browser-wallet");
    expect(bridgeUiMocks.showModal).not.toHaveBeenCalled();
  });

  it("mounts the swapper deposit iframe from more deposit options", async () => {
    renderBridgeDeposit();

    const moreDepositOptions = screen.getByRole("button", {
      name: new RegExp(m["bridge.deposit.moreOptions.title"]()),
    });

    expect(moreDepositOptions).toHaveTextContent(m["bridge.deposit.moreOptions.fee"]());

    fireEvent.click(moreDepositOptions);

    const swapperIframe = await screen.findByTitle("Swapper deposit");
    expect(swapperIframe).toBeInTheDocument();
    expect(swapperIframe.closest(".bg-surface-secondary-rice")).toBeInTheDocument();
    expect(
      screen.queryByRole("combobox", {
        name: m["bridge.selectCoin"](),
      }),
    ).not.toBeInTheDocument();

    await waitFor(() => expect(bridgeUiMocks.swapperOptions).toHaveLength(1));
    expect(bridgeUiMocks.swapperOptions[0]).toEqual(
      expect.objectContaining({
        depositWalletAddress: "0x6272696467657573657200000000000000000000",
        dstChainId: "dango",
        dstTokenAddr: "usdc",
        integratorId: "test-integrator",
        styles: expect.objectContaining({
          componentStyles: expect.objectContaining({
            backgroundColor: swapperCssVariableColors["--color-surface-secondary-rice"],
            borderRadius: "12px",
            primaryButtonTextColor: swapperCssVariableColors["--color-surface-primary-rice"],
            surfaceAltColor: swapperCssVariableColors["--color-surface-quaternary-rice"],
            surfaceColor: swapperCssVariableColors["--color-surface-tertiary-rice"],
            textColor: swapperCssVariableColors["--color-ink-primary-900"],
            width: "100%",
          }),
          themeMode: "light",
        }),
        iframeAttributes: expect.objectContaining({
          allowtransparency: "true",
          height: "560px",
          width: "100%",
        }),
        supportedDepositOptions: ["transferCrypto", "depositWithCash", "walletDeposit"],
      }),
    );
    await waitFor(() => {
      expect(swapperIframe.getAttribute("style")).toContain("background-color: transparent");
      expect(swapperIframe.getAttribute("style")).toContain("display: block");
      expect(swapperIframe.parentElement?.style.height).toBe("fit-content");
    });

    bridgeUiMocks.swapperOptions[0].onEvent?.({ type: "transaction_success" });
    expect(bridgeUiMocks.refreshBridgeBalances).toHaveBeenCalledOnce();
    expect(bridgeUiMocks.refreshUserStatus).toHaveBeenCalledOnce();

    fireEvent.click(
      screen.getByRole("button", {
        name: m["bridge.deposit.moreOptions.back"](),
      }),
    );

    expect(bridgeUiMocks.swapperDestroy).toHaveBeenCalledOnce();
    expect(
      screen.queryByRole("combobox", {
        name: m["bridge.selectCoin"](),
      }),
    ).not.toBeInTheDocument();
    expect(
      screen.getByRole("combobox", {
        name: m["bridge.selectNetwork"](),
      }),
    ).toBeInTheDocument();
  });

  it("keeps more deposit options visible and asks users to log in before opening swapper", () => {
    bridgeUiMocks.isConnected = false;

    renderBridgeDeposit();

    const moreDepositOptions = screen.getByRole("button", {
      name: new RegExp(m["bridge.deposit.moreOptions.title"]()),
    });

    expect(moreDepositOptions).toBeInTheDocument();
    expect(moreDepositOptions).toHaveTextContent(m["common.signin"]());
    expect(moreDepositOptions).not.toHaveTextContent(m["bridge.deposit.moreOptions.cta"]());
    expect(screen.queryByTitle("Swapper deposit")).not.toBeInTheDocument();

    fireEvent.click(moreDepositOptions);

    expect(bridgeUiMocks.showModal).toHaveBeenCalledWith("authenticate", { action: "signin" });
  });

  it("shows the Bitcoin deposit address without an EVM wallet flow", () => {
    bridgeUiMocks.network = "bitcoin";

    renderBridgeDeposit();

    expect(screen.getByText(m["bridge.depositAddress"]())).toBeInTheDocument();
    expect(screen.getByText("bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh")).toBeInTheDocument();
    expect(screen.getByText(m["bridge.rateLimitWarning"]())).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "connect wallet" })).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: m["bridge.deposit.title"]() }),
    ).not.toBeInTheDocument();
    expect(bridgeUiMocks.showModal).not.toHaveBeenCalled();
  });

  it("shows an unsupported warning instead of an EVM deposit form when config has no router", () => {
    bridgeUiMocks.hasRouter = false;

    renderBridgeDeposit();

    expect(screen.getByText(m["bridge.unsupportedAsset"]())).toBeInTheDocument();
    expect(screen.getByText(m["bridge.rateLimitWarning"]())).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "connect wallet" })).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: m["bridge.deposit.title"]() }),
    ).not.toBeInTheDocument();
    expect(bridgeUiMocks.showModal).not.toHaveBeenCalled();
  });

  it("hides the deposit coin selector and keeps the withdraw coin selector", () => {
    bridgeUiMocks.chainId = "dango-1";

    const { unmount } = renderBridgeDeposit();

    expect(
      screen.queryByRole("combobox", {
        name: m["bridge.selectCoin"](),
      }),
    ).not.toBeInTheDocument();
    expect(
      screen.getByRole("combobox", {
        name: m["bridge.selectNetwork"](),
      }),
    ).toBeInTheDocument();

    unmount();
    renderBridgeWithdraw();

    const withdrawCoinSelector = screen.getByRole("combobox", {
      name: m["bridge.selectCoin"](),
    }) as HTMLSelectElement;

    expect(Array.from(withdrawCoinSelector.options).map((option) => option.textContent)).toEqual([
      "USDC",
    ]);
  });

  it("routes coin and network selector changes through the bridge state callbacks", () => {
    renderBridgeWithdraw();

    fireEvent.change(screen.getByRole("combobox", { name: m["bridge.selectCoin"]() }), {
      target: {
        value: "bridge/usdc",
      },
    });
    fireEvent.change(screen.getByRole("combobox", { name: m["bridge.selectNetwork"]() }), {
      target: {
        value: "bitcoin",
      },
    });

    expect(bridgeUiMocks.changeCoin).toHaveBeenCalledWith("bridge/usdc");
    expect(bridgeUiMocks.setNetwork).toHaveBeenCalledWith("bitcoin");
  });

  it("shows the withdraw liquidity in each network option", () => {
    renderBridgeWithdraw();

    const networkSelector = screen.getByRole("combobox", {
      name: m["bridge.selectNetwork"](),
    }) as HTMLSelectElement;

    expect(Array.from(networkSelector.options).map((option) => option.textContent)).toEqual([
      `Sepolia ${m["bridge.withdrawLiquidity"]()}: 10 USDC`,
      `Bitcoin ${m["bridge.withdrawLiquidity"]()}: 10 USDC`,
    ]);
  });

  it("shows a login button instead of withdraw selectors when disconnected", () => {
    bridgeUiMocks.isConnected = false;

    renderBridgeWithdraw();

    const loginButton = screen.getByRole("button", { name: m["common.signin"]() });

    expect(loginButton).toBeInTheDocument();
    expect(
      screen.queryByRole("combobox", {
        name: m["bridge.selectCoin"](),
      }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("combobox", {
        name: m["bridge.selectNetwork"](),
      }),
    ).not.toBeInTheDocument();

    fireEvent.click(loginButton);

    expect(bridgeUiMocks.showModal).toHaveBeenCalledWith("authenticate", { action: "signin" });
  });

  it("opens deposit confirmation with allowance requirement and refresh reset wiring", async () => {
    bridgeUiMocks.allowanceQueryData = 1_000_000n;
    bridgeUiMocks.connector = {
      icon: "/wallet.svg",
      id: "browser-wallet",
      name: "Browser Wallet",
    };

    renderBridgeDeposit();

    fireEvent.change(screen.getByRole("textbox", { name: "amount" }), {
      target: {
        value: "3.25",
      },
    });

    expect(screen.getByRole("textbox", { name: m["bridge.youGet"]() })).toHaveValue("3.25");
    expect(screen.getByText("3.25:bridge/usdc")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: m["bridge.deposit.title"]() }));

    expect(bridgeUiMocks.showModal).toHaveBeenCalledWith(
      "BridgeDeposit",
      expect.objectContaining({
        allowanceMutation: bridgeUiMocks.allowanceMutation,
        amount: "3.25",
        coin: usdcCoin,
        config: bridgeConfig,
        deposit: bridgeUiMocks.deposit,
        requiresAllowance: true,
      }),
    );

    const [, modalProps] = bridgeUiMocks.showModal.mock.calls.at(-1) as [
      string,
      { reset: () => void },
    ];

    await act(async () => {
      modalProps.reset();
    });

    expect(bridgeUiMocks.refetchEvmBalances).toHaveBeenCalledOnce();
    expect(bridgeUiMocks.reset).toHaveBeenCalledOnce();
  });

  it("opens deposit confirmation without approval when the EVM allowance covers the parsed amount", () => {
    bridgeUiMocks.allowanceQueryData = 5_000_000n;
    bridgeUiMocks.connector = {
      icon: "/wallet.svg",
      id: "browser-wallet",
      name: "Browser Wallet",
    };

    renderBridgeDeposit();

    fireEvent.change(screen.getByRole("textbox", { name: "amount" }), {
      target: {
        value: "3.25",
      },
    });
    fireEvent.click(screen.getByRole("button", { name: m["bridge.deposit.title"]() }));

    expect(bridgeUiMocks.showModal).toHaveBeenCalledWith(
      "BridgeDeposit",
      expect.objectContaining({
        allowanceMutation: bridgeUiMocks.allowanceMutation,
        amount: "3.25",
        coin: usdcCoin,
        config: bridgeConfig,
        deposit: bridgeUiMocks.deposit,
        requiresAllowance: false,
      }),
    );
  });

  it("does not open deposit confirmation for a zero amount", () => {
    bridgeUiMocks.allowanceQueryData = 5_000_000n;
    bridgeUiMocks.connector = {
      icon: "/wallet.svg",
      id: "browser-wallet",
      name: "Browser Wallet",
    };

    renderBridgeDeposit();

    const depositButton = screen.getByRole("button", { name: m["bridge.deposit.title"]() });

    expect(depositButton).toBeDisabled();

    fireEvent.click(depositButton);

    expect(bridgeUiMocks.showModal).not.toHaveBeenCalled();
  });

  it("blocks inactive account deposits below the chain minimum and allows the configured amount", async () => {
    bridgeUiMocks.allowanceQueryData = 5_000_000n;
    bridgeUiMocks.connector = {
      icon: "/wallet.svg",
      id: "browser-wallet",
      name: "Browser Wallet",
    };
    bridgeUiMocks.userStatus = "inactive";

    renderBridgeDeposit();

    const amountInput = screen.getByRole("textbox", { name: "amount" });
    const depositButton = screen.getByRole("button", { name: m["bridge.deposit.title"]() });

    fireEvent.change(amountInput, {
      target: {
        value: "0.5",
      },
    });

    expect(
      screen.getByRole("alert", {
        name: "",
      }),
    ).toHaveTextContent(m["bridge.activeAccount"]({ amount: "1 USDC" }));
    expect(depositButton).toBeDisabled();

    fireEvent.click(depositButton);

    expect(bridgeUiMocks.showModal).not.toHaveBeenCalled();

    fireEvent.change(amountInput, {
      target: {
        value: "1",
      },
    });

    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    expect(depositButton).toBeEnabled();

    fireEvent.click(depositButton);

    expect(bridgeUiMocks.showModal).toHaveBeenCalledWith(
      "BridgeDeposit",
      expect.objectContaining({
        amount: "1",
        coin: usdcCoin,
        config: bridgeConfig,
        requiresAllowance: false,
      }),
    );
  });

  it("sets a destination address through the modal callback, subtracts fees, and opens withdraw confirmation", async () => {
    renderBridgeWithdraw();

    expect(screen.getByText(m["bridge.rateLimitWarning"]())).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: m["bridge.setDestinationAddress"]() }));

    expect(bridgeUiMocks.showModal).toHaveBeenCalledWith(
      "DestinationWallet",
      expect.objectContaining({
        onAddressSet: expect.any(Function),
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
      destinationProps.onAddressSet("0x4444444444444444444444444444444444444444", "Browser Wallet");
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
      destinationProps.onAddressSet("0x4444444444444444444444444444444444444444", "Browser Wallet");
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
      destinationProps.onAddressSet("0x5555555555555555555555555555555555555555", "Browser Wallet");
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
