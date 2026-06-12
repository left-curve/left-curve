import { cleanup, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import { Footer } from "../src/components/foundation/Footer";

const footerMocks = vi.hoisted(() => ({
  pathname: "/trade/BTCUSD",
  perpsPairStats: [] as Array<{
    currentPrice: string | null;
    pairId: string;
    priceChange24H: string | null;
  }>,
}));

vi.mock("@tanstack/react-router", () => ({
  useRouter: () => ({
    state: {
      location: {
        pathname: footerMocks.pathname,
      },
    },
  }),
}));

vi.mock("@left-curve/store", () => ({
  useAllPerpsPairStats: (
    selector: (state: { perpsPairStats: typeof footerMocks.perpsPairStats }) => unknown,
  ) =>
    selector({
      perpsPairStats: footerMocks.perpsPairStats,
    }),
}));

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

vi.mock("../src/components/foundation/StatusBadge", () => ({
  StatusBadge: ({ className }: { className?: string }) => (
    <div className={className} data-testid="footer-status-badge" />
  ),
}));

function setPerpsPairStats(stats: typeof footerMocks.perpsPairStats) {
  footerMocks.perpsPairStats = stats;
}

describe("Footer shell", () => {
  beforeEach(() => {
    footerMocks.pathname = "/trade/BTCUSD";
    class ResizeObserverMock {
      disconnect = vi.fn();
      observe = vi.fn();
      unobserve = vi.fn();
    }

    Object.defineProperty(globalThis, "ResizeObserver", {
      configurable: true,
      value: ResizeObserverMock,
    });
    setPerpsPairStats([
      {
        currentPrice: "67000.5",
        pairId: "perp/btcusd",
        priceChange24H: "2.45",
      },
      {
        currentPrice: "3350.25",
        pairId: "perp/ethusd",
        priceChange24H: "-1.5",
      },
      {
        currentPrice: null,
        pairId: "perp/solusd",
        priceChange24H: null,
      },
    ]);
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("renders backend-fed perps pair stats in the trade footer ticker", () => {
    render(<Footer />);

    const footer = screen.getByRole("contentinfo");
    expect(footer).toHaveClass("shadow-account-card");
    expect(screen.getByTestId("footer-status-badge")).toHaveClass("static flex");

    expect(within(footer).getAllByText("BTCUSD").length).toBeGreaterThan(0);
    expect(within(footer).getAllByText("ETHUSD").length).toBeGreaterThan(0);
    expect(within(footer).getAllByText("SOLUSD").length).toBeGreaterThan(0);
    expect(footer).toHaveTextContent("BTCUSD+2.45%67,001");
    expect(footer).toHaveTextContent("ETHUSD-1.50%3,350.25");
    expect(footer).toHaveTextContent("SOLUSD--");
  });

  it("preserves footer links and avoids trade-only shadow on non-trade routes", () => {
    footerMocks.pathname = "/earn";
    setPerpsPairStats([]);

    render(<Footer />);

    expect(screen.getByRole("contentinfo")).not.toHaveClass("shadow-account-card");
    expect(screen.getByRole("link", { name: m["footer.terms"]() })).toHaveAttribute(
      "href",
      "/documents/Dango - Terms of Use.pdf",
    );
    expect(screen.getByRole("link", { name: m["footer.privacyPolicy"]() })).toHaveAttribute(
      "href",
      "/documents/Dango - Privacy Policy.pdf",
    );
    expect(screen.getByRole("link", { name: "Discord" })).toHaveAttribute(
      "href",
      "https://discord.gg/BWJtyySxBM",
    );
    expect(screen.getByRole("link", { name: "Twitter" })).toHaveAttribute(
      "href",
      "https://x.com/dango",
    );
  });
});
