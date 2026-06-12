import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import {
  resetAppletsKitMocks,
  setAppletsKitUseAppFactory,
  setAppletsKitUseMediaQuery,
} from "./mocks/applets-kit";

import { Modals } from "@left-curve/applets-kit";

import { AccountCreation } from "../src/components/account/AccountCreation";

const accountCreationMocks = vi.hoisted(() => ({
  registerAccount: vi.fn(),
  showModal: vi.fn(),
  subscribe: vi.fn(),
  submitMutation: undefined as
    | undefined
    | {
        invalidateKeys?: unknown[][];
        mutationFn: () => Promise<unknown>;
      },
  toastError: vi.fn(),
  useAccount: vi.fn(),
  useBalances: vi.fn(),
  useConfig: vi.fn(),
  useNavigate: vi.fn(),
  useSigningClient: vi.fn(),
}));

vi.mock("@tanstack/react-router", () => ({
  useNavigate: () => accountCreationMocks.useNavigate,
  useRouter: () => ({
    history: {
      go: vi.fn(),
    },
  }),
}));

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
    showModal: accountCreationMocks.showModal,
  }),
}));

vi.mock("@left-curve/store", () => ({
  useAccount: accountCreationMocks.useAccount,
  useBalances: accountCreationMocks.useBalances,
  useConfig: accountCreationMocks.useConfig,
  useSigningClient: accountCreationMocks.useSigningClient,
  useSubmitTx: ({
    mutation,
    toast,
  }: {
    mutation: {
      invalidateKeys?: unknown[][];
      mutationFn: () => Promise<unknown>;
    };
    toast?: {
      error?: (error: unknown) => void;
    };
  }) => {
    accountCreationMocks.submitMutation = mutation;

    return {
      isPending: false,
      mutateAsync: async () => {
        try {
          return await mutation.mutationFn();
        } catch (error) {
          toast?.error?.(error);
        }
      },
    };
  },
}));

function getCapturedSubmitMutation() {
  if (!accountCreationMocks.submitMutation) {
    throw new Error("Expected account creation submit mutation to be captured");
  }
  return accountCreationMocks.submitMutation;
}

function getAmountInput() {
  const input = document.querySelector<HTMLInputElement>('input[name="amount"]');
  if (!input) throw new Error("Expected amount input to exist");
  return input;
}

function renderDeposit() {
  render(<AccountCreation.Deposit />);
}

describe("AccountCreation.Deposit", () => {
  beforeEach(() => {
    resetAppletsKitMocks();
    setAppletsKitUseAppFactory(() => ({
      showModal: accountCreationMocks.showModal,
      subscriptions: {
        subscribe: accountCreationMocks.subscribe,
      },
      toast: {
        error: accountCreationMocks.toastError,
      },
    }));
    setAppletsKitUseMediaQuery({
      isMd: true,
    });
    accountCreationMocks.useNavigate.mockReturnValue(undefined);
    accountCreationMocks.useAccount.mockReturnValue({
      account: { address: "0x616c696365000000000000000000000000000000" },
      isConnected: true,
      userIndex: 7,
      username: "alice",
    });
    accountCreationMocks.useBalances.mockReturnValue({
      data: {
        "bridge/usdc": "5000000",
      },
    });
    accountCreationMocks.useConfig.mockReturnValue({
      coins: {
        byDenom: {
          "bridge/usdc": {
            decimals: 6,
            denom: "bridge/usdc",
            logoURI: "/usdc.png",
            symbol: "USDC",
          },
        },
      },
    });
    accountCreationMocks.useSigningClient.mockReturnValue({
      data: {
        registerAccount: accountCreationMocks.registerAccount,
      },
    });
    accountCreationMocks.registerAccount.mockResolvedValue(undefined);
    accountCreationMocks.subscribe.mockReturnValue(() => {});
    accountCreationMocks.submitMutation = undefined;
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("registers a new account with parsed USDC funds", async () => {
    renderDeposit();

    fireEvent.change(getAmountInput(), {
      target: { value: "1.25" },
    });
    fireEvent.click(screen.getByRole("button", { name: m["common.continue"]() }));

    await waitFor(() => {
      expect(accountCreationMocks.registerAccount).toHaveBeenCalledWith({
        funds: {
          "bridge/usdc": "1250000",
        },
        sender: "0x616c696365000000000000000000000000000000",
      });
    });
  });

  it("registers without funds when the deposit amount is zero", async () => {
    renderDeposit();

    fireEvent.change(getAmountInput(), {
      target: { value: "0" },
    });
    fireEvent.click(screen.getByRole("button", { name: m["common.continue"]() }));

    await waitFor(() => {
      expect(accountCreationMocks.registerAccount).toHaveBeenCalledWith({
        sender: "0x616c696365000000000000000000000000000000",
      });
    });
  });

  it("does not include legacy quest invalidation metadata when registering", async () => {
    accountCreationMocks.useAccount.mockReturnValue({
      account: { address: "0x7a65726f00000000000000000000000000000000" },
      isConnected: true,
      userIndex: 0,
      username: "zero",
    });
    renderDeposit();

    expect(getCapturedSubmitMutation().invalidateKeys).toBeUndefined();

    fireEvent.change(getAmountInput(), {
      target: { value: "0.75" },
    });
    fireEvent.click(screen.getByRole("button", { name: m["common.continue"]() }));

    await waitFor(() => {
      expect(accountCreationMocks.registerAccount).toHaveBeenCalledWith({
        funds: {
          "bridge/usdc": "750000",
        },
        sender: "0x7a65726f00000000000000000000000000000000",
      });
    });
  });

  it("surfaces account registration failures through the app toast", async () => {
    accountCreationMocks.registerAccount.mockRejectedValue(new Error("backend rejected"));
    renderDeposit();

    fireEvent.change(getAmountInput(), {
      target: { value: "1.25" },
    });
    fireEvent.click(screen.getByRole("button", { name: m["common.continue"]() }));

    await waitFor(() => {
      expect(accountCreationMocks.toastError).toHaveBeenCalledWith({
        title: m["common.error"](),
        description: m["signup.errors.couldntCompleteRequest"](),
      });
    });
    expect(accountCreationMocks.registerAccount).toHaveBeenCalledWith({
      funds: {
        "bridge/usdc": "1250000",
      },
      sender: "0x616c696365000000000000000000000000000000",
    });
    expect(accountCreationMocks.showModal).not.toHaveBeenCalled();
  });

  it("does not register or clear the amount without a signing client", async () => {
    accountCreationMocks.useSigningClient.mockReturnValue({
      data: undefined,
    });
    renderDeposit();

    fireEvent.change(getAmountInput(), {
      target: { value: "1.25" },
    });
    fireEvent.click(screen.getByRole("button", { name: m["common.continue"]() }));

    await waitFor(() => {
      expect(accountCreationMocks.toastError).toHaveBeenCalledWith({
        title: m["common.error"](),
        description: m["signup.errors.couldntCompleteRequest"](),
      });
    });
    expect(accountCreationMocks.registerAccount).not.toHaveBeenCalled();
    expect(accountCreationMocks.showModal).not.toHaveBeenCalled();
    expect(getAmountInput()).toHaveValue("1.25");
  });

  it("blocks submission when the requested deposit exceeds the available balance", async () => {
    renderDeposit();

    fireEvent.change(getAmountInput(), {
      target: { value: "6" },
    });

    expect(screen.getByRole("button", { name: m["common.continue"]() })).toBeDisabled();
    expect(accountCreationMocks.registerAccount).not.toHaveBeenCalled();
  });

  it("does not subscribe to account confirmations before an account is connected", () => {
    accountCreationMocks.useAccount.mockReturnValue({
      account: undefined,
      isConnected: false,
      userIndex: undefined,
      username: undefined,
    });

    renderDeposit();

    expect(getAmountInput()).toBeDisabled();
    expect(accountCreationMocks.subscribe).not.toHaveBeenCalled();
    expect(accountCreationMocks.registerAccount).not.toHaveBeenCalled();
  });

  it("keeps the previous amount when users type an invalid decimal value", () => {
    renderDeposit();
    const input = getAmountInput();

    fireEvent.change(input, {
      target: { value: "1.2" },
    });
    fireEvent.change(input, {
      target: { value: "1.2.3" },
    });

    expect(input).toHaveValue("1.2");
  });

  it("opens the account confirmation modal from the account subscription payload", async () => {
    renderDeposit();

    fireEvent.change(getAmountInput(), {
      target: { value: "2" },
    });

    const latestSubscription =
      accountCreationMocks.subscribe.mock.calls[
        accountCreationMocks.subscribe.mock.calls.length - 1
      ];
    expect(latestSubscription).toEqual([
      "account",
      expect.objectContaining({
        params: { userIndex: 7 },
      }),
    ]);

    await latestSubscription[1].listener({
      accounts: [
        {
          accountIndex: 3,
          address: "0x6e65776163636f756e7400000000000000000000",
        },
      ],
    });

    expect(accountCreationMocks.showModal).toHaveBeenCalledWith(
      Modals.ConfirmAccount,
      expect.objectContaining({
        accountAddress: "0x6e65776163636f756e7400000000000000000000",
        accountName: "Account #3",
        amount: "2000000",
        denom: "bridge/usdc",
      }),
    );
  });

  it("subscribes to account confirmations for backend user index zero", async () => {
    accountCreationMocks.useAccount.mockReturnValue({
      account: { address: "0x7a65726f00000000000000000000000000000000" },
      isConnected: true,
      userIndex: 0,
      username: "zero",
    });

    renderDeposit();

    fireEvent.change(getAmountInput(), {
      target: { value: "0.75" },
    });

    const latestSubscription =
      accountCreationMocks.subscribe.mock.calls[
        accountCreationMocks.subscribe.mock.calls.length - 1
      ];
    expect(latestSubscription).toEqual([
      "account",
      expect.objectContaining({
        params: { userIndex: 0 },
      }),
    ]);

    await latestSubscription[1].listener({
      accounts: [
        {
          accountIndex: 0,
          address: "0x7a65726f6163636f756e740000000000000000",
        },
      ],
    });

    expect(accountCreationMocks.showModal).toHaveBeenCalledWith(
      Modals.ConfirmAccount,
      expect.objectContaining({
        accountAddress: "0x7a65726f6163636f756e740000000000000000",
        accountName: "Account #0",
        amount: "750000",
        denom: "bridge/usdc",
      }),
    );
  });

  it("refreshes the account subscription when the deposit amount changes", async () => {
    const firstUnsubscribe = vi.fn();
    const secondUnsubscribe = vi.fn();
    accountCreationMocks.subscribe
      .mockReturnValueOnce(firstUnsubscribe)
      .mockReturnValueOnce(secondUnsubscribe);

    const { unmount } = render(<AccountCreation.Deposit />);

    fireEvent.change(getAmountInput(), {
      target: { value: "2.5" },
    });

    expect(firstUnsubscribe).toHaveBeenCalledOnce();
    expect(secondUnsubscribe).not.toHaveBeenCalled();

    const latestSubscription =
      accountCreationMocks.subscribe.mock.calls[
        accountCreationMocks.subscribe.mock.calls.length - 1
      ];

    await latestSubscription[1].listener({
      accounts: [
        {
          accountIndex: 4,
          address: "0x7265737562736372696265640000000000000000",
        },
      ],
    });

    expect(accountCreationMocks.showModal).toHaveBeenCalledWith(
      Modals.ConfirmAccount,
      expect.objectContaining({
        accountAddress: "0x7265737562736372696265640000000000000000",
        accountName: "Account #4",
        amount: "2500000",
        denom: "bridge/usdc",
      }),
    );

    unmount();

    expect(secondUnsubscribe).toHaveBeenCalledOnce();
  });
});
