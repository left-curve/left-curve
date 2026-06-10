import { QueryClientProvider } from "@tanstack/react-query";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import type React from "react";

import {
  resetAppletsKitMocks,
  setAppletsKitUseApp,
  setAppletsKitUseMediaQuery,
} from "./mocks/applets-kit";
import { Modals } from "@left-curve/applets-kit";

import { KeyManagementSection } from "../src/components/settings/KeyManagementSection";
import { createTestQueryClient } from "./utils/query-client";

const keyManagementMocks = vi.hoisted(() => ({
  ConnectionStatus: {
    Connected: "connected",
    Connecting: "connecting",
    Disconnected: "disconnected",
    Reconnecting: "reconnecting",
  },
  getUserKeys: vi.fn(),
  showModal: vi.fn(),
  useAccount: vi.fn(),
  useSigningClient: vi.fn(),
}));

vi.mock("@left-curve/store", () => ({
  useAccount: keyManagementMocks.useAccount,
  useSigningClient: keyManagementMocks.useSigningClient,
}));

vi.mock("@left-curve/store/types", () => ({
  ConnectionStatus: keyManagementMocks.ConnectionStatus,
}));

function renderWithQueryClient(component: React.ReactNode) {
  const queryClient = createTestQueryClient();
  const rendered = render(
    <QueryClientProvider client={queryClient}>{component}</QueryClientProvider>,
  );
  return { queryClient, ...rendered };
}

function getKeyRow(keyRepresentation: string) {
  const keyLabel = screen.getByText(keyRepresentation);
  const row = keyLabel.closest(".rounded-2xl");
  if (!row) throw new Error(`Could not find key row for ${keyRepresentation}`);
  return row;
}

function getTrashIcon(row: Element) {
  const icons = row.querySelectorAll("svg.cursor-pointer, svg.cursor-default");
  const trashIcon = icons.item(icons.length - 1);
  if (!trashIcon) throw new Error("Could not find key removal icon");
  return trashIcon;
}

describe("KeyManagementSection", () => {
  beforeEach(() => {
    resetAppletsKitMocks();
    setAppletsKitUseApp({
      settings: {
        dateFormat: "yyyy/MM/dd",
        timeFormat: "HH:mm",
      },
      showModal: keyManagementMocks.showModal,
    });
    setAppletsKitUseMediaQuery({
      isMd: true,
    });
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: {
        writeText: vi.fn(),
      },
    });

    keyManagementMocks.useAccount.mockReturnValue({
      keyHash: "active-key",
      status: keyManagementMocks.ConnectionStatus.Connected,
      userIndex: 7,
    });
    keyManagementMocks.useSigningClient.mockReturnValue({
      data: {
        getUserKeys: keyManagementMocks.getUserKeys,
      },
    });
    keyManagementMocks.getUserKeys.mockResolvedValue([
      {
        createdAt: "2026-01-02T03:04:05Z",
        keyHash: "active-key",
        keyType: "SECP256R1",
        publicKey: "AQID",
      },
      {
        createdAt: "2026-01-03T03:04:05Z",
        keyHash: "wallet-key",
        keyType: "ETHEREUM",
        publicKey: "0xabc123",
      },
    ]);
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("queries and renders user keys with decoded key representations", async () => {
    const { container } = renderWithQueryClient(<KeyManagementSection />);

    expect(container.querySelector(".animate-spinner-ease-spin")).toBeInTheDocument();

    await waitFor(() => {
      expect(keyManagementMocks.getUserKeys).toHaveBeenCalledWith({ userIndex: 7 });
    });

    expect(await screen.findByText("0x010203")).toBeInTheDocument();
    expect(screen.getByText("0xabc123")).toBeInTheDocument();
    expect(screen.getByText("Passkey")).toBeInTheDocument();
    expect(screen.getByText("Ethereum Wallet")).toBeInTheDocument();
    expect(screen.getByText("2026/01/02 03:04")).toBeInTheDocument();

    fireEvent.pointerUp(getKeyRow("0x010203").querySelector("button")!);
    expect(navigator.clipboard.writeText).toHaveBeenCalledWith("0x010203");
  });

  it("opens add-key and remove-key modals while protecting the active key", async () => {
    renderWithQueryClient(<KeyManagementSection />);

    const addKeyButton = screen.getByRole("button", { name: m["settings.keyManagement.add"]() });

    fireEvent.click(addKeyButton);
    expect(keyManagementMocks.showModal).toHaveBeenCalledWith(Modals.AddKey);

    await screen.findByText("0xabc123");

    const activeKeyTrash = getTrashIcon(getKeyRow("0x010203"));
    const walletKeyTrash = getTrashIcon(getKeyRow("0xabc123"));

    fireEvent.click(activeKeyTrash);
    expect(keyManagementMocks.showModal).toHaveBeenCalledTimes(1);

    fireEvent.click(walletKeyTrash);
    expect(keyManagementMocks.showModal).toHaveBeenLastCalledWith(Modals.RemoveKey, {
      keyHash: "wallet-key",
    });
  });

  it("does not render or query keys when the account is disconnected", () => {
    keyManagementMocks.useAccount.mockReturnValue({
      keyHash: undefined,
      status: keyManagementMocks.ConnectionStatus.Disconnected,
      userIndex: undefined,
    });

    renderWithQueryClient(<KeyManagementSection />);

    expect(screen.queryByText(m["settings.keyManagement.description"]())).not.toBeInTheDocument();
    expect(keyManagementMocks.getUserKeys).not.toHaveBeenCalled();
  });

  it("does not query keys before a signing client is available", () => {
    keyManagementMocks.useSigningClient.mockReturnValue({
      data: undefined,
    });

    renderWithQueryClient(<KeyManagementSection />);

    expect(document.querySelector(".animate-spinner-ease-spin")).toBeInTheDocument();
    expect(keyManagementMocks.getUserKeys).not.toHaveBeenCalled();
  });

  it("does not query keys before the connected user index is available", () => {
    keyManagementMocks.useAccount.mockReturnValue({
      keyHash: "active-key",
      status: keyManagementMocks.ConnectionStatus.Connected,
      userIndex: undefined,
    });

    renderWithQueryClient(<KeyManagementSection />);

    expect(document.querySelector(".animate-spinner-ease-spin")).toBeInTheDocument();
    expect(keyManagementMocks.getUserKeys).not.toHaveBeenCalled();
  });
});
