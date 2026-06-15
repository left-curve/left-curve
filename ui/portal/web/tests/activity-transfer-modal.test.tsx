import { QueryClientProvider } from "@tanstack/react-query";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { formatDate } from "@left-curve/foundation";
import { m } from "@left-curve/foundation/paraglide/messages.js";

import { resetAppletsKitMocks, setAppletsKitUseApp } from "./mocks/applets-kit";
import { ActivityTransferModal } from "../src/components/modals/activities/ActivityTransferModal";
import { createTestQueryClient } from "./utils/query-client";

const activityModalMocks = vi.hoisted(() => ({
  getAccountInfo: vi.fn(),
  getContractInfo: vi.fn(),
  getPrice: vi.fn(),
  hideModal: vi.fn(),
  setSidebarVisibility: vi.fn(),
}));

const fromAddress = "0x66726f6d00000000000000000000000000000000";
const toAddress = "0x746f000000000000000000000000000000000000";

vi.mock("@left-curve/foundation", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@left-curve/foundation")>();

  return {
    ...actual,
    useApp: () => ({
      settings: {
        formatNumberOptions: {
          language: "en-US",
          mask: 1,
        },
      },
    }),
  };
});

vi.mock("@left-curve/store", () => ({
  useAccount: () => ({
    accounts: [],
    username: undefined,
  }),
  useAppConfig: () => ({
    data: {
      addresses: {},
    },
  }),
  useConfig: () => ({
    chain: {
      blockExplorer: {
        accountPage: `/account/${"$"}{address}`,
        contractPage: `/contract/${"$"}{address}`,
      },
    },
    coins: {
      getCoinInfo: (denom: string) => {
        const coins = {
          "bridge/usdc": {
            decimals: 6,
            denom: "bridge/usdc",
            logoURI: "/usdc.png",
            symbol: "USDC",
          },
          uatom: {
            decimals: 6,
            denom: "uatom",
            logoURI: "/atom.png",
            symbol: "ATOM",
          },
        };
        const coin = coins[denom as keyof typeof coins];
        if (!coin) throw new Error(`missing coin fixture for ${denom}`);
        return coin;
      },
    },
  }),
  usePublicClient: () => ({
    getAccountInfo: activityModalMocks.getAccountInfo,
    getContractInfo: activityModalMocks.getContractInfo,
  }),
  usePrices: () => ({
    getPrice: activityModalMocks.getPrice,
  }),
}));

function renderTransferModal({
  blockHeight = 1234,
  txHash = "0xtxhash000000000000000000000000000000000000000000000000000000000001",
}: {
  blockHeight?: number;
  txHash?: string;
} = {}) {
  const navigate = vi.fn();
  const queryClient = createTestQueryClient();

  const result = render(
    <QueryClientProvider client={queryClient}>
      <ActivityTransferModal
        action="sent"
        blockHeight={blockHeight}
        coins={{
          "bridge/usdc": "2500000",
          uatom: "3000000",
        }}
        from={fromAddress}
        navigate={navigate as never}
        time="2026-06-08T12:34:56Z"
        to={toAddress}
        txHash={txHash}
      />
    </QueryClientProvider>,
  );

  return {
    container: result.container,
    from: fromAddress,
    navigate,
    to: toAddress,
    txHash,
  };
}

function clickDetailCell(label: string) {
  const labelNode = screen.getByText(label);
  const row = labelNode.closest("div");
  if (!row) throw new Error(`Expected row for ${label}`);
  const clickable = row.querySelector(".cursor-pointer");
  if (!clickable) throw new Error(`Expected clickable link for ${label}`);
  fireEvent.click(clickable);
}

describe("activity transfer modal", () => {
  beforeEach(() => {
    resetAppletsKitMocks();
    setAppletsKitUseApp({
      hideModal: activityModalMocks.hideModal,
      setSidebarVisibility: activityModalMocks.setSidebarVisibility,
      settings: {
        dateFormat: "yyyy-MM-dd",
        timeFormat: "HH:mm",
      },
    });
    activityModalMocks.getPrice.mockImplementation((amount: string, denom: string) =>
      denom === "bridge/usdc" ? amount : String(Number(amount) * 12),
    );
    activityModalMocks.getAccountInfo.mockImplementation(
      async ({ address }: { address: string }) => {
        if (address === fromAddress) return { index: 1, username: "from" };
        if (address === toAddress) return { index: 2, username: "to" };
        return null;
      },
    );
    activityModalMocks.getContractInfo.mockResolvedValue(null);
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("renders sent transfer amounts, prices, participants, timestamp, and tx hash", async () => {
    const { from, to, txHash } = renderTransferModal();

    expect(
      screen.getByRole("heading", { name: m["activities.activity.modal.sent"]().trim() }),
    ).toBeInTheDocument();
    expect(screen.getByText("2.5 USDC")).toBeInTheDocument();
    expect(
      screen.getByText((_, node) => node?.tagName === "SPAN" && node.textContent === "$2.50"),
    ).toBeInTheDocument();
    expect(screen.getByText("3 ATOM")).toBeInTheDocument();
    expect(
      screen.getByText((_, node) => node?.tagName === "SPAN" && node.textContent === "$36.00"),
    ).toBeInTheDocument();
    expect(await screen.findByRole("link", { name: "from #1" })).toHaveAttribute(
      "href",
      `/account/${from}`,
    );
    expect(await screen.findByRole("link", { name: "to #2" })).toHaveAttribute(
      "href",
      `/account/${to}`,
    );
    expect(
      screen.getByText(formatDate("2026-06-08T12:34:56Z", "yyyy-MM-dd HH:mm")),
    ).toBeInTheDocument();
    expect(screen.getByText(txHash.slice(0, 8))).toBeInTheDocument();
    expect(screen.getByText(txHash.slice(-8))).toBeInTheDocument();
    expect(activityModalMocks.getPrice).toHaveBeenCalledWith("2.5", "bridge/usdc");
    expect(activityModalMocks.getPrice).toHaveBeenCalledWith("3", "uatom");
  });

  it("keeps raw transfer details usable when one participant lookup rejects", async () => {
    activityModalMocks.getAccountInfo.mockImplementation(
      async ({ address }: { address: string }) => {
        if (address === fromAddress) throw new Error("account lookup unavailable");
        if (address === toAddress) return { index: 2, username: "to" };
        return null;
      },
    );
    const { from, navigate, to, txHash } = renderTransferModal();

    expect(screen.getByText("2.5 USDC")).toBeInTheDocument();
    expect(screen.getByText("3 ATOM")).toBeInTheDocument();
    await waitFor(() => {
      expect(activityModalMocks.getAccountInfo).toHaveBeenCalledWith({ address: from });
    });
    expect(
      screen.getByText((_, node) => node?.tagName === "SPAN" && node.textContent === from),
    ).toBeInTheDocument();
    expect(await screen.findByRole("link", { name: "to #2" })).toHaveAttribute(
      "href",
      `/account/${to}`,
    );

    clickDetailCell(m["activities.activity.link"]({ link: "txHash" }));

    expect(activityModalMocks.hideModal).toHaveBeenCalledOnce();
    expect(activityModalMocks.setSidebarVisibility).toHaveBeenCalledWith(false);
    expect(navigate).toHaveBeenCalledWith({ to: `/tx/${txHash}` });
  });

  it("navigates from account visualizers and transaction details while closing the modal", async () => {
    const { from, navigate, txHash } = renderTransferModal();

    fireEvent.click(await screen.findByRole("link", { name: "from #1" }));

    expect(activityModalMocks.hideModal).toHaveBeenCalledOnce();
    expect(activityModalMocks.setSidebarVisibility).toHaveBeenCalledWith(false);
    expect(navigate).toHaveBeenCalledWith({ to: `/account/${from}` });

    vi.clearAllMocks();

    clickDetailCell(m["activities.activity.link"]({ link: "txHash" }));

    expect(activityModalMocks.hideModal).toHaveBeenCalledOnce();
    expect(activityModalMocks.setSidebarVisibility).toHaveBeenCalledWith(false);
    expect(navigate).toHaveBeenCalledWith({ to: `/tx/${txHash}` });
  });

  it("falls back to block-height detail navigation when there is no tx hash", () => {
    const { navigate } = renderTransferModal({
      blockHeight: 777,
      txHash: "",
    });

    expect(
      screen.getByText(m["activities.activity.link"]({ link: "blockHeight" })),
    ).toBeInTheDocument();
    expect(screen.getByText("777")).toBeInTheDocument();

    clickDetailCell(m["activities.activity.link"]({ link: "blockHeight" }));

    expect(activityModalMocks.hideModal).toHaveBeenCalledOnce();
    expect(activityModalMocks.setSidebarVisibility).toHaveBeenCalledWith(false);
    expect(navigate).toHaveBeenCalledWith({ to: "/block/777" });
  });

  it("preserves backend block height zero when falling back from a missing tx hash", () => {
    const { navigate } = renderTransferModal({
      blockHeight: 0,
      txHash: "",
    });

    expect(
      screen.getByText(m["activities.activity.link"]({ link: "blockHeight" })),
    ).toBeInTheDocument();
    expect(screen.getByText("0")).toBeInTheDocument();

    clickDetailCell(m["activities.activity.link"]({ link: "blockHeight" }));

    expect(activityModalMocks.hideModal).toHaveBeenCalledOnce();
    expect(activityModalMocks.setSidebarVisibility).toHaveBeenCalledWith(false);
    expect(navigate).toHaveBeenCalledWith({ to: "/block/0" });
  });

  it("hides the modal from the close button", () => {
    const { container } = renderTransferModal();

    const closeButton = container.querySelector("button.absolute");
    expect(closeButton).not.toBeNull();
    fireEvent.click(closeButton!);

    expect(activityModalMocks.hideModal).toHaveBeenCalledOnce();
    expect(activityModalMocks.setSidebarVisibility).not.toHaveBeenCalled();
  });
});
