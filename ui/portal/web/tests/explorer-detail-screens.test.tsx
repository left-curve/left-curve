import { QueryClientProvider } from "@tanstack/react-query";
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import { Suspense, type ReactNode } from "react";

import { resetAppletsKitMocks, setAppletsKitUseCountdownFactory } from "./mocks/applets-kit";
import { BlockExplorer } from "../src/components/explorer/BlockExplorer";
import { TransactionExplorer } from "../src/components/explorer/TransactionExplorer";
import { createTestQueryClient } from "./utils/query-client";

const explorerDetailMocks = vi.hoisted(() => ({
  getAccountInfo: vi.fn(),
  getContractInfo: vi.fn(),
  navigate: vi.fn(),
  parseExplorerErrorMessage: vi.fn(),
  useCountdown: vi.fn(),
  useExplorerBlock: vi.fn(),
  useExplorerTransaction: vi.fn(),
}));

vi.mock("@left-curve/store", () => ({
  parseExplorerErrorMessage: explorerDetailMocks.parseExplorerErrorMessage,
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
        accountPage: `https://explorer.example/account/${"$"}{address}`,
        contractPage: `https://explorer.example/contract/${"$"}{address}`,
      },
    },
  }),
  useExplorerBlock: explorerDetailMocks.useExplorerBlock,
  useExplorerTransaction: explorerDetailMocks.useExplorerTransaction,
  usePublicClient: () => ({
    getAccountInfo: explorerDetailMocks.getAccountInfo,
    getContractInfo: explorerDetailMocks.getContractInfo,
  }),
}));

vi.mock("@tanstack/react-router", () => ({
  useNavigate: () => explorerDetailMocks.navigate,
}));

vi.mock("../src/components/explorer/TransactionsTable", () => ({
  TransactionsTable: ({ transactions }: { transactions: unknown[] }) => (
    <div>transactions-table:{transactions.length}</div>
  ),
}));

vi.mock("@microlink/react-json-view", () => ({
  default: ({ src }: { src: unknown }) => (
    <pre data-testid="json-visualizer">{JSON.stringify(src)}</pre>
  ),
}));

const senderAddress = "0x73656e6465720000000000000000000000000000";
const txHash = "0x7478686173680000000000000000000000000000000000000000000000000000";
const failedTxHash = "0x6661696c65640000000000000000000000000000000000000000000000000000";
const blockHash = "0x626c6f636b000000000000000000000000000000000000000000000000000000";

const indexedTransaction = {
  blockHeight: 1234,
  createdAt: "2026-06-08T12:00:00.000Z",
  errorMessage: "insufficient funds\n 1: execute transfer 2: debit account",
  gasUsed: 24000,
  gasWanted: 30000,
  hasSucceeded: false,
  hash: txHash,
  messages: [
    {
      data: {
        transfer: {
          amount: "1500000",
          denom: "uatom",
          to: "0x726563697069656e740000000000000000000000",
        },
      },
      methodName: "transfer",
      orderIdx: 0,
    },
  ],
  nestedEvents: '[{"type":"transfer","amount":"1500000"}]',
  sender: senderAddress,
  transactionIdx: 2,
};

function renderWithQueryClient(component: ReactNode) {
  const queryClient = createTestQueryClient();

  return render(
    <QueryClientProvider client={queryClient}>
      <Suspense fallback={null}>{component}</Suspense>
    </QueryClientProvider>,
  );
}

function renderTransactionExplorer(children: ReactNode, data = indexedTransaction) {
  explorerDetailMocks.useExplorerTransaction.mockReturnValue({
    data,
    isLoading: false,
  });

  renderWithQueryClient(
    <TransactionExplorer txHash={data?.hash ?? failedTxHash}>{children}</TransactionExplorer>,
  );
}

function renderBlockExplorer(children: ReactNode, data = blockExplorerData) {
  explorerDetailMocks.useExplorerBlock.mockReturnValue({
    data,
    isLoading: false,
  });

  renderWithQueryClient(<BlockExplorer height={String(data.height)}>{children}</BlockExplorer>);
}

function copyButtonBeside(text: string) {
  const textContainer = screen.getByText(text).closest("p");

  expect(textContainer).not.toBeNull();

  return within(textContainer as HTMLElement).getByRole("button");
}

function detailRow(label: string) {
  const row = screen.getByText(label).closest("div");

  expect(row).not.toBeNull();

  return within(row as HTMLElement);
}

const blockExplorerData = {
  currentBlock: {
    blockHeight: 1234,
    hash: blockHash,
  },
  height: 1234,
  isFutureBlock: false,
  isInvalidBlock: false,
  searchBlock: {
    blockHeight: 1234,
    createdAt: "2026-06-08T12:00:00.000Z",
    cronsOutcomes: ['{"kind":"cron","ok":true}', "plain cron outcome"],
    hash: blockHash,
    transactions: [indexedTransaction],
  },
};

describe("explorer detail screens", () => {
  beforeEach(() => {
    resetAppletsKitMocks();
    setAppletsKitUseCountdownFactory(explorerDetailMocks.useCountdown);
    explorerDetailMocks.getAccountInfo.mockResolvedValue({
      index: 1,
      username: "alice",
    });
    explorerDetailMocks.getContractInfo.mockResolvedValue(null);
    explorerDetailMocks.parseExplorerErrorMessage.mockReturnValue({
      backtrace: " 1: execute transfer 2: debit account",
      error: {
        message: "insufficient funds",
      },
    });
    explorerDetailMocks.useCountdown.mockReturnValue({
      days: "0",
      hours: "0",
      minutes: "0",
      seconds: "4",
    });
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: {
        writeText: vi.fn(),
      },
    });
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("renders transaction details with backend fields, status, copy target, and navigation links", async () => {
    renderTransactionExplorer(<TransactionExplorer.Details />);

    expect(screen.getByText(m["explorer.txs.txDetails"]())).toBeInTheDocument();
    expect(screen.getByText(m["explorer.txs.txHash"]())).toBeInTheDocument();
    expect(screen.getByText(txHash)).toBeInTheDocument();

    fireEvent.pointerUp(copyButtonBeside(txHash));
    expect(navigator.clipboard.writeText).toHaveBeenCalledWith(txHash);

    const senderLink = await screen.findByRole("link", { name: /alice #1/ });
    expect(
      screen.getByText(new Date(indexedTransaction.createdAt).toLocaleString()),
    ).toBeInTheDocument();
    expect(screen.getByText("1234")).toBeInTheDocument();
    expect(screen.getByText("2")).toBeInTheDocument();
    expect(screen.getByText("24000")).toBeInTheDocument();
    expect(screen.getByText("30000")).toBeInTheDocument();
    expect(screen.getByText(m["explorer.txs.failed"]())).toHaveClass("bg-surface-secondary-red");

    fireEvent.click(senderLink);
    expect(explorerDetailMocks.navigate).toHaveBeenCalledWith({
      to: `https://explorer.example/account/${senderAddress}`,
    });

    fireEvent.click(screen.getByRole("link", { name: /1234/ }));
    expect(explorerDetailMocks.navigate).toHaveBeenCalledWith({ to: "/block/1234" });
  });

  it("preserves backend block height zero in transaction detail links", () => {
    renderTransactionExplorer(<TransactionExplorer.Details />, {
      ...indexedTransaction,
      blockHeight: 0,
      transactionIdx: 0,
    });

    fireEvent.click(screen.getByRole("link", { name: /^0$/ }));

    expect(explorerDetailMocks.navigate).toHaveBeenCalledWith({ to: "/block/0" });
  });

  it("renders zero-valued backend transaction detail fields", () => {
    renderTransactionExplorer(<TransactionExplorer.Details />, {
      ...indexedTransaction,
      blockHeight: 0,
      gasUsed: 0,
      gasWanted: 0,
      hasSucceeded: true,
      transactionIdx: 0,
    });

    expect(detailRow(m["explorer.txs.block"]()).getByRole("link", { name: /^0$/ })).toHaveAttribute(
      "href",
      "/block/0",
    );
    expect(detailRow(m["explorer.txs.index"]()).getByText("0")).toBeInTheDocument();
    expect(detailRow(m["explorer.txs.gasUsed"]()).getByText("0")).toBeInTheDocument();
    expect(detailRow(m["explorer.txs.gasWanted"]()).getByText("0")).toBeInTheDocument();
    expect(screen.getByText(m["explorer.txs.success"]())).toBeInTheDocument();
  });

  it("renders transaction messages, parsed error details, backtrace, and nested events", async () => {
    renderTransactionExplorer(<TransactionExplorer.Messages />);

    expect(explorerDetailMocks.parseExplorerErrorMessage).toHaveBeenCalledWith(
      indexedTransaction.errorMessage,
    );
    expect(await screen.findByText(m["explorer.txs.error"]())).toBeInTheDocument();
    expect(screen.getByText(m["explorer.txs.message"]())).toBeInTheDocument();
    expect(screen.getByText(m["explorer.txs.backtrace"]())).toBeInTheDocument();
    expect(screen.getByText(m["explorer.txs.messages"]())).toBeInTheDocument();
    expect(screen.getByText("transfer")).toBeInTheDocument();
    expect(screen.getByText(m["explorer.txs.events"]())).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: m["explorer.txs.message"]() }));
    fireEvent.click(screen.getByRole("button", { name: m["explorer.txs.backtrace"]() }));

    const visualizedPayloads = screen
      .getAllByTestId("json-visualizer")
      .map((node) => node.textContent);
    expect(visualizedPayloads).toContain(
      JSON.stringify({ error: { message: "insufficient funds" } }),
    );
    expect(visualizedPayloads).toContain(
      JSON.stringify({
        amount: "1500000",
        denom: "uatom",
        to: "0x726563697069656e740000000000000000000000",
      }),
    );
    expect(visualizedPayloads).toContain(indexedTransaction.nestedEvents);
    expect(
      screen.getByText((content) => content.includes("1: execute transfer")),
    ).toBeInTheDocument();
    expect(screen.getByText((content) => content.includes("2: debit account"))).toBeInTheDocument();
  });

  it("renders backend error backtraces without inventing parsed error payloads", async () => {
    explorerDetailMocks.parseExplorerErrorMessage.mockReturnValue({
      backtrace: " 1: verify signature 2: execute message",
    });

    renderTransactionExplorer(<TransactionExplorer.Messages />, {
      ...indexedTransaction,
      errorMessage: "signature verification failed",
      messages: [],
      nestedEvents: "[]",
    });

    expect(explorerDetailMocks.parseExplorerErrorMessage).toHaveBeenCalledWith(
      "signature verification failed",
    );
    expect(await screen.findByText(m["explorer.txs.error"]())).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: m["explorer.txs.message"]() }),
    ).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: m["explorer.txs.backtrace"]() })).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: m["explorer.txs.backtrace"]() }));

    expect(
      screen.getByText((content) => content.includes("1: verify signature")),
    ).toBeInTheDocument();
    expect(
      screen.getByText((content) => content.includes("2: execute message")),
    ).toBeInTheDocument();
    expect(screen.getByTestId("json-visualizer").textContent).toBe("[]");
  });

  it("renders successful transaction messages without an error accordion", async () => {
    explorerDetailMocks.parseExplorerErrorMessage.mockReturnValue({});
    renderTransactionExplorer(
      <>
        <TransactionExplorer.Details />
        <TransactionExplorer.Messages />
      </>,
      {
        ...indexedTransaction,
        errorMessage: undefined,
        gasUsed: 21000,
        gasWanted: 25000,
        hasSucceeded: true,
        nestedEvents: "[]",
      },
    );

    expect(await screen.findByText(m["explorer.txs.success"]())).toBeInTheDocument();
    expect(screen.getByText("21000")).toBeInTheDocument();
    expect(screen.getByText("25000")).toBeInTheDocument();
    expect(explorerDetailMocks.parseExplorerErrorMessage).toHaveBeenCalledWith(undefined);
    expect(screen.queryByText(m["explorer.txs.error"]())).not.toBeInTheDocument();
    expect(screen.getByText(m["explorer.txs.messages"]())).toBeInTheDocument();
    expect(screen.getByText(m["explorer.txs.events"]())).toBeInTheDocument();
    expect(screen.getByText("transfer")).toBeInTheDocument();
    const visualizedPayloads = screen
      .getAllByTestId("json-visualizer")
      .map((node) => node.textContent);
    expect(visualizedPayloads).toContain(
      JSON.stringify({
        amount: "1500000",
        denom: "uatom",
        to: "0x726563697069656e740000000000000000000000",
      }),
    );
    expect(visualizedPayloads).toContain("[]");
  });

  it("renders empty backend transaction message lists without inventing message rows", async () => {
    renderTransactionExplorer(<TransactionExplorer.Messages />, {
      ...indexedTransaction,
      errorMessage: "",
      hasSucceeded: true,
      messages: [],
      nestedEvents: "[]",
    });

    expect(explorerDetailMocks.parseExplorerErrorMessage).toHaveBeenCalledWith("");
    expect(await screen.findByText(m["explorer.txs.messages"]())).toBeInTheDocument();
    expect(screen.queryByText(m["explorer.txs.error"]())).not.toBeInTheDocument();
    expect(screen.getByText(m["explorer.txs.events"]())).toBeInTheDocument();
    expect(screen.queryByText("transfer")).not.toBeInTheDocument();
    expect(screen.getByTestId("json-visualizer").textContent).toBe("[]");
  });

  it("renders the transaction not-found state with the searched hash", () => {
    explorerDetailMocks.useExplorerTransaction.mockReturnValue({
      data: null,
      isLoading: false,
    });

    renderWithQueryClient(
      <TransactionExplorer txHash={failedTxHash}>
        <TransactionExplorer.NotFound />
      </TransactionExplorer>,
    );

    expect(screen.getByText(m["explorer.txs.notFound.title"]())).toBeInTheDocument();
    expect(screen.getByText((content) => content.includes(failedTxHash))).toBeInTheDocument();
  });

  it("renders block details, cron outcomes, copy target, and the block transaction table", async () => {
    renderBlockExplorer(
      <>
        <BlockExplorer.Details />
        <BlockExplorer.CronsOutcomes />
        <BlockExplorer.TxTable />
      </>,
    );

    expect(
      await screen.findByText(m["explorer.block.details.blockDetails"]({ height: "#1234" })),
    ).toBeInTheDocument();
    expect(screen.getByText(blockHash)).toBeInTheDocument();

    fireEvent.pointerUp(copyButtonBeside(blockHash));
    expect(navigator.clipboard.writeText).toHaveBeenCalledWith(blockHash);

    expect(screen.getByText("Leftcurve Validator")).toBeInTheDocument();
    expect(
      screen.getByText(new Date(blockExplorerData.searchBlock.createdAt).toLocaleString()),
    ).toBeInTheDocument();
    expect(screen.getByText("transactions-table:1")).toBeInTheDocument();
    expect(screen.getByText(m["explorer.block.cronsOutcomes"]())).toBeInTheDocument();

    expect(screen.getByTestId("json-visualizer").textContent).toContain('"kind":"cron"');
    expect(screen.getByTestId("json-visualizer").textContent).toContain("plain cron outcome");
  });

  it("renders genesis block details with zero transactions and empty cron outcomes", async () => {
    renderBlockExplorer(
      <>
        <BlockExplorer.Details />
        <BlockExplorer.CronsOutcomes />
        <BlockExplorer.TxTable />
      </>,
      {
        ...blockExplorerData,
        currentBlock: {
          blockHeight: 0,
          hash: blockHash,
        },
        height: 0,
        searchBlock: {
          blockHeight: 0,
          createdAt: "2026-06-08T00:00:00.000Z",
          cronsOutcomes: [],
          hash: blockHash,
          transactions: [],
        },
      },
    );

    expect(
      await screen.findByText(m["explorer.block.details.blockDetails"]({ height: "#0" })),
    ).toBeInTheDocument();
    expect(detailRow(m["explorer.block.details.numberOfTx"]()).getByText("0")).toBeInTheDocument();
    expect(
      detailRow(m["explorer.block.details.blockTime"]()).getByText(
        new Date("2026-06-08T00:00:00.000Z").toLocaleString(),
      ),
    ).toBeInTheDocument();
    expect(screen.getByText("transactions-table:0")).toBeInTheDocument();
    expect(screen.getByTestId("json-visualizer").textContent).toBe("[]");
  });

  it("renders future block estimates from current backend height", async () => {
    const now = new Date("2026-06-09T12:00:00.000Z").getTime();
    const dateNowSpy = vi.spyOn(Date, "now").mockReturnValue(now);
    const targetBlockTime = now + 8 * 500;

    try {
      renderBlockExplorer(<BlockExplorer.FutureBlock />, {
        currentBlock: {
          blockHeight: 1234,
          hash: blockHash,
        },
        height: 1242,
        isFutureBlock: true,
        isInvalidBlock: false,
        searchBlock: null,
      });

      expect(
        await screen.findByText(`${m["explorer.block.futureBlock.targetBlock"]()} 1242`),
      ).toBeInTheDocument();
      expect(
        screen.getByText(m["explorer.block.futureBlock.hasNotBeenCreated"]({ height: 1242 })),
      ).toBeInTheDocument();
      expect(screen.getByText(new Date(targetBlockTime).toISOString())).toBeInTheDocument();
      expect(screen.getByText(new Date(targetBlockTime).toUTCString())).toBeInTheDocument();
      expect(screen.getByText("#1242")).toBeInTheDocument();
      expect(screen.getByText("#1236")).toBeInTheDocument();
      expect(screen.getByText("#6")).toBeInTheDocument();
      expect(explorerDetailMocks.useCountdown).toHaveBeenLastCalledWith({
        date: targetBlockTime,
      });
    } finally {
      dateNowSpy.mockRestore();
    }
  });

  it("renders the invalid block not-found state without a backend search block", () => {
    renderBlockExplorer(<BlockExplorer.NotFound />, {
      currentBlock: {
        blockHeight: 100,
        hash: blockHash,
      },
      height: Number.NaN,
      isFutureBlock: false,
      isInvalidBlock: true,
      searchBlock: null,
    });

    expect(screen.getByText(m["explorer.block.notFound.title"]())).toBeInTheDocument();
    expect(screen.getByText(m["explorer.block.notFound.description"]())).toBeInTheDocument();
  });
});
