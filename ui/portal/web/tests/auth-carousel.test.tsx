import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import {
  resetAppletsKitMocks,
  setAppletsKitUseApp,
  setAppletsKitUseTheme,
} from "./mocks/applets-kit";

import { AuthCarousel } from "../src/components/auth/AuthCarousel";

const authCarouselMocks = vi.hoisted(() => ({
  changeSettings: vi.fn(),
}));

describe("AuthCarousel", () => {
  beforeEach(() => {
    resetAppletsKitMocks();
    setAppletsKitUseApp({
      changeSettings: authCarouselMocks.changeSettings,
      settings: {
        isFirstVisit: true,
      },
    });
    setAppletsKitUseTheme({
      theme: "light",
    });
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("renders the localized onboarding steps with the matching carousel images", async () => {
    render(<AuthCarousel />);

    for (const step of [0, 1, 2]) {
      fireEvent.click(screen.getAllByRole("button")[step]);

      const title = m["signup.carousel.title"]({ step });
      await waitFor(() => {
        expect(screen.getByRole("img", { name: title })).toHaveAttribute(
          "src",
          expect.stringContaining(["stonk", "leverage", "smaug"][step]),
        );
      });
      expect(screen.getByText(title)).toBeInTheDocument();
      expect(screen.getByText(m["signup.carousel.description"]({ step }))).toBeInTheDocument();
    }
  });

  it("uses the theme-specific frame assets and preserves desktop visibility after first visit", () => {
    setAppletsKitUseApp({
      changeSettings: authCarouselMocks.changeSettings,
      settings: {
        isFirstVisit: false,
      },
    });
    setAppletsKitUseTheme({
      theme: "dark",
    });

    const { container } = render(<AuthCarousel />);
    const root = container.firstElementChild;

    expect(root).toHaveClass("hidden", "xl:flex");
    expect(root).toHaveClass("xl:bg-[url('./images/dark-frame-rounded.svg')]");
    expect(root).not.toHaveClass("fixed");
  });

  it("persists the first-visit dismissal from the onboarding action", () => {
    render(<AuthCarousel />);

    fireEvent.click(screen.getByRole("button", { name: m["common.continue"]() }));

    expect(authCarouselMocks.changeSettings).toHaveBeenCalledWith({
      isFirstVisit: false,
    });
  });
});
