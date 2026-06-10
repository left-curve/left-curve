import { QueryClientProvider } from "@tanstack/react-query";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { AddressVisualizer } from "@left-curve/applets-kit";

import type React from "react";
import type { Address } from "@left-curve/types";

import { createTestQueryClient } from "./utils/query-client";

const addressVisualizerMocks = vi.hoisted(() => ({
  accounts: [] as Array<{ address: Address; index: number }>,
  appConfig: {
    addresses: {} as Record<string, string>,
  },
  getAccountInfo: vi.fn(),
  getContractInfo: vi.fn(),
  navigate: vi.fn(),
  username: undefined as string | undefined,
}));

vi.mock("@left-curve/store", () => ({
  useAccount: () => ({
    accounts: addressVisualizerMocks.accounts,
    username: addressVisualizerMocks.username,
  }),
  useAppConfig: () => ({
    data: addressVisualizerMocks.appConfig,
  }),
  useConfig: () => ({
    chain: {
      blockExplorer: {
        accountPage: `https://explorer.example/account/${"$"}{address}`,
        contractPage: `https://explorer.example/contract/${"$"}{address}`,
      },
    },
  }),
  usePublicClient: () => ({
    getAccountInfo: addressVisualizerMocks.getAccountInfo,
    getContractInfo: addressVisualizerMocks.getContractInfo,
  }),
}));

function renderWithQueryClient(component: React.ReactNode) {
  const queryClient = createTestQueryClient();
  return render(<QueryClientProvider client={queryClient}>{component}</QueryClientProvider>);
}

describe("AddressVisualizer", () => {
  beforeEach(() => {
    addressVisualizerMocks.accounts = [];
    addressVisualizerMocks.appConfig = {
      addresses: {},
    };
    addressVisualizerMocks.getAccountInfo.mockResolvedValue(null);
    addressVisualizerMocks.getContractInfo.mockResolvedValue(null);
    addressVisualizerMocks.username = undefined;
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("labels configured Dango contracts without asking the backend", async () => {
    const perpsAddress = "0x7065727073000000000000000000000000000000" as Address;
    addressVisualizerMocks.appConfig = {
      addresses: {
        perps: perpsAddress,
        [perpsAddress]: "perps",
      },
    };

    renderWithQueryClient(
      <AddressVisualizer
        address={perpsAddress}
        onClick={addressVisualizerMocks.navigate}
        withIcon
      />,
    );

    const perpsLink = await screen.findByRole("link", { name: /Perps/ });
    expect(addressVisualizerMocks.getAccountInfo).not.toHaveBeenCalled();
    expect(addressVisualizerMocks.getContractInfo).not.toHaveBeenCalled();

    fireEvent.click(perpsLink);

    expect(addressVisualizerMocks.navigate).toHaveBeenCalledWith(
      `https://explorer.example/contract/${perpsAddress}`,
    );
  });

  it("prefers the connected user's account metadata before backend lookups", async () => {
    const accountAddress = "0x616c6963652d6163636f756e7400000000000000" as Address;
    addressVisualizerMocks.accounts = [{ address: accountAddress, index: 2 }];
    addressVisualizerMocks.username = "alice";

    renderWithQueryClient(
      <AddressVisualizer
        address={accountAddress}
        onClick={addressVisualizerMocks.navigate}
        withIcon
      />,
    );

    const accountLink = await screen.findByRole("link", { name: /alice #2/ });
    expect(addressVisualizerMocks.getAccountInfo).not.toHaveBeenCalled();
    expect(addressVisualizerMocks.getContractInfo).not.toHaveBeenCalled();

    fireEvent.click(accountLink);

    expect(addressVisualizerMocks.navigate).toHaveBeenCalledWith(
      `https://explorer.example/account/${accountAddress}`,
    );
  });

  it("preserves backend account index zero from account lookups", async () => {
    const accountAddress = "0x67656e657369732d6163636f756e740000000000" as Address;
    addressVisualizerMocks.getAccountInfo.mockResolvedValueOnce({
      index: 0,
      username: "genesis",
    });

    renderWithQueryClient(
      <AddressVisualizer address={accountAddress} onClick={addressVisualizerMocks.navigate} />,
    );

    const accountLink = await screen.findByRole("link", { name: /genesis #0/ });

    expect(addressVisualizerMocks.getAccountInfo).toHaveBeenCalledWith({
      address: accountAddress,
    });
    expect(addressVisualizerMocks.getContractInfo).not.toHaveBeenCalled();

    fireEvent.click(accountLink);

    expect(addressVisualizerMocks.navigate).toHaveBeenCalledWith(
      `https://explorer.example/account/${accountAddress}`,
    );
  });

  it("falls back from backend account lookup to backend contract labels", async () => {
    const contractAddress = "0x7661756c742d636f6e7472616374000000000000" as Address;
    addressVisualizerMocks.getContractInfo.mockResolvedValueOnce({ label: "Vault Manager" });

    renderWithQueryClient(
      <AddressVisualizer address={contractAddress} onClick={addressVisualizerMocks.navigate} />,
    );

    const contractLink = await screen.findByRole("link", { name: /Vault Manager/ });

    await waitFor(() => {
      expect(addressVisualizerMocks.getAccountInfo).toHaveBeenCalledWith({
        address: contractAddress,
      });
    });
    expect(addressVisualizerMocks.getContractInfo).toHaveBeenCalledWith({
      address: contractAddress,
    });

    fireEvent.click(contractLink);

    expect(addressVisualizerMocks.navigate).toHaveBeenCalledWith(
      `https://explorer.example/contract/${contractAddress}`,
    );
  });

  it("keeps rendering the raw address when backend metadata lookup rejects", async () => {
    const unknownAddress = "0x756e6b6e6f776e2d6163636f756e740000000000" as Address;
    addressVisualizerMocks.getAccountInfo.mockRejectedValueOnce(
      new Error("account lookup unavailable"),
    );

    renderWithQueryClient(
      <AddressVisualizer address={unknownAddress} onClick={addressVisualizerMocks.navigate} />,
    );

    await waitFor(() => {
      expect(addressVisualizerMocks.getAccountInfo).toHaveBeenCalledWith({
        address: unknownAddress,
      });
    });
    expect(
      screen.getByText(
        (_, node) => node?.tagName === "SPAN" && node.textContent === unknownAddress,
      ),
    ).toBeInTheDocument();
    expect(screen.queryByRole("link")).not.toBeInTheDocument();
    expect(addressVisualizerMocks.navigate).not.toHaveBeenCalled();
  });
});
