import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import { getTransferMocks } from "./mocks/transfer";
import { Transfer } from "../src/components/transfer/Transfer";

const mocks = getTransferMocks();

function renderTransfer(action: "send" | "spot-perp") {
  return render(
    <Transfer action={action} changeAction={mocks.changeAction}>
      <Transfer.Send />
      <Transfer.SpotPerp />
    </Transfer>,
  );
}

function getInputByName(name: string) {
  const input = document.querySelector<HTMLInputElement>(`input[name="${name}"]`);
  if (!input) throw new Error(`Expected input[name="${name}"] to exist`);
  return input;
}

function getSubmitButton() {
  const button = document.querySelector<HTMLButtonElement>('form button[type="submit"]');
  if (!button) throw new Error("Expected form submit button to exist");
  return button;
}

const transferLabels = {
  from: m["transfer.spotPerp.from"](),
  spotAccount: m["accountMenu.spotAccount"](),
  perpAccount: m["accountMenu.perpAccount"](),
  to: m["transfer.spotPerp.to"](),
  transfer: m["sendAndReceive.title"](),
};

describe("Transfer applet behavior", () => {
  beforeEach(() => {
    mocks.balances = {
      "bridge/usdc": "100000000",
    };
    mocks.coinsByDenom = {
      "bridge/usdc": {
        decimals: 6,
        denom: "bridge/usdc",
        logoURI: "/usdc.png",
        symbol: "USDC",
      },
    };
    mocks.hasSigningClient = true;
    mocks.isConnected = true;
    mocks.isValidAddress.mockReturnValue(true);
    mocks.showModal.mockImplementation((_modal, props: { confirmSend: () => void }) => {
      props.confirmSend();
    });
    mocks.useQuery.mockReturnValue({
      data: true,
      isLoading: false,
    });
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("submits a confirmed Dango account transfer with parsed token units", async () => {
    renderTransfer("send");

    fireEvent.change(getInputByName("amount"), {
      target: { value: "1.25" },
    });
    fireEvent.change(getInputByName("address"), {
      target: { value: "0x524543495049454E540000000000000000000000" },
    });
    fireEvent.click(getSubmitButton());

    await waitFor(() => {
      expect(mocks.transfer).toHaveBeenCalledWith({
        sender: "0x73656e6465720000000000000000000000000000",
        transfer: {
          "0x524543495049454e540000000000000000000000": {
            "bridge/usdc": "1250000",
          },
        },
      });
    });

    expect(mocks.showModal).toHaveBeenCalledWith(
      "confirm-send",
      expect.objectContaining({
        amount: "1250000",
        denom: "bridge/usdc",
        to: "0x524543495049454e540000000000000000000000",
      }),
    );
    expect(mocks.refreshBalances).toHaveBeenCalledOnce();
    expect(mocks.queryInvalidate).toHaveBeenCalledWith({ queryKey: ["quests", "alice"] });
    expect(getInputByName("amount")).toHaveValue("");
    expect(getInputByName("address")).toHaveValue("");
  });

  it("uses the selected backend coin denom and decimals for confirmed account transfers", async () => {
    mocks.balances = {
      "bridge/eth": "10000000000000000000",
      "bridge/usdc": "100000000",
    };
    mocks.coinsByDenom = {
      "bridge/eth": {
        decimals: 18,
        denom: "bridge/eth",
        logoURI: "/eth.png",
        symbol: "ETH",
      },
      "bridge/usdc": {
        decimals: 6,
        denom: "bridge/usdc",
        logoURI: "/usdc.png",
        symbol: "USDC",
      },
    };

    renderTransfer("send");

    fireEvent.change(screen.getByTestId("coin-selector"), {
      target: { value: "bridge/eth" },
    });
    fireEvent.change(getInputByName("amount"), {
      target: { value: "1.25" },
    });
    fireEvent.change(getInputByName("address"), {
      target: { value: "0x524543495049454E540000000000000000000000" },
    });
    fireEvent.click(getSubmitButton());

    await waitFor(() => {
      expect(mocks.transfer).toHaveBeenCalledWith({
        sender: "0x73656e6465720000000000000000000000000000",
        transfer: {
          "0x524543495049454e540000000000000000000000": {
            "bridge/eth": "1250000000000000000",
          },
        },
      });
    });

    expect(mocks.showModal).toHaveBeenCalledWith(
      "confirm-send",
      expect.objectContaining({
        amount: "1250000000000000000",
        denom: "bridge/eth",
        to: "0x524543495049454e540000000000000000000000",
      }),
    );
    expect(mocks.refreshBalances).toHaveBeenCalledOnce();
    expect(mocks.queryInvalidate).toHaveBeenCalledWith({ queryKey: ["quests", "alice"] });
  });

  it("enables recipient account lookup only after an address is entered", async () => {
    renderTransfer("send");

    expect(mocks.useQuery).toHaveBeenCalledWith(
      expect.objectContaining({
        enabled: false,
        queryKey: ["transfer", undefined],
      }),
    );

    fireEvent.change(getInputByName("address"), {
      target: { value: "0x524543495049454E540000000000000000000000" },
    });

    await waitFor(() => {
      expect(mocks.useQuery).toHaveBeenCalledWith(
        expect.objectContaining({
          enabled: true,
          queryKey: ["transfer", "0x524543495049454e540000000000000000000000"],
        }),
      );
    });
  });

  it("resolves recipient account existence through the backend lookup query", async () => {
    const recipientAddress = "0x524543495049454e540000000000000000000000";
    mocks.getAccountInfo.mockResolvedValueOnce({
      address: recipientAddress,
    });

    renderTransfer("send");

    fireEvent.change(getInputByName("address"), {
      target: { value: recipientAddress },
    });

    await waitFor(() => {
      expect(mocks.useQuery).toHaveBeenCalledWith(
        expect.objectContaining({
          enabled: true,
          queryKey: ["transfer", recipientAddress],
        }),
      );
    });

    const accountLookupQuery = mocks.useQuery.mock.calls.at(-1)?.[0] as {
      queryFn: (context: { signal: AbortSignal }) => Promise<boolean>;
    };

    await expect(
      accountLookupQuery.queryFn({
        signal: new AbortController().signal,
      }),
    ).resolves.toBe(true);
    expect(mocks.getAccountInfo).toHaveBeenCalledWith({
      address: recipientAddress,
    });

    mocks.getAccountInfo.mockResolvedValueOnce(null);

    await expect(
      accountLookupQuery.queryFn({
        signal: new AbortController().signal,
      }),
    ).resolves.toBe(false);
  });

  it("skips recipient backend lookup when the account query is aborted", async () => {
    const recipientAddress = "0x524543495049454e540000000000000000000000";

    renderTransfer("send");

    fireEvent.change(getInputByName("address"), {
      target: { value: recipientAddress },
    });

    await waitFor(() => {
      expect(mocks.useQuery).toHaveBeenCalledWith(
        expect.objectContaining({
          enabled: true,
          queryKey: ["transfer", recipientAddress],
        }),
      );
    });

    const accountLookupQuery = mocks.useQuery.mock.calls.at(-1)?.[0] as {
      queryFn: (context: { signal: AbortSignal }) => Promise<boolean>;
    };
    const controller = new AbortController();
    controller.abort();

    await expect(
      accountLookupQuery.queryFn({
        signal: controller.signal,
      }),
    ).resolves.toBe(false);
    expect(mocks.getAccountInfo).not.toHaveBeenCalled();
  });

  it("does not transfer or reset the form when send confirmation is rejected", async () => {
    mocks.showModal.mockImplementation((_modal, props: { rejectSend: () => void }) => {
      props.rejectSend();
    });

    renderTransfer("send");

    fireEvent.change(getInputByName("amount"), {
      target: { value: "1.25" },
    });
    fireEvent.change(getInputByName("address"), {
      target: { value: "0x524543495049454E540000000000000000000000" },
    });
    fireEvent.click(getSubmitButton());

    await waitFor(() => {
      expect(mocks.showModal).toHaveBeenCalledWith(
        "confirm-send",
        expect.objectContaining({
          amount: "1250000",
          denom: "bridge/usdc",
          to: "0x524543495049454e540000000000000000000000",
        }),
      );
    });

    expect(mocks.transfer).not.toHaveBeenCalled();
    expect(mocks.refreshBalances).not.toHaveBeenCalled();
    expect(mocks.queryInvalidate).not.toHaveBeenCalled();
    expect(getInputByName("amount")).toHaveValue("1.25");
    expect(getInputByName("address")).toHaveValue("0x524543495049454e540000000000000000000000");
  });

  it("keeps the send form intact when the confirmed backend transfer fails", async () => {
    mocks.transfer.mockRejectedValueOnce(new Error("chain rejected transfer"));

    renderTransfer("send");

    fireEvent.change(getInputByName("amount"), {
      target: { value: "1.25" },
    });
    fireEvent.change(getInputByName("address"), {
      target: { value: "0x524543495049454E540000000000000000000000" },
    });
    fireEvent.click(getSubmitButton());

    await waitFor(() => {
      expect(mocks.transfer).toHaveBeenCalledWith({
        sender: "0x73656e6465720000000000000000000000000000",
        transfer: {
          "0x524543495049454e540000000000000000000000": {
            "bridge/usdc": "1250000",
          },
        },
      });
    });
    expect(mocks.refreshBalances).not.toHaveBeenCalled();
    expect(mocks.queryInvalidate).not.toHaveBeenCalled();
    expect(getInputByName("amount")).toHaveValue("1.25");
    expect(getInputByName("address")).toHaveValue("0x524543495049454e540000000000000000000000");
  });

  it("shows the Dango-account warning and blocks submission when the recipient is not found", () => {
    mocks.useQuery.mockReturnValue({
      data: false,
      isLoading: false,
    });

    renderTransfer("send");

    fireEvent.change(getInputByName("amount"), {
      target: { value: "1" },
    });
    fireEvent.change(getInputByName("address"), {
      target: { value: "0x6d697373696e6700000000000000000000000000" },
    });

    const missingAccountWarningPrefix = m["transfer.warning.sendNonDango"]({
      app: "{app}",
    }).split("{app}")[0];
    expect(screen.getByText(missingAccountWarningPrefix, { exact: false })).toBeInTheDocument();
    expect(getSubmitButton()).toBeDisabled();
    expect(mocks.transfer).not.toHaveBeenCalled();
  });

  it("blocks send while recipient account lookup is still loading", () => {
    mocks.useQuery.mockReturnValue({
      data: false,
      isLoading: true,
    });

    renderTransfer("send");

    fireEvent.change(getInputByName("amount"), {
      target: { value: "1" },
    });
    fireEvent.change(getInputByName("address"), {
      target: { value: "0x524543495049454e540000000000000000000000" },
    });

    const missingAccountWarningPrefix = m["transfer.warning.sendNonDango"]({
      app: "{app}",
    }).split("{app}")[0];
    expect(
      screen.queryByText(missingAccountWarningPrefix, { exact: false }),
    ).not.toBeInTheDocument();
    expect(getSubmitButton()).toBeDisabled();

    fireEvent.click(getSubmitButton());

    expect(mocks.showModal).not.toHaveBeenCalled();
    expect(mocks.transfer).not.toHaveBeenCalled();
    expect(mocks.refreshBalances).not.toHaveBeenCalled();
  });

  it("blocks malformed recipient addresses before opening confirmation or backend lookup", async () => {
    mocks.isValidAddress.mockReturnValue(false);

    renderTransfer("send");

    fireEvent.change(getInputByName("amount"), {
      target: { value: "1" },
    });
    fireEvent.change(getInputByName("address"), {
      target: { value: "0xnotvalid" },
    });

    await waitFor(() => {
      expect(mocks.useQuery).toHaveBeenCalledWith(
        expect.objectContaining({
          enabled: true,
          queryKey: ["transfer", "0xnotvalid"],
        }),
      );
    });

    const accountLookupQuery = mocks.useQuery.mock.calls.at(-1)?.[0] as {
      queryFn: (context: { signal: AbortSignal }) => Promise<boolean>;
    };

    await expect(
      accountLookupQuery.queryFn({
        signal: new AbortController().signal,
      }),
    ).resolves.toBe(false);

    fireEvent.click(getSubmitButton());

    expect(mocks.getAccountInfo).not.toHaveBeenCalled();
    expect(getSubmitButton()).toBeDisabled();
    expect(mocks.showModal).not.toHaveBeenCalled();
    expect(mocks.transfer).not.toHaveBeenCalled();
    expect(mocks.refreshBalances).not.toHaveBeenCalled();
  });

  it("does not open confirmation or reset the send form without a signing client", async () => {
    mocks.hasSigningClient = false;

    renderTransfer("send");

    fireEvent.change(getInputByName("amount"), {
      target: { value: "1.25" },
    });
    fireEvent.change(getInputByName("address"), {
      target: { value: "0x524543495049454E540000000000000000000000" },
    });
    fireEvent.click(getSubmitButton());

    await waitFor(() => {
      expect(mocks.showModal).not.toHaveBeenCalled();
    });
    expect(mocks.transfer).not.toHaveBeenCalled();
    expect(mocks.refreshBalances).not.toHaveBeenCalled();
    expect(mocks.queryInvalidate).not.toHaveBeenCalled();
    expect(getInputByName("amount")).toHaveValue("1.25");
    expect(getInputByName("address")).toHaveValue("0x524543495049454e540000000000000000000000");
  });

  it("keeps send controls disabled and does not open confirmation when disconnected", () => {
    mocks.isConnected = false;

    renderTransfer("send");

    expect(getInputByName("amount")).toBeDisabled();
    expect(getInputByName("address")).toBeDisabled();
    expect(getSubmitButton()).toBeDisabled();

    fireEvent.click(getSubmitButton());

    expect(mocks.showModal).not.toHaveBeenCalled();
    expect(mocks.transfer).not.toHaveBeenCalled();
    expect(mocks.refreshBalances).not.toHaveBeenCalled();
  });

  it("deposits spot USDC into the perps account and invalidates perps account resources", async () => {
    renderTransfer("spot-perp");

    fireEvent.change(getInputByName("amount"), {
      target: { value: "2.5" },
    });
    fireEvent.click(screen.getByRole("button", { name: transferLabels.transfer }));

    await waitFor(() => {
      expect(mocks.depositMargin).toHaveBeenCalledWith({
        amount: "2500000",
        sender: "0x73656e6465720000000000000000000000000000",
      });
    });

    expect(mocks.refreshBalances).toHaveBeenCalledOnce();
    expect(mocks.invalidatePerpsAccountResources).toHaveBeenCalledWith({
      accountAddress: "0x73656e6465720000000000000000000000000000",
      chainId: "dango-dev-1",
      perpsContract: "0x7065727073000000000000000000000000000000",
    });
  });

  it("withdraws perps margin into the spot account and invalidates perps account resources", async () => {
    renderTransfer("spot-perp");

    fireEvent.change(getInputByName("amount"), {
      target: { value: "1.75" },
    });
    fireEvent.click(screen.getByTestId("flip-direction"));
    fireEvent.click(screen.getByRole("button", { name: transferLabels.transfer }));

    await waitFor(() => {
      expect(mocks.withdrawMargin).toHaveBeenCalledWith({
        amount: "1.75",
        sender: "0x73656e6465720000000000000000000000000000",
      });
    });

    expect(mocks.depositMargin).not.toHaveBeenCalled();
    expect(mocks.refreshBalances).toHaveBeenCalledOnce();
    expect(mocks.invalidatePerpsAccountResources).toHaveBeenCalledWith({
      accountAddress: "0x73656e6465720000000000000000000000000000",
      chainId: "dango-dev-1",
      perpsContract: "0x7065727073000000000000000000000000000000",
    });
    expect(getInputByName("amount")).toHaveValue("");
  });

  it("does not submit spot-perp transfers or reset the amount without a signing client", async () => {
    mocks.hasSigningClient = false;

    renderTransfer("spot-perp");

    fireEvent.change(getInputByName("amount"), {
      target: { value: "2.5" },
    });
    fireEvent.click(screen.getByRole("button", { name: transferLabels.transfer }));

    await waitFor(() => {
      expect(mocks.depositMargin).not.toHaveBeenCalled();
    });
    expect(mocks.withdrawMargin).not.toHaveBeenCalled();
    expect(mocks.refreshBalances).not.toHaveBeenCalled();
    expect(mocks.invalidatePerpsAccountResources).not.toHaveBeenCalled();
    expect(getInputByName("amount")).toHaveValue("2.5");
  });

  it("keeps spot-perp amount intact when backend margin deposit fails", async () => {
    mocks.depositMargin.mockRejectedValueOnce(new Error("chain rejected deposit"));

    renderTransfer("spot-perp");

    fireEvent.change(getInputByName("amount"), {
      target: { value: "2.5" },
    });
    fireEvent.click(screen.getByRole("button", { name: transferLabels.transfer }));

    await waitFor(() => {
      expect(mocks.depositMargin).toHaveBeenCalledWith({
        amount: "2500000",
        sender: "0x73656e6465720000000000000000000000000000",
      });
    });
    expect(mocks.refreshBalances).not.toHaveBeenCalled();
    expect(mocks.invalidatePerpsAccountResources).not.toHaveBeenCalled();
    expect(getInputByName("amount")).toHaveValue("2.5");
  });

  it("flips to perp withdrawal, clamps amount to available margin, and submits truncated units", async () => {
    renderTransfer("spot-perp");

    fireEvent.change(getInputByName("amount"), {
      target: { value: "10" },
    });
    fireEvent.click(screen.getByTestId("flip-direction"));

    expect(getInputByName("from")).toHaveValue(transferLabels.perpAccount);
    expect(getInputByName("to")).toHaveValue(transferLabels.spotAccount);
    expect(getInputByName("amount")).toHaveValue("3.5");

    fireEvent.click(screen.getByRole("button", { name: transferLabels.transfer }));

    await waitFor(() => {
      expect(mocks.withdrawMargin).toHaveBeenCalledWith({
        amount: "3.5",
        sender: "0x73656e6465720000000000000000000000000000",
      });
    });
    expect(mocks.depositMargin).not.toHaveBeenCalled();
  });

  it("flips back to spot deposit and clamps amount to the spot balance", () => {
    renderTransfer("spot-perp");

    fireEvent.click(screen.getByTestId("flip-direction"));
    fireEvent.change(getInputByName("amount"), {
      target: { value: "200" },
    });
    fireEvent.click(screen.getByTestId("flip-direction"));

    expect(getInputByName("from")).toHaveValue(transferLabels.spotAccount);
    expect(getInputByName("to")).toHaveValue(transferLabels.perpAccount);
    expect(getInputByName("amount")).toHaveValue("100");
    expect(mocks.depositMargin).not.toHaveBeenCalled();
    expect(mocks.withdrawMargin).not.toHaveBeenCalled();
  });

  it("keeps spot-perp controls disabled and does not submit when disconnected", () => {
    mocks.isConnected = false;

    renderTransfer("spot-perp");

    expect(getInputByName("amount")).toBeDisabled();
    expect(getSubmitButton()).toBeDisabled();

    fireEvent.click(getSubmitButton());

    expect(mocks.depositMargin).not.toHaveBeenCalled();
    expect(mocks.withdrawMargin).not.toHaveBeenCalled();
    expect(mocks.refreshBalances).not.toHaveBeenCalled();
    expect(mocks.invalidatePerpsAccountResources).not.toHaveBeenCalled();
  });

  it("requests the send action when the connected account disappears", async () => {
    mocks.isConnected = false;

    renderTransfer("spot-perp");

    await waitFor(() => {
      expect(mocks.changeAction).toHaveBeenCalledWith("send");
    });
  });
});
