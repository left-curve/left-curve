import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { TestnetBanner } from "../src/components/foundation/TestnetBanner";

class MockResizeObserver {
  disconnect = vi.fn();
  observe = vi.fn();
  unobserve = vi.fn();
}

function setPathname(pathname: string) {
  window.history.pushState({}, "", pathname);
}

function setBanner(text: string | undefined) {
  Object.defineProperty(window, "dango", {
    configurable: true,
    value: text === undefined ? {} : { banner: text },
  });
}

describe("TestnetBanner", () => {
  beforeEach(() => {
    vi.stubGlobal("ResizeObserver", MockResizeObserver);
    setPathname("/trade");
    setBanner("Dango testnet is live");
  });

  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
  });

  it("renders the configured runtime banner and lets users dismiss it", () => {
    render(<TestnetBanner />);

    expect(screen.getAllByText("Dango testnet is live").length).toBeGreaterThan(0);

    fireEvent.click(screen.getByRole("button"));

    expect(screen.queryByText("Dango testnet is live")).not.toBeInTheDocument();
  });

  it("does not render when the runtime banner is not configured", () => {
    setBanner(undefined);

    const { container } = render(<TestnetBanner />);

    expect(container).toBeEmptyDOMElement();
  });

  it("keeps the banner above the landing page chrome", () => {
    setPathname("/");

    const { container } = render(<TestnetBanner />);

    expect(container.firstElementChild).toHaveClass("relative", "z-50");
  });
});
