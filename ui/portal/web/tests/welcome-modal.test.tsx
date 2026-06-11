import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import {
  resetAppletsKitMocks,
  setAppletsKitUseApp,
} from "./mocks/applets-kit";

import { WelcomeModal } from "../src/components/modals/WelcomeModal";

const welcomeModalMocks = vi.hoisted(() => ({
  changeSettings: vi.fn(),
  chainName: "Testnet",
}));

vi.mock("@left-curve/store", () => ({
  useConfig: () => ({
    chain: {
      name: welcomeModalMocks.chainName,
    },
  }),
}));

describe("welcome modal", () => {
  beforeEach(() => {
    resetAppletsKitMocks();
    setAppletsKitUseApp({
      changeSettings: welcomeModalMocks.changeSettings,
      settings: {
        showWelcome: true,
      },
    });
    welcomeModalMocks.chainName = "Testnet";
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("stays hidden when the user disabled it or the connected chain is not testnet", () => {
    setAppletsKitUseApp({
      changeSettings: welcomeModalMocks.changeSettings,
      settings: {
        showWelcome: false,
      },
    });
    const { container } = render(<WelcomeModal />);

    expect(container).toBeEmptyDOMElement();

    cleanup();
    setAppletsKitUseApp({
      changeSettings: welcomeModalMocks.changeSettings,
      settings: {
        showWelcome: true,
      },
    });
    welcomeModalMocks.chainName = "Mainnet";
    const { container: mainnetContainer } = render(<WelcomeModal />);

    expect(mainnetContainer).toBeEmptyDOMElement();
  });

  it("renders the testnet welcome content and persists dismissal", () => {
    render(<WelcomeModal />);

    expect(screen.getByText(m["common.testnet.title"]())).toBeInTheDocument();
    expect(screen.getByAltText("dango logo")).toHaveAttribute("src", "/favicon.svg");
    expect(screen.getByRole("link", { name: "learn more" })).toHaveAttribute(
      "href",
      "https://x.com/larry0x/status/1947685791167353284",
    );
    expect(screen.getByRole("link", { name: "Galxe" })).toHaveAttribute(
      "href",
      "https://app.galxe.com/quest/dango/GCMTJtfErm",
    );
    expect(screen.getByRole("link", { name: "Discord" })).toHaveAttribute(
      "href",
      "https://discord.gg/BWJtyySxBM",
    );
    expect(screen.getByRole("link", { name: "@larry0x" })).toHaveAttribute(
      "href",
      "https://x.com/larry0x",
    );

    fireEvent.click(screen.getByRole("button", { name: m["common.dismiss"]() }));

    expect(welcomeModalMocks.changeSettings).toHaveBeenCalledWith({
      showWelcome: false,
    });
  });
});
