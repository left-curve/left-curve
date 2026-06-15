import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
  resetAppletsKitMocks,
  setAppletsKitMarqueeFactory,
  setAppletsKitUseMediaQueryFactory,
} from "./mocks/applets-kit";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import { CountBadge } from "../src/components/foundation/CountBadge";
import { EmptyPlaceholder } from "../src/components/foundation/EmptyPlaceholder";
import { GeoblockBanner } from "../src/components/foundation/GeoblockBanner";
import { MobileTitle } from "../src/components/foundation/MobileTitle";

const foundationPrimitiveMocks = vi.hoisted(() => ({
  historyGo: vi.fn(),
  isXl: true,
}));

vi.mock("@tanstack/react-router", () => ({
  useRouter: () => ({
    history: {
      go: foundationPrimitiveMocks.historyGo,
    },
  }),
}));

describe("foundation primitives", () => {
  beforeEach(() => {
    foundationPrimitiveMocks.isXl = true;
    resetAppletsKitMocks();
    setAppletsKitUseMediaQueryFactory(() => ({
      isXl: foundationPrimitiveMocks.isXl,
    }));
    setAppletsKitMarqueeFactory(({ className, item, speed }) => (
      <div className={className} data-speed={speed} data-testid="geoblock-marquee">
        {item}
      </div>
    ));
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("renders a mobile title and routes back through router history", () => {
    const { container } = render(<MobileTitle title="Trade" className="p-4" />);

    expect(screen.getByRole("heading", { name: "Trade" })).toBeInTheDocument();
    expect(container.firstElementChild).toHaveClass("lg:hidden", "p-4");

    fireEvent.click(screen.getByRole("button"));

    expect(foundationPrimitiveMocks.historyGo).toHaveBeenCalledWith(-1);
  });

  it("renders string placeholders when no children are provided", () => {
    render(<EmptyPlaceholder className="p-4" component="No balances yet" />);

    expect(screen.getByText("No balances yet")).toHaveClass(
      "diatype-xs-regular",
      "text-ink-secondary-700",
    );
    expect(screen.getByText("No balances yet").parentElement).toHaveClass("p-4");
  });

  it("prefers children over the placeholder component", () => {
    render(
      <EmptyPlaceholder component="No rows">
        <button type="button">Create row</button>
      </EmptyPlaceholder>,
    );

    expect(screen.getByRole("button", { name: "Create row" })).toBeInTheDocument();
    expect(screen.queryByText("No rows")).not.toBeInTheDocument();
  });

  it("renders positive count badges and hides zero or negative counts", () => {
    const { rerender } = render(<CountBadge count={3} />);

    expect(screen.getByText("3")).toHaveClass("rounded-full");

    rerender(<CountBadge count={0} />);
    expect(screen.queryByText("0")).not.toBeInTheDocument();

    rerender(<CountBadge count={-1} />);
    expect(screen.queryByText("-1")).not.toBeInTheDocument();
  });

  it("renders the geoblock warning statically on wide screens and as a marquee below xl", () => {
    const bannerText = [
      m["geoblock.bannerLead"](),
      m["geoblock.bannerEmphasis"](),
      m["geoblock.bannerTail"](),
    ].join(" ");
    const { rerender } = render(<GeoblockBanner />);

    expect(screen.getByRole("alert")).toHaveAttribute("aria-live", "polite");
    expect(screen.getByRole("alert")).toHaveTextContent(bannerText);
    expect(screen.queryByTestId("geoblock-marquee")).not.toBeInTheDocument();

    foundationPrimitiveMocks.isXl = false;
    rerender(<GeoblockBanner />);

    expect(screen.getByTestId("geoblock-marquee")).toHaveAttribute("data-speed", "60");
    expect(screen.getByRole("alert")).toHaveTextContent(bannerText);
  });
});
