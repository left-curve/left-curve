import { QueryClientProvider } from "@tanstack/react-query";
import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
  resetAppletsKitMocks,
  setAppletsKitUseAppFactory,
  setAppletsKitUseInfiniteScrollFactory,
} from "./mocks/applets-kit";

import { formatDate } from "@left-curve/foundation";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import { Modals } from "@left-curve/applets-kit";
import { OrderType } from "@left-curve/types";

import type React from "react";
import type { ActivityRecord } from "@left-curve/store";

import { Activities } from "../src/components/activities/Activities";
import { OrderActivity } from "../src/components/activities/OrderActivity";
import { createTestQueryClient } from "./utils/query-client";

const activityListMocks = vi.hoisted(() => ({
  deleteActivityRecord: vi.fn(),
  getAccountInfo: vi.fn(),
  getContractInfo: vi.fn(),
  lastInfiniteScroll: undefined as
    | {
        callback: () => void;
        hasMore: boolean;
      }
    | undefined,
  navigate: vi.fn(),
  showModal: vi.fn(),
  useActivities: vi.fn(),
}));

vi.mock("framer-motion", () => ({
  AnimatePresence: ({ children }: React.PropsWithChildren) => <>{children}</>,
  motion: {
    div: ({
      animate: _animate,
      children,
      exit: _exit,
      initial: _initial,
      layout: _layout,
      layoutId: _layoutId,
      layoutRoot: _layoutRoot,
      transition: _transition,
      ...props
    }: React.HTMLAttributes<HTMLDivElement> & {
      animate?: unknown;
      exit?: unknown;
      initial?: unknown;
      layout?: unknown;
      layoutId?: unknown;
      layoutRoot?: unknown;
      transition?: unknown;
    }) => <div {...props}>{children}</div>,
  },
}));

vi.mock("@tanstack/react-router", () => ({
  useNavigate: () => activityListMocks.navigate,
  useRouter: () => ({
    navigate: activityListMocks.navigate,
  }),
}));

vi.mock("@left-curve/foundation", async (importOriginal) => ({
  ...(await importOriginal<typeof import("@left-curve/foundation")>()),
  useApp: () => ({
    settings: {
      dateFormat: "yyyy/MM/dd",
      formatNumberOptions: {
        language: "en-US",
        mask: 1,
      },
    },
  }),
}));

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
  useActivities: activityListMocks.useActivities,
  useConfig: () => ({
    chain: {
      blockExplorer: {
        accountPage: `https://explorer.example/account/${"$"}{address}`,
        contractPage: `https://explorer.example/contract/${"$"}{address}`,
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
    getAccountInfo: activityListMocks.getAccountInfo,
    getContractInfo: activityListMocks.getContractInfo,
  }),
}));

const fromAddress = "0x66726f6d00000000000000000000000000000000";
const toAddress = "0x746f000000000000000000000000000000000000";
const accountAddress = "0x6163636f756e7400000000000000000000000000";
const genesisAccountAddress = "0x67656e657369732d6163636f756e740000000000";

function makeTransferActivity(
  overrides: Partial<ActivityRecord<"transfer">> & {
    id: string;
    createdAt: string;
  },
): ActivityRecord<"transfer"> {
  return {
    blockHeight: 123,
    data: {
      coins: {
        "bridge/usdc": "2500000",
      },
      fromAddress,
      toAddress,
      type: "sent",
    },
    txHash: "0x7478686173680000000000000000000000000000000000000000000000000000",
    type: "transfer",
    ...overrides,
  };
}

function makeAccountActivity(
  overrides: Partial<ActivityRecord<"account">> & {
    id: string;
    createdAt: string;
  },
): ActivityRecord<"account"> {
  return {
    blockHeight: 98,
    data: {
      accountIndex: 2,
      address: accountAddress,
    },
    type: "account",
    ...overrides,
  };
}

function makePerpOrderFilledActivity(
  overrides: Partial<ActivityRecord<"perpOrderFilled">> & {
    id: string;
    createdAt: string;
  },
): ActivityRecord<"perpOrderFilled"> {
  return {
    blockHeight: 456,
    data: {
      closing_size: "0",
      fee: "0.2",
      fill_price: "65000",
      fill_size: "0.5",
      is_maker: true,
      opening_size: "0.5",
      order_id: "order-1",
      pair_id: "perp/btcusd",
      realized_pnl: "12.5",
      user: fromAddress,
    },
    txHash: "0x706572706f726465720000000000000000000000000000000000000000000000",
    type: "perpOrderFilled",
    ...overrides,
  };
}

function makePerpLiquidatedActivity(
  overrides: Partial<ActivityRecord<"perpLiquidated">> & {
    id: string;
    createdAt: string;
  },
): ActivityRecord<"perpLiquidated"> {
  return {
    blockHeight: 457,
    data: {
      adl_price: "3250",
      adl_realized_pnl: "-7.5",
      adl_size: "-1.25",
      pair_id: "perp/ethusd",
      user: fromAddress,
    },
    txHash: "0x706572706c697175696461746564000000000000000000000000000000000000",
    type: "perpLiquidated",
    ...overrides,
  };
}

function makePerpDeleveragedActivity(
  overrides: Partial<ActivityRecord<"perpDeleveraged">> & {
    id: string;
    createdAt: string;
  },
): ActivityRecord<"perpDeleveraged"> {
  return {
    blockHeight: 458,
    data: {
      closing_size: "-9",
      fill_price: "2.45",
      pair_id: "perp/atomusd",
      realized_pnl: "0",
      user: fromAddress,
    },
    txHash: "0x7065727064656c65766572616765640000000000000000000000000000000000",
    type: "perpDeleveraged",
    ...overrides,
  };
}

function setActivities(userActivities: ActivityRecord[]) {
  activityListMocks.useActivities.mockReturnValue({
    deleteActivityRecord: activityListMocks.deleteActivityRecord,
    hasActivities: userActivities.length > 0,
    totalActivities: userActivities.length,
    userActivities,
  });
}

function getByTextContent(text: string, container: HTMLElement = document.body) {
  return within(container).getByText((_content, element) => {
    if (!element?.textContent?.includes(text)) return false;
    return Array.from(element.children).every((child) => !child.textContent?.includes(text));
  });
}

function activityCardFor(title: string) {
  const card = screen.getByText(title).closest(".cursor-pointer");

  expect(card).not.toBeNull();

  return card as HTMLElement;
}

function renderWithQueryClient(component: React.ReactNode) {
  const queryClient = createTestQueryClient();

  return render(<QueryClientProvider client={queryClient}>{component}</QueryClientProvider>);
}

function removeActivityButton(container: HTMLElement) {
  const button = container.querySelector(".remove-activity");

  expect(button).not.toBeNull();

  return button as SVGElement;
}

function expectSpinner(container: HTMLElement) {
  expect(container.querySelector(".animate-spinner-ease-spin")).not.toBeNull();
}

function expectNoSpinner(container: HTMLElement) {
  expect(container.querySelector(".animate-spinner-ease-spin")).toBeNull();
}

function findActivityText(text: string) {
  return screen.findByText(text, {}, { timeout: 10_000 });
}

describe("activities list", () => {
  beforeEach(() => {
    resetAppletsKitMocks();
    setAppletsKitUseAppFactory(() => ({
      settings: {
        dateFormat: "yyyy/MM/dd",
      },
      showModal: activityListMocks.showModal,
    }));
    setAppletsKitUseInfiniteScrollFactory((callback, hasMore) => {
      activityListMocks.lastInfiniteScroll = {
        callback: callback as () => void,
        hasMore: Boolean(hasMore),
      };
      return {
        loadMoreRef: vi.fn(),
      };
    });
    activityListMocks.getAccountInfo.mockImplementation(async ({ address }) => {
      const accounts = {
        [fromAddress]: { index: 1, username: "from" },
        [toAddress]: { index: 2, username: "to" },
        [accountAddress]: { index: 3, username: "account" },
        [genesisAccountAddress]: { index: 0, username: "genesis" },
      };

      return accounts[address as keyof typeof accounts] ?? null;
    });
    activityListMocks.getContractInfo.mockResolvedValue(null);
    activityListMocks.lastInfiniteScroll = undefined;
    setActivities([]);
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("renders the localized empty state when there are no activities", () => {
    renderWithQueryClient(<Activities />);

    expect(screen.getByText(m["activities.noActivities.title"]())).toBeInTheDocument();
    expect(screen.getByText(m["activities.noActivities.description"]())).toBeInTheDocument();
    expect(activityListMocks.lastInfiniteScroll?.hasMore).toBe(false);
  });

  it("orders recent activities, groups them by day, and loads more through infinite scroll", async () => {
    const now = new Date();
    const yesterday = new Date(now.getTime() - 24 * 60 * 60 * 1000);
    const older = new Date(now.getTime() - 48 * 60 * 60 * 1000);
    const newest = makeTransferActivity({
      blockHeight: 304,
      createdAt: now.toISOString(),
      id: "transfer-newest",
    });
    const middle = makeTransferActivity({
      blockHeight: 203,
      createdAt: yesterday.toISOString(),
      data: {
        coins: {
          uatom: "5000000",
        },
        fromAddress: toAddress,
        toAddress: fromAddress,
        type: "received",
      },
      id: "transfer-middle",
      txHash: "0x6d6964646c650000000000000000000000000000000000000000000000000000",
    });
    const oldest = makeAccountActivity({
      createdAt: older.toISOString(),
      id: "account-oldest",
    });

    setActivities([oldest, middle, newest]);

    const { container } = renderWithQueryClient(<Activities activitiesPerCall={2} />);

    expect(
      await findActivityText(m["activities.activity.transfer.title"]({ action: "sent" })),
    ).toBeInTheDocument();
    expect(
      screen.getByText(m["activities.activity.transfer.title"]({ action: "received" })),
    ).toBeInTheDocument();
    expect(screen.getByText("Today")).toBeInTheDocument();
    expect(screen.getByText(formatDate(middle.createdAt, "yyyy/MM/dd"))).toBeInTheDocument();
    expect(screen.getByText("1m")).toBeInTheDocument();
    expect(getByTextContent("2.5 USDC")).toBeInTheDocument();
    expect(getByTextContent("5 ATOM")).toBeInTheDocument();
    expect(screen.queryByText(m["activities.activity.account.title"]())).not.toBeInTheDocument();
    expectSpinner(container);
    expect(activityListMocks.lastInfiniteScroll?.hasMore).toBe(true);

    fireEvent.click(removeActivityButton(container));

    expect(activityListMocks.deleteActivityRecord).toHaveBeenCalledWith("transfer-newest");
    expect(activityListMocks.showModal).not.toHaveBeenCalled();

    act(() => {
      activityListMocks.lastInfiniteScroll?.callback();
    });

    expect(await findActivityText(m["activities.activity.account.title"]())).toBeInTheDocument();
    expect(
      await screen.findByRole("link", { name: /account #3/ }, { timeout: 5_000 }),
    ).toBeInTheDocument();
    expectNoSpinner(container);
  });

  it("renders perps activity records with side, size, price, and PnL semantics", async () => {
    const now = new Date();
    const orderFilled = makePerpOrderFilledActivity({
      createdAt: now.toISOString(),
      id: "perp-order-filled",
    });
    const liquidated = makePerpLiquidatedActivity({
      createdAt: new Date(now.getTime() - 1000).toISOString(),
      id: "perp-liquidated",
    });
    const deleveraged = makePerpDeleveragedActivity({
      createdAt: new Date(now.getTime() - 2000).toISOString(),
      id: "perp-deleveraged",
    });

    setActivities([deleveraged, liquidated, orderFilled]);

    renderWithQueryClient(<Activities activitiesPerCall={5} />);

    const orderFilledTitle = m["activities.activity.perpOrderFilled.title"]();
    expect(await findActivityText(orderFilledTitle)).toBeInTheDocument();
    const orderFilledCard = activityCardFor(orderFilledTitle);
    expect(within(orderFilledCard).getByText("BTC/USD")).toBeInTheDocument();
    expect(within(orderFilledCard).getByText("Long")).toHaveClass("text-status-success");
    expect(within(orderFilledCard).getByText("Maker")).toBeInTheDocument();
    expect(getByTextContent("0.5 BTC", orderFilledCard)).toBeInTheDocument();
    expect(getByTextContent("65,000", orderFilledCard)).toBeInTheDocument();
    expect(getByTextContent("+12.5", orderFilledCard)).toHaveClass("text-status-success");

    const liquidatedTitle = m["activities.activity.perpLiquidated.title"]();
    expect(await findActivityText(liquidatedTitle)).toHaveClass("text-status-fail");
    const liquidatedCard = activityCardFor(liquidatedTitle);
    expect(within(liquidatedCard).getByText("ETH/USD")).toBeInTheDocument();
    expect(getByTextContent("1.25 ETH", liquidatedCard)).toBeInTheDocument();
    expect(getByTextContent("3,250", liquidatedCard)).toBeInTheDocument();
    const liquidatedPnl = Array.from(liquidatedCard?.querySelectorAll("span") ?? []).find(
      (element) =>
        element.textContent?.includes("7.5") && element.classList.contains("text-status-fail"),
    );
    expect(liquidatedPnl).toBeInTheDocument();

    const deleveragedTitle = m["activities.activity.perpDeleveraged.title"]();
    expect(await findActivityText(deleveragedTitle)).toBeInTheDocument();
    const deleveragedCard = activityCardFor(deleveragedTitle);
    expect(within(deleveragedCard).getByText("ATOM/USD")).toBeInTheDocument();
    expect(getByTextContent("9 ATOM", deleveragedCard)).toBeInTheDocument();
    expect(getByTextContent("2.45", deleveragedCard)).toBeInTheDocument();
  });

  it("keeps address clicks scoped to navigation and opens transfer details from the row", async () => {
    const transfer = makeTransferActivity({
      blockHeight: 777,
      createdAt: new Date().toISOString(),
      id: "transfer-row",
    });
    setActivities([transfer]);

    renderWithQueryClient(<Activities />);

    fireEvent.click(await screen.findByRole("link", { name: /from #1/ }, { timeout: 5_000 }));

    expect(activityListMocks.navigate).toHaveBeenCalledWith({
      to: `https://explorer.example/account/${fromAddress}`,
    });
    expect(activityListMocks.showModal).not.toHaveBeenCalled();

    vi.clearAllMocks();

    fireEvent.click(screen.getByText(m["activities.activity.transfer.title"]({ action: "sent" })));

    expect(activityListMocks.showModal).toHaveBeenCalledWith(Modals.ActivityTransfer, {
      action: "sent",
      blockHeight: 777,
      coins: {
        "bridge/usdc": "2500000",
      },
      from: fromAddress,
      navigate: activityListMocks.navigate,
      time: transfer.createdAt,
      to: toAddress,
      txHash: transfer.txHash,
    });
    expect(activityListMocks.navigate).not.toHaveBeenCalled();
  });

  it("preserves every backend transfer coin when opening transfer details", async () => {
    const transfer = makeTransferActivity({
      blockHeight: 778,
      createdAt: new Date().toISOString(),
      data: {
        coins: {
          "bridge/usdc": "2500000",
          uatom: "3000000",
        },
        fromAddress,
        toAddress,
        type: "received",
      },
      id: "transfer-multi-coin",
      txHash: "0x6d756c7469636f696e0000000000000000000000000000000000000000000000",
    });
    setActivities([transfer]);

    renderWithQueryClient(<Activities />);

    const title = m["activities.activity.transfer.title"]({ action: "received" });
    expect(await findActivityText(title)).toBeInTheDocument();

    const card = activityCardFor(title);
    expect(getByTextContent("2.5 USDC", card)).toBeInTheDocument();
    expect(getByTextContent("3 ATOM", card)).toBeInTheDocument();

    fireEvent.click(within(card).getByText(title));

    expect(activityListMocks.showModal).toHaveBeenCalledWith(Modals.ActivityTransfer, {
      action: "received",
      blockHeight: 778,
      coins: {
        "bridge/usdc": "2500000",
        uatom: "3000000",
      },
      from: fromAddress,
      navigate: activityListMocks.navigate,
      time: transfer.createdAt,
      to: toAddress,
      txHash: transfer.txHash,
    });
    expect(activityListMocks.navigate).not.toHaveBeenCalled();
  });

  it("routes account activity address clicks without opening activity details", async () => {
    const account = makeAccountActivity({
      createdAt: new Date().toISOString(),
      id: "account-row",
    });
    setActivities([account]);

    renderWithQueryClient(<Activities />);

    fireEvent.click(await screen.findByRole("link", { name: /account #3/ }, { timeout: 5_000 }));

    expect(activityListMocks.navigate).toHaveBeenCalledWith({
      to: `https://explorer.example/account/${accountAddress}`,
    });
    expect(activityListMocks.showModal).not.toHaveBeenCalled();

    fireEvent.click(screen.getByText(m["activities.activity.account.title"]()));

    expect(activityListMocks.navigate).toHaveBeenCalledTimes(1);
    expect(activityListMocks.showModal).not.toHaveBeenCalled();
  });

  it("preserves backend account index zero in account activity links", async () => {
    const account = makeAccountActivity({
      createdAt: new Date().toISOString(),
      data: {
        accountIndex: 0,
        address: genesisAccountAddress,
      },
      id: "genesis-account-row",
    });
    setActivities([account]);

    renderWithQueryClient(<Activities />);

    const accountLink = await screen.findByRole("link", { name: /genesis #0/ }, { timeout: 5_000 });

    fireEvent.click(accountLink);

    expect(activityListMocks.navigate).toHaveBeenCalledWith({
      to: `https://explorer.example/account/${genesisAccountAddress}`,
    });
    expect(activityListMocks.showModal).not.toHaveBeenCalled();

    fireEvent.click(screen.getByText(m["activities.activity.account.title"]()));

    expect(activityListMocks.navigate).toHaveBeenCalledTimes(1);
    expect(activityListMocks.showModal).not.toHaveBeenCalled();
  });

  it("keeps order row actions separate from nested address and remove controls", () => {
    const onClick = vi.fn();

    render(
      <OrderActivity kind={OrderType.Limit} onClick={onClick}>
        <p>Limit order filled</p>
        <button className="address-visualizer" type="button">
          Trader account
        </button>
        <button className="remove-activity" type="button">
          Remove activity
        </button>
      </OrderActivity>,
    );

    fireEvent.click(screen.getByText("Limit order filled"));
    expect(onClick).toHaveBeenCalledOnce();

    fireEvent.click(screen.getByRole("button", { name: "Trader account" }));
    fireEvent.click(screen.getByRole("button", { name: "Remove activity" }));

    expect(onClick).toHaveBeenCalledOnce();
  });
});
