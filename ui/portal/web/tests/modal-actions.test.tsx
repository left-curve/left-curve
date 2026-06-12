import { QueryClientProvider } from "@tanstack/react-query";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterAll, afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import { createRef } from "react";
import type React from "react";

import { resetAppletsKitMocks, setAppletsKitUseApp } from "./mocks/applets-kit";
import { createTestQueryClient } from "./utils/query-client";

const modalMocks = vi.hoisted(() => ({
  cancelPerpsOrder: vi.fn(),
  getAccountInfo: vi.fn(),
  getPrice: vi.fn(),
  hideModal: vi.fn(),
  navigate: vi.fn(),
  refetchBalances: vi.fn(),
  refreshUserStatus: vi.fn(),
  toastError: vi.fn(),
  updateKey: vi.fn(),
  useAccount: vi.fn(),
  useBalances: vi.fn(),
  useConfig: vi.fn(),
  usePrices: vi.fn(),
  usePublicClient: vi.fn(),
  useSigningClient: vi.fn(),
  submitMutation: undefined as
    | undefined
    | {
        invalidateKeys?: unknown[][];
        mutationFn: () => Promise<unknown>;
        onSuccess?: () => void;
      },
}));

let ActivateAccount: typeof import("../src/components/modals/ActivateAccount").ActivateAccount;
let ConfirmSend: typeof import("../src/components/modals/ConfirmSend").ConfirmSend;
let ConfirmSwap: typeof import("../src/components/modals/ConfirmSwap").ConfirmSwap;
let PerpsCloseAll: typeof import("../src/components/modals/PerpsCloseAll").PerpsCloseAll;
let PerpsCloseOrder: typeof import("../src/components/modals/PerpsCloseOrder").PerpsCloseOrder;
let RemoveKey: typeof import("../src/components/modals/RemoveKey").RemoveKey;

vi.mock("@left-curve/foundation", async (importOriginal) => ({
  ...(await importOriginal<typeof import("@left-curve/foundation")>()),
  useApp: () => ({
    settings: {
      formatNumberOptions: {
        currency: "USD",
        language: "en-US",
        mask: 1,
      },
    },
  }),
}));

vi.mock("@left-curve/store", () => ({
  useAccount: modalMocks.useAccount,
  useBalances: modalMocks.useBalances,
  useConfig: modalMocks.useConfig,
  usePrices: modalMocks.usePrices,
  usePublicClient: modalMocks.usePublicClient,
  useSigningClient: modalMocks.useSigningClient,
  useSubmitTx: ({
    mutation,
  }: {
    mutation: {
      invalidateKeys?: unknown[][];
      mutationFn: () => Promise<unknown>;
      onSuccess?: () => void;
    };
  }) => {
    modalMocks.submitMutation = mutation;

    return {
      isPending: false,
      mutateAsync: async () => {
        const result = await mutation.mutationFn();
        mutation.onSuccess?.();
        return result;
      },
    };
  },
}));

function getCapturedSubmitMutation() {
  if (!modalMocks.submitMutation) {
    throw new Error("Expected modal submit mutation to be captured");
  }
  return modalMocks.submitMutation;
}

const uatomCoin = {
  decimals: 6,
  denom: "uatom",
  logoURI: "/images/coins/atom.svg",
  symbol: "ATOM",
};

const usdcCoin = {
  decimals: 6,
  denom: "uusdc",
  logoURI: "/images/coins/usdc.svg",
  symbol: "USDC",
};

function renderWithQueryClient(component: React.ReactNode) {
  const queryClient = createTestQueryClient();
  const invalidateQueries = vi.spyOn(queryClient, "invalidateQueries");

  const renderResult = render(
    <QueryClientProvider client={queryClient}>{component}</QueryClientProvider>,
  );

  return {
    invalidateQueries,
    queryClient,
    ...renderResult,
  };
}

function getIconOnlyButton(container: HTMLElement) {
  const button = Array.from(container.querySelectorAll<HTMLButtonElement>("button")).find(
    (candidate) => candidate.textContent?.trim() === "",
  );
  if (!button) throw new Error("Expected an icon-only modal button to exist");
  return button;
}

function setDefaultStoreMocks({
  balances = {},
  chainId = "dango-dev-1",
}: {
  balances?: Record<string, string>;
  chainId?: string;
} = {}) {
  modalMocks.useAccount.mockReturnValue({
    account: { address: "0x616c696365000000000000000000000000000000" },
    refreshUserStatus: modalMocks.refreshUserStatus,
    username: "alice",
  });
  modalMocks.useSigningClient.mockReturnValue({
    data: {
      cancelPerpsOrder: modalMocks.cancelPerpsOrder,
      updateKey: modalMocks.updateKey,
    },
  });
  modalMocks.useConfig.mockReturnValue({
    chain: {
      id: chainId,
    },
    coins: {
      byDenom: {
        uatom: uatomCoin,
        uusdc: usdcCoin,
      },
      getCoinInfo: vi.fn((denom: string) => {
        const coin = { uatom: uatomCoin, uusdc: usdcCoin }[denom as "uatom" | "uusdc"];
        if (!coin) throw new Error(`missing coin fixture for ${denom}`);
        return coin;
      }),
    },
  });
  modalMocks.usePrices.mockReturnValue({
    getPrice: modalMocks.getPrice,
  });
  modalMocks.usePublicClient.mockReturnValue({
    getAccountInfo: modalMocks.getAccountInfo,
  });
  modalMocks.useBalances.mockReturnValue({
    data: balances,
    refetch: modalMocks.refetchBalances,
  });
}

describe("modal transaction actions", () => {
  beforeAll(async () => {
    const originalConsoleError = console.error;
    const consoleError = vi.spyOn(console, "error").mockImplementation((message, ...args) => {
      if (
        typeof message === "string" &&
        message.includes("forwardRef render functions accept exactly two parameters")
      ) {
        return;
      }
      originalConsoleError(message, ...args);
    });

    ({ ActivateAccount } = await import("../src/components/modals/ActivateAccount"));
    ({ ConfirmSend } = await import("../src/components/modals/ConfirmSend"));
    ({ ConfirmSwap } = await import("../src/components/modals/ConfirmSwap"));
    ({ PerpsCloseAll } = await import("../src/components/modals/PerpsCloseAll"));
    ({ PerpsCloseOrder } = await import("../src/components/modals/PerpsCloseOrder"));
    ({ RemoveKey } = await import("../src/components/modals/RemoveKey"));
    consoleError.mockRestore();
  }, 30000);

  afterAll(() => {
    vi.restoreAllMocks();
  });

  beforeEach(() => {
    resetAppletsKitMocks();
    setAppletsKitUseApp({
      hideModal: modalMocks.hideModal,
      navigate: modalMocks.navigate,
      settings: {
        formatNumberOptions: {
          currency: "USD",
          language: "en-US",
          mask: 1,
        },
      },
      toast: {
        error: modalMocks.toastError,
      },
    });
    setDefaultStoreMocks();
    modalMocks.cancelPerpsOrder.mockResolvedValue(undefined);
    modalMocks.getAccountInfo.mockResolvedValue({
      index: 3,
      username: "bob",
    });
    modalMocks.getPrice.mockImplementation((amount: string, denom: string) =>
      denom === "uatom" ? Number(amount) * 12 : Number(amount),
    );
    modalMocks.updateKey.mockResolvedValue(undefined);
    modalMocks.submitMutation = undefined;
    Object.defineProperty(window, "dango", {
      configurable: true,
      value: {
        urls: {
          faucetUrl: "https://faucet.example",
        },
      },
    });
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
    vi.unstubAllGlobals();
  });

  it("removes a user key with the selected key hash and closes the modal", async () => {
    renderWithQueryClient(<RemoveKey keyHash="keyhash123" />);

    fireEvent.click(screen.getByRole("button", { name: m["common.delete"]() }));

    await waitFor(() => {
      expect(modalMocks.updateKey).toHaveBeenCalledWith({
        action: "delete",
        keyHash: "keyhash123",
        sender: "0x616c696365000000000000000000000000000000",
      });
    });
    expect(modalMocks.hideModal).toHaveBeenCalledOnce();
  });

  it("cancels key removal without deleting the selected key", () => {
    renderWithQueryClient(<RemoveKey keyHash="keyhash123" />);

    fireEvent.click(screen.getByRole("button", { name: m["common.cancel"]() }));

    expect(modalMocks.updateKey).not.toHaveBeenCalled();
    expect(modalMocks.hideModal).toHaveBeenCalledOnce();
  });

  it("keeps key-management state open when backend key removal fails", async () => {
    renderWithQueryClient(<RemoveKey keyHash="keyhash123" />);
    modalMocks.updateKey.mockRejectedValueOnce(new Error("key removal rejected"));

    const mutation = getCapturedSubmitMutation();

    await expect(mutation.mutationFn()).rejects.toThrow("key removal rejected");

    expect(mutation.invalidateKeys).toEqual([["user_keys"]]);
    expect(modalMocks.updateKey).toHaveBeenCalledWith({
      action: "delete",
      keyHash: "keyhash123",
      sender: "0x616c696365000000000000000000000000000000",
    });
    expect(modalMocks.hideModal).not.toHaveBeenCalled();
  });

  it("confirms a send after resolving the recipient account details", async () => {
    const confirmSend = vi.fn();
    const rejectSend = vi.fn();
    const recipient = "0x626f620000000000000000000000000000000000";

    renderWithQueryClient(
      <ConfirmSend
        amount="1500000"
        confirmSend={confirmSend}
        denom="uatom"
        rejectSend={rejectSend}
        to={recipient}
      />,
    );

    expect(screen.getByText("1.5 ATOM")).toBeInTheDocument();
    expect(modalMocks.getPrice).toHaveBeenCalledWith("1.5", "uatom");
    expect(modalMocks.getAccountInfo).toHaveBeenCalledWith({ address: recipient });
    expect(await screen.findByText("bob #3")).toBeInTheDocument();
    const truncatedRecipient = screen.getByText("0x626f62").closest("p");
    expect(truncatedRecipient).toHaveTextContent("00000000");

    fireEvent.click(screen.getByRole("button", { name: m["modals.confirmSend.confirmButton"]() }));

    expect(confirmSend).toHaveBeenCalledOnce();
    expect(rejectSend).not.toHaveBeenCalled();
    expect(modalMocks.hideModal).toHaveBeenCalledOnce();
  });

  it("preserves backend account index zero in send recipient details", async () => {
    const confirmSend = vi.fn();
    const rejectSend = vi.fn();
    const recipient = "0x67656e6573697300000000000000000000000000";
    modalMocks.getAccountInfo.mockResolvedValueOnce({
      index: 0,
      username: "genesis",
    });

    renderWithQueryClient(
      <ConfirmSend
        amount="1500000"
        confirmSend={confirmSend}
        denom="uatom"
        rejectSend={rejectSend}
        to={recipient}
      />,
    );

    expect(await screen.findByText("genesis #0")).toBeInTheDocument();
    expect(modalMocks.getAccountInfo).toHaveBeenCalledWith({ address: recipient });

    fireEvent.click(screen.getByRole("button", { name: m["modals.confirmSend.confirmButton"]() }));

    expect(confirmSend).toHaveBeenCalledOnce();
    expect(rejectSend).not.toHaveBeenCalled();
    expect(modalMocks.hideModal).toHaveBeenCalledOnce();
  });

  it("preserves zero-valued backend send amounts in confirmation details", async () => {
    const confirmSend = vi.fn();
    const rejectSend = vi.fn();
    const recipient = "0x626f620000000000000000000000000000000000";

    renderWithQueryClient(
      <ConfirmSend
        amount="0"
        confirmSend={confirmSend}
        denom="uatom"
        rejectSend={rejectSend}
        to={recipient}
      />,
    );

    expect(screen.getByText("0 ATOM")).toBeInTheDocument();
    expect(modalMocks.getPrice).toHaveBeenCalledWith("0", "uatom");
    expect(await screen.findByText("bob #3")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: m["modals.confirmSend.confirmButton"]() }));

    expect(confirmSend).toHaveBeenCalledOnce();
    expect(rejectSend).not.toHaveBeenCalled();
    expect(modalMocks.hideModal).toHaveBeenCalledOnce();
  });

  it("refreshes recipient account details when the send target changes", async () => {
    const firstRecipient = "0x626f620000000000000000000000000000000000";
    const secondRecipient = "0x6361726f6c0000000000000000000000000000";
    modalMocks.getAccountInfo
      .mockResolvedValueOnce({
        index: 3,
        username: "bob",
      })
      .mockResolvedValueOnce({
        index: 4,
        username: "carol",
      });

    const { rerender, queryClient } = renderWithQueryClient(
      <ConfirmSend
        amount="1500000"
        confirmSend={vi.fn()}
        denom="uatom"
        rejectSend={vi.fn()}
        to={firstRecipient}
      />,
    );

    expect(await screen.findByText("bob #3")).toBeInTheDocument();
    expect(modalMocks.getAccountInfo).toHaveBeenCalledWith({ address: firstRecipient });

    rerender(
      <QueryClientProvider client={queryClient}>
        <ConfirmSend
          amount="1500000"
          confirmSend={vi.fn()}
          denom="uatom"
          rejectSend={vi.fn()}
          to={secondRecipient}
        />
      </QueryClientProvider>,
    );

    expect(await screen.findByText("carol #4")).toBeInTheDocument();
    expect(modalMocks.getAccountInfo).toHaveBeenLastCalledWith({ address: secondRecipient });
    expect(modalMocks.getAccountInfo).toHaveBeenCalledTimes(2);
  });

  it("rejects a pending send when the visible close control is pressed", () => {
    const confirmSend = vi.fn();
    const rejectSend = vi.fn();

    const { container } = renderWithQueryClient(
      <ConfirmSend
        amount="1500000"
        confirmSend={confirmSend}
        denom="uatom"
        rejectSend={rejectSend}
        to="0x626f620000000000000000000000000000000000"
      />,
    );

    fireEvent.click(getIconOnlyButton(container));

    expect(rejectSend).toHaveBeenCalledOnce();
    expect(confirmSend).not.toHaveBeenCalled();
    expect(modalMocks.hideModal).toHaveBeenCalledOnce();
  });

  it("keeps a send rejectable when the recipient account lookup fails", async () => {
    const confirmSend = vi.fn();
    const rejectSend = vi.fn();
    const recipient = "0x626f620000000000000000000000000000000000";
    modalMocks.getAccountInfo.mockResolvedValueOnce(undefined);

    const { container } = renderWithQueryClient(
      <ConfirmSend
        amount="1500000"
        confirmSend={confirmSend}
        denom="uatom"
        rejectSend={rejectSend}
        to={recipient}
      />,
    );

    expect(screen.getByText("1.5 ATOM")).toBeInTheDocument();
    const truncatedRecipient = screen.getByText("0x626f62").closest("p");
    expect(truncatedRecipient).toHaveTextContent("00000000");
    await waitFor(() => {
      expect(modalMocks.getAccountInfo).toHaveBeenCalledWith({ address: recipient });
    });

    fireEvent.click(getIconOnlyButton(container));

    expect(rejectSend).toHaveBeenCalledOnce();
    expect(confirmSend).not.toHaveBeenCalled();
    expect(modalMocks.hideModal).toHaveBeenCalledOnce();
  });

  it("keeps a send rejectable when the recipient account lookup rejects", async () => {
    const confirmSend = vi.fn();
    const rejectSend = vi.fn();
    const recipient = "0x626f620000000000000000000000000000000000";
    modalMocks.getAccountInfo.mockRejectedValueOnce(new Error("recipient lookup unavailable"));

    const { container } = renderWithQueryClient(
      <ConfirmSend
        amount="1500000"
        confirmSend={confirmSend}
        denom="uatom"
        rejectSend={rejectSend}
        to={recipient}
      />,
    );

    expect(screen.getByText("1.5 ATOM")).toBeInTheDocument();
    const truncatedRecipient = screen.getByText("0x626f62").closest("p");
    expect(truncatedRecipient).toHaveTextContent("00000000");
    await waitFor(() => {
      expect(modalMocks.getAccountInfo).toHaveBeenCalledWith({ address: recipient });
    });

    fireEvent.click(getIconOnlyButton(container));

    expect(rejectSend).toHaveBeenCalledOnce();
    expect(confirmSend).not.toHaveBeenCalled();
    expect(modalMocks.hideModal).toHaveBeenCalledOnce();
  });

  it("rejects a pending send when the modal is closed imperatively", async () => {
    const rejectSend = vi.fn();
    const modalRef = createRef<{ triggerOnClose: () => void }>();

    renderWithQueryClient(
      <ConfirmSend
        ref={modalRef}
        amount="1500000"
        confirmSend={vi.fn()}
        denom="uatom"
        rejectSend={rejectSend}
        to="0x626f620000000000000000000000000000000000"
      />,
    );

    modalRef.current?.triggerOnClose();

    expect(rejectSend).toHaveBeenCalledOnce();
  });

  it("confirms a swap with normalized token amounts and fee details", () => {
    const confirmSwap = vi.fn();
    const rejectSwap = vi.fn();
    const modalRef = createRef<{ triggerOnClose: () => void }>();

    renderWithQueryClient(
      <ConfirmSwap
        ref={modalRef}
        confirmSwap={confirmSwap}
        fee="0.03 USDC"
        input={{ amount: "1500000", coin: uatomCoin }}
        output={{ amount: "2300000", coin: usdcCoin }}
        rejectSwap={rejectSwap}
      />,
    );

    expect(screen.getByText("1.5 ATOM")).toBeInTheDocument();
    expect(screen.getByText("2.3 USDC")).toBeInTheDocument();
    expect(screen.getByText("0.03 USDC")).toBeInTheDocument();
    expect(modalMocks.getPrice).toHaveBeenCalledWith("1.5", "uatom");
    expect(modalMocks.getPrice).toHaveBeenCalledWith("2.3", "uusdc");

    modalRef.current?.triggerOnClose();
    expect(rejectSwap).toHaveBeenCalledOnce();

    fireEvent.click(screen.getByRole("button", { name: m["common.confirm"]() }));

    expect(confirmSwap).toHaveBeenCalledOnce();
    expect(modalMocks.hideModal).toHaveBeenCalledOnce();
  });

  it("preserves zero-valued backend swap amounts in confirmation details", () => {
    const confirmSwap = vi.fn();
    const rejectSwap = vi.fn();

    renderWithQueryClient(
      <ConfirmSwap
        confirmSwap={confirmSwap}
        fee="0 USDC"
        input={{ amount: "0", coin: uatomCoin }}
        output={{ amount: "0", coin: usdcCoin }}
        rejectSwap={rejectSwap}
      />,
    );

    expect(screen.getByText("0 ATOM")).toBeInTheDocument();
    expect(screen.getAllByText("0 USDC")).toHaveLength(2);
    expect(modalMocks.getPrice).toHaveBeenCalledWith("0", "uatom");
    expect(modalMocks.getPrice).toHaveBeenCalledWith("0", "uusdc");

    fireEvent.click(screen.getByRole("button", { name: m["common.confirm"]() }));

    expect(confirmSwap).toHaveBeenCalledOnce();
    expect(rejectSwap).not.toHaveBeenCalled();
    expect(modalMocks.hideModal).toHaveBeenCalledOnce();
  });

  it("rejects a pending swap when the visible close control is pressed", () => {
    const confirmSwap = vi.fn();
    const rejectSwap = vi.fn();

    const { container } = renderWithQueryClient(
      <ConfirmSwap
        confirmSwap={confirmSwap}
        fee="0.03 USDC"
        input={{ amount: "1500000", coin: uatomCoin }}
        output={{ amount: "2300000", coin: usdcCoin }}
        rejectSwap={rejectSwap}
      />,
    );

    fireEvent.click(getIconOnlyButton(container));

    expect(rejectSwap).toHaveBeenCalledOnce();
    expect(confirmSwap).not.toHaveBeenCalled();
    expect(modalMocks.hideModal).toHaveBeenCalledOnce();
  });

  it("cancels one perps order and invalidates the account trade history", async () => {
    const { invalidateQueries } = renderWithQueryClient(<PerpsCloseOrder orderId="order-42" />);

    fireEvent.click(screen.getByRole("button", { name: m["modals.proTradeCloseOrder.action"]() }));

    await waitFor(() => {
      expect(modalMocks.cancelPerpsOrder).toHaveBeenCalledWith({
        request: { one: "order-42" },
        sender: "0x616c696365000000000000000000000000000000",
      });
    });
    expect(invalidateQueries).toHaveBeenCalledWith({
      queryKey: ["perpsTradeHistory", "0x616c696365000000000000000000000000000000"],
    });
    expect(modalMocks.hideModal).toHaveBeenCalledOnce();
  });

  it("cancels all perps orders with the all-orders request shape", async () => {
    const { invalidateQueries } = renderWithQueryClient(<PerpsCloseAll />);

    fireEvent.click(
      screen.getByRole("button", { name: m["modals.protradeCloseAllOrders.action"]() }),
    );

    await waitFor(() => {
      expect(modalMocks.cancelPerpsOrder).toHaveBeenCalledWith({
        request: "all",
        sender: "0x616c696365000000000000000000000000000000",
      });
    });
    expect(invalidateQueries).toHaveBeenCalledWith({
      queryKey: ["perpsTradeHistory", "0x616c696365000000000000000000000000000000"],
    });
    expect(modalMocks.hideModal).toHaveBeenCalledOnce();
  });

  it("claims the faucet on testnet and refreshes account status plus balances", async () => {
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      text: vi.fn(),
    });
    vi.stubGlobal("fetch", fetchMock);
    renderWithQueryClient(<ActivateAccount />);

    fireEvent.click(screen.getByRole("button", { name: m["signup.faucet.cta"]() }));

    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalledWith(
        "https://faucet.example/0x616c696365000000000000000000000000000000?skip_check=true",
      );
    });
    expect(modalMocks.refreshUserStatus).toHaveBeenCalledOnce();
    expect(modalMocks.refetchBalances).toHaveBeenCalledOnce();
    expect(modalMocks.toastError).not.toHaveBeenCalled();
  });

  it("surfaces faucet errors through the app toast", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue({
        ok: false,
        text: vi.fn().mockResolvedValue("rate limited"),
      }),
    );
    renderWithQueryClient(<ActivateAccount />);

    fireEvent.click(screen.getByRole("button", { name: m["signup.faucet.cta"]() }));

    await waitFor(() => {
      expect(modalMocks.toastError).toHaveBeenCalledWith({
        description: "rate limited",
        title: m["common.error"](),
      });
    });
    expect(modalMocks.refreshUserStatus).not.toHaveBeenCalled();
    expect(modalMocks.refetchBalances).not.toHaveBeenCalled();
  });

  it("surfaces faucet transport failures through the app toast", async () => {
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error("faucet unavailable")));
    renderWithQueryClient(<ActivateAccount />);

    fireEvent.click(screen.getByRole("button", { name: m["signup.faucet.cta"]() }));

    await waitFor(() => {
      expect(modalMocks.toastError).toHaveBeenCalledWith({
        description: "faucet unavailable",
        title: m["common.error"](),
      });
    });
    expect(modalMocks.refreshUserStatus).not.toHaveBeenCalled();
    expect(modalMocks.refetchBalances).not.toHaveBeenCalled();
  });

  it("dismisses activate-account onboarding without calling faucet or navigation side effects", () => {
    const fetchMock = vi.fn();
    vi.stubGlobal("fetch", fetchMock);
    renderWithQueryClient(<ActivateAccount />);

    fireEvent.click(screen.getByRole("button", { name: m["signup.doThisLater"]() }));

    expect(modalMocks.hideModal).toHaveBeenCalledOnce();
    expect(fetchMock).not.toHaveBeenCalled();
    expect(modalMocks.navigate).not.toHaveBeenCalled();
    expect(modalMocks.refreshUserStatus).not.toHaveBeenCalled();
    expect(modalMocks.refetchBalances).not.toHaveBeenCalled();
    expect(modalMocks.toastError).not.toHaveBeenCalled();
  });

  it("routes mainnet activation to bridge without calling the faucet flow", () => {
    const fetchMock = vi.fn();
    vi.stubGlobal("fetch", fetchMock);
    setDefaultStoreMocks({ chainId: "dango-1" });
    renderWithQueryClient(<ActivateAccount />);

    fireEvent.click(screen.getByRole("button", { name: m["signup.deposit.cta"]() }));

    expect(modalMocks.hideModal).toHaveBeenCalledOnce();
    expect(modalMocks.navigate).toHaveBeenCalledWith("/bridge");
    expect(fetchMock).not.toHaveBeenCalled();
    expect(modalMocks.refreshUserStatus).not.toHaveBeenCalled();
    expect(modalMocks.refetchBalances).not.toHaveBeenCalled();
    expect(modalMocks.toastError).not.toHaveBeenCalled();
  });

  it("routes funded testnet activation to settings without calling the faucet flow", () => {
    const fetchMock = vi.fn();
    vi.stubGlobal("fetch", fetchMock);
    setDefaultStoreMocks({
      balances: {
        uatom: "1",
      },
    });
    renderWithQueryClient(<ActivateAccount />);

    fireEvent.click(screen.getByRole("button", { name: m["signup.faucet.changeUsername"]() }));

    expect(modalMocks.hideModal).toHaveBeenCalledOnce();
    expect(modalMocks.navigate).toHaveBeenCalledWith("/settings");
    expect(fetchMock).not.toHaveBeenCalled();
    expect(modalMocks.refreshUserStatus).not.toHaveBeenCalled();
    expect(modalMocks.refetchBalances).not.toHaveBeenCalled();
    expect(modalMocks.toastError).not.toHaveBeenCalled();
  });

  it("routes active users to settings and mainnet users to bridge", () => {
    setDefaultStoreMocks({
      balances: {
        uatom: "1",
      },
    });
    renderWithQueryClient(<ActivateAccount />);

    fireEvent.click(screen.getByRole("button", { name: m["signup.faucet.changeUsername"]() }));

    expect(modalMocks.hideModal).toHaveBeenCalledOnce();
    expect(modalMocks.navigate).toHaveBeenCalledWith("/settings");

    cleanup();
    vi.clearAllMocks();
    setDefaultStoreMocks({ chainId: "dango-1" });
    renderWithQueryClient(<ActivateAccount />);

    fireEvent.click(screen.getByRole("button", { name: m["signup.deposit.cta"]() }));

    expect(modalMocks.hideModal).toHaveBeenCalledOnce();
    expect(modalMocks.navigate).toHaveBeenCalledWith("/bridge");
  });
});
