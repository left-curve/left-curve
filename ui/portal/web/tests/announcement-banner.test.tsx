import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import {
  resetAppletsKitMocks,
  setAppletsKitUseMediaQueryFactory,
  setAppletsKitUsePortalTargetFactory,
} from "./mocks/applets-kit";

import {
  AnnouncementBanner,
  AnnouncementBannerRender,
} from "../src/components/foundation/AnnouncementBanner";

const announcementUrl = "https://x.com/dango/status/2080707796144705625";

describe("AnnouncementBanner", () => {
  beforeEach(() => {
    resetAppletsKitMocks();
  });

  afterEach(() => {
    cleanup();
    document.body.innerHTML = "";
    vi.clearAllMocks();
  });

  it("renders the winding-down notice and timeline link", () => {
    render(<AnnouncementBanner onDismiss={vi.fn()} />);

    expect(screen.getByRole("alert")).toHaveTextContent(m["announcement.windingDown"]());
    expect(screen.getByRole("link", { name: announcementUrl })).toHaveAttribute(
      "href",
      announcementUrl,
    );
    expect(screen.getByRole("link", { name: announcementUrl })).toHaveAttribute("target", "_blank");
  });

  it("dismisses the notice from its accessible close control", () => {
    const onDismiss = vi.fn();
    render(<AnnouncementBanner onDismiss={onDismiss} />);

    fireEvent.click(screen.getByRole("button", { name: m["common.dismiss"]() }));

    expect(onDismiss).toHaveBeenCalledOnce();
  });

  it("portals the notice into the responsive shell target and keeps dismissal state", () => {
    let isLg = true;
    const desktopTarget = document.createElement("div");
    desktopTarget.id = "announcement-banner";
    const mobileTarget = document.createElement("div");
    mobileTarget.id = "announcement-banner-mobile";
    document.body.append(desktopTarget, mobileTarget);

    setAppletsKitUseMediaQueryFactory(() => ({ isLg }));
    setAppletsKitUsePortalTargetFactory((selector) => document.querySelector(selector));

    const { rerender } = render(<AnnouncementBannerRender />);

    expect(within(desktopTarget).getByRole("alert")).toBeInTheDocument();
    expect(mobileTarget).toBeEmptyDOMElement();

    isLg = false;
    rerender(<AnnouncementBannerRender />);

    expect(desktopTarget).toBeEmptyDOMElement();
    expect(within(mobileTarget).getByRole("alert")).toBeInTheDocument();

    fireEvent.click(within(mobileTarget).getByRole("button", { name: m["common.dismiss"]() }));
    isLg = true;
    rerender(<AnnouncementBannerRender />);

    expect(desktopTarget).toBeEmptyDOMElement();
    expect(mobileTarget).toBeEmptyDOMElement();
  });
});
