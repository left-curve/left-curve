import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
  resetAppletsKitMocks,
  setAppletsKitUseMediaQuery,
  setAppletsKitUseTheme,
} from "./mocks/applets-kit";

import { Landing } from "../src/components/landing/Landing";

vi.mock("../src/components/foundation/SearchMenu", () => ({
  SearchMenu: () => <div data-testid="landing-search-menu" />,
}));

vi.mock("../src/components/landing/AppletsSection", () => ({
  AppletsSection: () => <div data-testid="landing-applets-section" />,
}));

describe("Landing", () => {
  beforeEach(() => {
    resetAppletsKitMocks();
    setAppletsKitUseMediaQuery({
      isLg: true,
    });
    setAppletsKitUseTheme({
      theme: "light",
    });
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("uses the theme-specific Dango logo and shows desktop search with applets", () => {
    render(<Landing />);

    expect(screen.getByAltText("Dango")).toHaveAttribute("src", "/images/dango.svg");
    expect(screen.getByTestId("landing-search-menu")).toBeInTheDocument();
    expect(screen.getByTestId("landing-applets-section")).toBeInTheDocument();
  });

  it("uses the dark logo and keeps desktop search out of the mobile landing layout", () => {
    setAppletsKitUseMediaQuery({
      isLg: false,
    });
    setAppletsKitUseTheme({
      theme: "dark",
    });

    render(<Landing />);

    expect(screen.getByAltText("Dango")).toHaveAttribute("src", "/images/dango-dark.svg");
    expect(screen.queryByTestId("landing-search-menu")).not.toBeInTheDocument();
    expect(screen.getByTestId("landing-applets-section")).toBeInTheDocument();
  });
});
