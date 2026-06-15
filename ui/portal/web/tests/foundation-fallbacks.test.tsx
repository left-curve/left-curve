import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import {
  resetAppletsKitMocks,
  setAppletsKitUseTheme,
} from "./mocks/applets-kit";

import { ChunkErrorFallback } from "../src/components/foundation/ChunkErrorFallback";
import { Maintenance } from "../src/components/foundation/Maintenance";
import { NotFound } from "../src/components/foundation/NotFound";

const fallbackMocks = vi.hoisted(() => ({
  navigate: vi.fn(),
}));

vi.mock("@tanstack/react-router", () => ({
  useNavigate: () => fallbackMocks.navigate,
}));

describe("foundation fallbacks", () => {
  beforeEach(() => {
    resetAppletsKitMocks();
    setAppletsKitUseTheme({
      theme: "light",
    });
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("renders the not-found state and routes users back home", () => {
    render(<NotFound />);

    expect(screen.getByAltText("404 Not Found")).toHaveAttribute(
      "src",
      "/images/characters/emptybox1.svg",
    );
    expect(screen.getByText(m["notFound.title"]())).toBeInTheDocument();
    expect(screen.getByText(m["notFound.description"]())).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: m["notFound.goToHome"]() }));

    expect(fallbackMocks.navigate).toHaveBeenCalledWith({
      to: "/",
    });
  });

  it("lets error boundaries retry failed lazy content", () => {
    const resetErrorBoundary = vi.fn();

    render(<ChunkErrorFallback resetErrorBoundary={resetErrorBoundary} />);

    expect(screen.getByText(m["common.failedToLoad"]())).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: m["common.retry"]() }));

    expect(resetErrorBoundary).toHaveBeenCalledOnce();
  });

  it("renders maintenance copy with light theme assets", () => {
    render(<Maintenance />);

    expect(screen.getByAltText("bg-image")).toHaveAttribute("src", "/images/union.png");
    expect(screen.getByAltText("Dango")).toHaveAttribute("src", "/images/dango.svg");
    expect(screen.getByAltText("Maintenance")).toHaveAttribute(
      "src",
      "/images/characters/grugo.svg",
    );
    expect(screen.getByText(m["maintenance.title"]())).toBeInTheDocument();
    expect(screen.getByText(m["maintenance.description"]())).toBeInTheDocument();
  });

  it("uses dark theme assets for maintenance mode", () => {
    setAppletsKitUseTheme({
      theme: "dark",
    });

    render(<Maintenance />);

    expect(screen.getByAltText("bg-image")).toHaveAttribute("src", "/images/union-dark.png");
    expect(screen.getByAltText("Dango")).toHaveAttribute("src", "/images/dango-dark.svg");
  });
});
