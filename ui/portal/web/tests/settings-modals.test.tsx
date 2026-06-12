import { QueryClientProvider } from "@tanstack/react-query";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import type React from "react";

import { resetAppletsKitMocks, setAppletsKitUseApp } from "./mocks/applets-kit";

import { AddressWarning } from "../src/components/modals/AddressWarning";
import { AdjustSlippage } from "../src/components/modals/AdjustSlippage";
import { EditUsername } from "../src/components/modals/EditUsername";
import { createTestQueryClient } from "./utils/query-client";

const settingsModalMocks = vi.hoisted(() => ({
  getUser: vi.fn(),
  hideModal: vi.fn(),
  maxSlippage: "0.005",
  navigate: vi.fn(),
  refreshAccounts: vi.fn(),
  setMaxSlippage: vi.fn(),
  submitMutation: undefined as
    | undefined
    | {
        mutationFn: () => Promise<unknown>;
        onSuccess?: () => void;
      },
  updateUsername: vi.fn(),
}));

vi.mock("@left-curve/utils", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@left-curve/utils")>();

  return {
    ...actual,
    wait: vi.fn(() => Promise.resolve()),
  };
});

vi.mock("@left-curve/store", () => ({
  useAccount: () => ({
    account: { address: "0x616c696365000000000000000000000000000000" },
    refreshAccounts: settingsModalMocks.refreshAccounts,
    username: "alice",
  }),
  usePublicClient: () => ({
    getUser: settingsModalMocks.getUser,
  }),
  useSigningClient: () => ({
    data: {
      updateUsername: settingsModalMocks.updateUsername,
    },
  }),
  useStorage: () => [settingsModalMocks.maxSlippage, settingsModalMocks.setMaxSlippage],
  useSubmitTx: ({
    mutation,
  }: {
    mutation: {
      mutationFn: () => Promise<unknown>;
      onSuccess?: () => void;
    };
  }) => {
    settingsModalMocks.submitMutation = mutation;

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
  if (!settingsModalMocks.submitMutation) {
    throw new Error("Expected settings modal submit mutation to be captured");
  }
  return settingsModalMocks.submitMutation;
}

function renderWithQueryClient(component: React.ReactNode) {
  return render(
    <QueryClientProvider client={createTestQueryClient()}>{component}</QueryClientProvider>,
  );
}

function inputByName(name: string) {
  const input = document.querySelector<HTMLInputElement>(`input[name="${name}"]`);
  if (!input) throw new Error(`Expected input named ${name}`);
  return input;
}

describe("settings modals", () => {
  beforeEach(() => {
    resetAppletsKitMocks();
    setAppletsKitUseApp({
      hideModal: settingsModalMocks.hideModal,
      navigate: settingsModalMocks.navigate,
    });
    settingsModalMocks.maxSlippage = "0.005";
    settingsModalMocks.getUser.mockRejectedValue(new Error("not found"));
    settingsModalMocks.submitMutation = undefined;
    settingsModalMocks.updateUsername.mockResolvedValue(undefined);
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("persists max slippage as a fraction and closes the modal", () => {
    renderWithQueryClient(<AdjustSlippage />);

    expect(inputByName("slippage")).toHaveValue("0.5");

    fireEvent.change(inputByName("slippage"), {
      target: { value: "1.25" },
    });
    fireEvent.click(screen.getByRole("button", { name: m["common.confirm"]() }));

    expect(settingsModalMocks.setMaxSlippage).toHaveBeenCalledWith("0.0125");
    expect(settingsModalMocks.hideModal).toHaveBeenCalledOnce();
  });

  it("closes adjusted slippage without persisting the edited value", () => {
    const { container } = renderWithQueryClient(<AdjustSlippage />);

    fireEvent.change(inputByName("slippage"), {
      target: { value: "1.25" },
    });

    const closeButton = container.querySelector("button.absolute");
    if (!closeButton) throw new Error("Expected slippage close button");

    fireEvent.click(closeButton);

    expect(settingsModalMocks.setMaxSlippage).not.toHaveBeenCalled();
    expect(settingsModalMocks.hideModal).toHaveBeenCalledOnce();
  });

  it("blocks invalid slippage values before storage is updated", () => {
    renderWithQueryClient(<AdjustSlippage />);

    fireEvent.change(inputByName("slippage"), {
      target: { value: "6" },
    });

    expect(
      screen.getByText(m["dex.protrade.perps.slippageOutOfRange"]({ max: "5" })),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: m["common.confirm"]() })).toBeDisabled();

    fireEvent.change(inputByName("slippage"), {
      target: { value: "1.234" },
    });

    expect(
      screen.getByText(m["dex.protrade.perps.slippageMaxDecimals"]({ max: "2" })),
    ).toBeInTheDocument();
    expect(settingsModalMocks.setMaxSlippage).not.toHaveBeenCalled();
  });

  it("routes address warnings to bridge deposit and closes the modal", () => {
    renderWithQueryClient(<AddressWarning />);

    fireEvent.click(
      screen.getByRole("button", {
        name: m["accountCard.addressWarning.descriptionLink"](),
      }),
    );

    expect(settingsModalMocks.navigate).toHaveBeenCalledWith("/bridge");
    expect(settingsModalMocks.hideModal).toHaveBeenCalledOnce();
  });

  it("dismisses address warnings without navigating away", () => {
    renderWithQueryClient(<AddressWarning />);

    fireEvent.click(
      screen.getByRole("button", {
        name: m["accountCard.addressWarning.button"](),
      }),
    );

    expect(settingsModalMocks.navigate).not.toHaveBeenCalled();
    expect(settingsModalMocks.hideModal).toHaveBeenCalledOnce();
  });

  it("updates an available username with the connected account address", async () => {
    renderWithQueryClient(<EditUsername />);

    fireEvent.change(inputByName("editedUsername"), {
      target: { value: "alice_new" },
    });

    expect(inputByName("editedUsername")).toHaveValue("alice_new");
    await waitFor(() => {
      expect(settingsModalMocks.getUser).toHaveBeenCalledWith({
        userIndexOrName: { name: "alice_new" },
      });
    });

    fireEvent.click(screen.getByRole("button", { name: m["settings.session.username.save"]() }));

    await waitFor(() => {
      expect(settingsModalMocks.updateUsername).toHaveBeenCalledWith({
        sender: "0x616c696365000000000000000000000000000000",
        username: "alice_new",
      });
    });
    expect(settingsModalMocks.refreshAccounts).toHaveBeenCalledOnce();
    expect(settingsModalMocks.hideModal).toHaveBeenCalledOnce();
  });

  it("keeps username save disabled and skips lookup when the username is unchanged", () => {
    renderWithQueryClient(<EditUsername />);

    expect(inputByName("editedUsername")).toHaveValue("alice");
    expect(
      screen.getByRole("button", { name: m["settings.session.username.save"]() }),
    ).toBeDisabled();
    expect(settingsModalMocks.getUser).not.toHaveBeenCalled();
    expect(settingsModalMocks.updateUsername).not.toHaveBeenCalled();
  });

  it("closes username editing without submitting the changed username", () => {
    const { container } = renderWithQueryClient(<EditUsername />);

    fireEvent.change(inputByName("editedUsername"), {
      target: { value: "alice_new" },
    });

    const closeButton = container.querySelector("button.absolute");
    if (!closeButton) throw new Error("Expected username close button");

    fireEvent.click(closeButton);

    expect(settingsModalMocks.updateUsername).not.toHaveBeenCalled();
    expect(settingsModalMocks.refreshAccounts).not.toHaveBeenCalled();
    expect(settingsModalMocks.hideModal).toHaveBeenCalledOnce();
  });

  it("keeps username editing open when backend username update fails", async () => {
    renderWithQueryClient(<EditUsername />);
    settingsModalMocks.updateUsername.mockRejectedValueOnce(new Error("username update rejected"));

    fireEvent.change(inputByName("editedUsername"), {
      target: { value: "alice_new" },
    });

    await waitFor(() => {
      expect(settingsModalMocks.getUser).toHaveBeenCalledWith({
        userIndexOrName: { name: "alice_new" },
      });
    });

    const mutation = getCapturedSubmitMutation();

    await expect(mutation.mutationFn()).rejects.toThrow("username update rejected");

    expect(settingsModalMocks.updateUsername).toHaveBeenCalledWith({
      sender: "0x616c696365000000000000000000000000000000",
      username: "alice_new",
    });
    expect(settingsModalMocks.refreshAccounts).not.toHaveBeenCalled();
    expect(settingsModalMocks.hideModal).not.toHaveBeenCalled();
  });

  it("blocks invalid username edits before checking backend availability", async () => {
    renderWithQueryClient(<EditUsername />);

    fireEvent.change(inputByName("editedUsername"), {
      target: { value: "alice-invalid" },
    });

    expect(await screen.findByText(m["errors.validations.usernameRule"]())).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: m["settings.session.username.save"]() }),
    ).toBeDisabled();
    expect(settingsModalMocks.getUser).not.toHaveBeenCalled();
    expect(settingsModalMocks.updateUsername).not.toHaveBeenCalled();
    expect(settingsModalMocks.refreshAccounts).not.toHaveBeenCalled();
    expect(settingsModalMocks.hideModal).not.toHaveBeenCalled();
  });

  it("surfaces taken usernames and keeps the save action disabled", async () => {
    settingsModalMocks.getUser.mockResolvedValue({
      accounts: {
        0: "0x74616b656e000000000000000000000000000000",
      },
    });

    renderWithQueryClient(<EditUsername />);

    fireEvent.change(inputByName("editedUsername"), {
      target: { value: "taken" },
    });

    await waitFor(() => {
      expect(screen.getByText(m["signup.errors.usernameTaken"]())).toBeInTheDocument();
    });
    expect(
      screen.getByRole("button", { name: m["settings.session.username.save"]() }),
    ).toBeDisabled();
    expect(settingsModalMocks.updateUsername).not.toHaveBeenCalled();
  });
});
