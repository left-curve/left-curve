import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import {
  resetAppletsKitMocks,
  setAppletsKitUseAppFactory,
  setAppletsKitUsePortalTargetFactory,
} from "./mocks/applets-kit";

import { Modals } from "@left-curve/applets-kit";

import { TradeButtons } from "../src/components/dex/components/TradeButtons";

const tradeButtonMocks = vi.hoisted(() => ({
  isConnected: true,
  navigate: vi.fn(),
  onChangeAction: vi.fn(),
  setTradeBarVisibility: vi.fn(),
  showModal: vi.fn(),
  usePortalTarget: vi.fn(),
}));

vi.mock("@tanstack/react-router", () => ({
  useNavigate: () => tradeButtonMocks.navigate,
}));

vi.mock("../src/components/dex/components/ProTrade", () => ({
  useProTrade: () => ({
    onChangeAction: tradeButtonMocks.onChangeAction,
    pair: {
      base: {
        symbol: "BTC",
      },
    },
  }),
}));

vi.mock("@left-curve/store", () => ({
  useAccount: () => ({
    isConnected: tradeButtonMocks.isConnected,
  }),
}));

function renderTradeButtons() {
  const portalTarget = document.createElement("div");
  portalTarget.id = "trade-buttons";
  document.body.appendChild(portalTarget);
  tradeButtonMocks.usePortalTarget.mockReturnValue(portalTarget);

  return {
    portalTarget,
    ...render(<TradeButtons />),
  };
}

describe("DEX mobile trade buttons", () => {
  beforeEach(() => {
    resetAppletsKitMocks();
    tradeButtonMocks.isConnected = true;
    setAppletsKitUseAppFactory(() => ({
      setTradeBarVisibility: tradeButtonMocks.setTradeBarVisibility,
      showModal: tradeButtonMocks.showModal,
    }));
    setAppletsKitUsePortalTargetFactory(
      (selector) => tradeButtonMocks.usePortalTarget(selector) as Element | null,
    );
  });

  afterEach(() => {
    cleanup();
    document.body.innerHTML = "";
    vi.clearAllMocks();
  });

  it("does not render actions when the portal target is unavailable", () => {
    tradeButtonMocks.usePortalTarget.mockReturnValue(null);

    render(<TradeButtons />);

    expect(
      screen.queryByRole("button", { name: `${m["proSwap.buy"]()} BTC` }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: `${m["proSwap.sell"]()} BTC` }),
    ).not.toBeInTheDocument();
  });

  it("navigates back to the app home from the utility button", () => {
    const { portalTarget } = renderTradeButtons();
    const [backButton] = within(portalTarget).getAllByRole("button");

    fireEvent.click(backButton);

    expect(tradeButtonMocks.navigate).toHaveBeenCalledWith({
      to: "/",
    });
  });

  it("opens the trade bar and sets buy or sell action for connected accounts", () => {
    renderTradeButtons();

    fireEvent.click(screen.getByRole("button", { name: `${m["proSwap.buy"]()} BTC` }));

    expect(tradeButtonMocks.setTradeBarVisibility).toHaveBeenCalledWith(true);
    expect(tradeButtonMocks.onChangeAction).toHaveBeenCalledWith("buy");

    fireEvent.click(screen.getByRole("button", { name: `${m["proSwap.sell"]()} BTC` }));

    expect(tradeButtonMocks.setTradeBarVisibility).toHaveBeenCalledTimes(2);
    expect(tradeButtonMocks.onChangeAction).toHaveBeenLastCalledWith("sell");
    expect(tradeButtonMocks.showModal).not.toHaveBeenCalled();
  });

  it("opens authentication instead of trade actions for disconnected users", () => {
    tradeButtonMocks.isConnected = false;

    renderTradeButtons();

    expect(
      screen.queryByRole("button", { name: `${m["proSwap.buy"]()} BTC` }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: `${m["proSwap.sell"]()} BTC` }),
    ).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: m["common.connect"]() }));

    expect(tradeButtonMocks.showModal).toHaveBeenCalledWith(Modals.Authenticate, {
      action: "signin",
    });
    expect(tradeButtonMocks.setTradeBarVisibility).not.toHaveBeenCalled();
    expect(tradeButtonMocks.onChangeAction).not.toHaveBeenCalled();
  });
});
