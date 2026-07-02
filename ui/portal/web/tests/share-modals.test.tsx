import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { format } from "date-fns";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import {
  resetAppletsKitMocks,
  setAppletsKitUseApp,
  setAppletsKitUseTheme,
} from "./mocks/applets-kit";

import { PnlShare } from "../src/components/modals/pnl-share/PnlShare";
import { PointsShare } from "../src/components/modals/points-share/PointsShare";
import { shareCardFontEmbedCSS } from "../src/components/modals/shareCardFonts";

const shareModalMocks = vi.hoisted(() => ({
  getReferralLink: vi.fn(),
  hideModal: vi.fn(),
  saveCardAsImage: vi.fn(),
  useAccount: vi.fn(),
}));

vi.mock("@left-curve/foundation", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@left-curve/foundation")>();

  return {
    ...actual,
    saveCardAsImage: shareModalMocks.saveCardAsImage,
    useApp: () => ({
      hideModal: shareModalMocks.hideModal,
      settings: {
        formatNumberOptions: {
          language: "en-US",
          mask: 1,
        },
      },
    }),
  };
});

vi.mock("@left-curve/store", () => ({
  getReferralLink: shareModalMocks.getReferralLink,
  useAccount: shareModalMocks.useAccount,
  useConfig: () => ({
    coins: {
      bySymbol: {
        BTC: {
          logoURI: "/btc.png",
        },
        ETH: {
          logoURI: "/eth.png",
        },
      },
    },
  }),
}));

function characterImage() {
  const image = screen.getByAltText("character");
  if (!(image instanceof HTMLImageElement)) throw new Error("Expected character image");
  return image;
}

function getCloseButton(container: HTMLElement) {
  const button = container.querySelector("button.absolute");
  if (!(button instanceof HTMLButtonElement)) throw new Error("Could not find close button");
  return button;
}

describe("share modals", () => {
  beforeEach(() => {
    resetAppletsKitMocks();
    setAppletsKitUseApp({
      hideModal: shareModalMocks.hideModal,
    });
    setAppletsKitUseTheme({
      theme: "light",
    });
    shareModalMocks.useAccount.mockReturnValue({
      userIndex: 7,
    });
    shareModalMocks.getReferralLink.mockReturnValue("https://dango.exchange/ref/7");
    shareModalMocks.saveCardAsImage.mockResolvedValue(undefined);
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("renders points share details, updates the selected character, and saves the image", async () => {
    const { container } = render(<PointsShare points={12_345} weekNumber={9} />);

    expect(screen.getAllByText(m["modals.shareCard.referralCode"]())).toHaveLength(2);
    expect(screen.getAllByText("https://dango.exchange/ref/7")).toHaveLength(2);
    expect(screen.getByText(`${m["modals.pointsShare.weekLabel"]()} 9`)).toBeInTheDocument();
    expect(screen.getByText(m["modals.pointsShare.programLabel"]())).toBeInTheDocument();
    expect(container).toHaveTextContent("12,345");
    expect(screen.getByText(m["modals.shareCard.overlay"]())).toBeInTheDocument();
    expect(characterImage().src).toContain("/images/pnl-modal/frog1.png");

    fireEvent.click(screen.getByRole("button", { name: "clouds" }));

    expect(characterImage().src).toContain("/images/pnl-modal/clouds.png");

    fireEvent.click(screen.getByRole("button", { name: m["modals.shareCard.saveImage"]() }));

    expect(shareModalMocks.saveCardAsImage).toHaveBeenCalledWith(
      expect.objectContaining({
        filename: "points-week-9.png",
        fontEmbedCSS: shareCardFontEmbedCSS,
        source: expect.any(HTMLDivElement),
        width: 752,
      }),
    );

    fireEvent.click(getCloseButton(container));

    expect(shareModalMocks.hideModal).toHaveBeenCalledOnce();
  });

  it("keeps fractional points visible on sub-one point share cards", () => {
    const { container } = render(<PointsShare points={0.0082233} weekNumber={12} />);

    expect(container).toHaveTextContent("0.008223");
  });

  it("normalizes open-position pnl into a leveraged long share card and saves it", () => {
    const { container } = render(
      <PnlShare
        mode="position"
        pairId="BTCUSDC"
        symbol="BTC"
        size="2"
        entryPrice="100"
        currentPrice={125}
        pnl={50}
        equity="50"
      />,
    );

    expect(screen.getByText("BTC")).toBeInTheDocument();
    expect(screen.getByText("Long 5.00x")).toBeInTheDocument();
    expect(screen.getByText("+125.00%")).toBeInTheDocument();
    expect(screen.getByText(m["modals.pnlShare.entryPrice"]())).toBeInTheDocument();
    expect(container).toHaveTextContent("$100.00");
    expect(screen.getByText(m["modals.pnlShare.markPrice"]())).toBeInTheDocument();
    expect(container).toHaveTextContent("$125.00");
    expect(screen.getAllByText("https://dango.exchange/ref/7")).toHaveLength(2);

    fireEvent.click(screen.getByRole("button", { name: "dog1" }));

    expect(characterImage().src).toContain("/images/pnl-modal/dog1.png");

    fireEvent.click(screen.getByRole("button", { name: m["modals.shareCard.saveImage"]() }));

    expect(shareModalMocks.saveCardAsImage).toHaveBeenCalledWith(
      expect.objectContaining({
        filename: "pnl-BTC.png",
        fontEmbedCSS: shareCardFontEmbedCSS,
        source: expect.any(HTMLDivElement),
        width: 752,
      }),
    );
  });

  it("normalizes closed fill pnl into a short share card with a closed-at subtitle", () => {
    const createdAt = "2026-06-08T12:34:00Z";
    const { container } = render(
      <PnlShare
        mode="fill"
        pairId="ETHUSDC"
        symbol="ETH"
        size="-2"
        fillPrice="2000"
        realizedPnl="-100"
        createdAt={createdAt}
      />,
    );

    expect(screen.getByText("ETH")).toBeInTheDocument();
    expect(screen.getByText("Short")).toBeInTheDocument();
    expect(screen.getByText("-2.50%")).toBeInTheDocument();
    expect(
      screen.getByText(
        m["modals.pnlShare.closedAt"]({
          date: format(new Date(createdAt), "MMM d, yyyy HH:mm"),
        }),
      ),
    ).toBeInTheDocument();
    expect(container).toHaveTextContent("$2,000.00");
  });

  it("omits referral sections when the store does not provide a referral link", () => {
    shareModalMocks.getReferralLink.mockReturnValue("");

    render(<PointsShare points={1_000} weekNumber={10} />);

    expect(screen.queryByText(m["modals.shareCard.referralCode"]())).not.toBeInTheDocument();
    expect(shareModalMocks.getReferralLink).toHaveBeenCalledWith(7);

    cleanup();

    render(
      <PnlShare
        mode="position"
        pairId="BTCUSDC"
        symbol="BTC"
        size="1"
        entryPrice="100"
        currentPrice={110}
        pnl={10}
        equity="100"
      />,
    );

    expect(screen.queryByText(m["modals.shareCard.referralCode"]())).not.toBeInTheDocument();
    expect(shareModalMocks.getReferralLink).toHaveBeenCalledWith(7);
  });

  it("passes backend user index zero into share-card referral links", () => {
    shareModalMocks.useAccount.mockReturnValue({
      userIndex: 0,
    });
    shareModalMocks.getReferralLink.mockReturnValue("https://dango.exchange/ref/0");

    render(<PointsShare points={1_000} weekNumber={1} />);

    expect(screen.getAllByText("https://dango.exchange/ref/0")).toHaveLength(2);
    expect(shareModalMocks.getReferralLink).toHaveBeenCalledWith(0);

    cleanup();
    vi.clearAllMocks();
    shareModalMocks.useAccount.mockReturnValue({
      userIndex: 0,
    });
    shareModalMocks.getReferralLink.mockReturnValue("https://dango.exchange/ref/0");

    render(
      <PnlShare
        mode="position"
        pairId="BTCUSDC"
        symbol="BTC"
        size="1"
        entryPrice="100"
        currentPrice={110}
        pnl={10}
        equity="100"
      />,
    );

    expect(screen.getAllByText("https://dango.exchange/ref/0")).toHaveLength(2);
    expect(shareModalMocks.getReferralLink).toHaveBeenCalledWith(0);
  });

  it("keeps the bundled share-card font CSS available for image export", () => {
    expect(shareCardFontEmbedCSS).toContain("font-family: 'ABCDiatypeRounded'");
    expect(shareCardFontEmbedCSS).toContain("font-weight: 400");
    expect(shareCardFontEmbedCSS).toContain("font-weight: 500");
    expect(shareCardFontEmbedCSS).toContain("font-weight: 700");
    expect(shareCardFontEmbedCSS).toContain("font-family: 'Exposure'");
    expect(shareCardFontEmbedCSS).toContain("font-style: italic");
    expect(shareCardFontEmbedCSS).toContain("format('woff2')");
  });
});
