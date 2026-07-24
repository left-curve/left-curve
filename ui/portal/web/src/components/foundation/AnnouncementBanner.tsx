import { IconClose, useMediaQuery, usePortalTarget } from "@left-curve/applets-kit";
import { useState } from "react";
import { createPortal } from "react-dom";

import { m } from "@left-curve/foundation/paraglide/messages.js";

const ANNOUNCEMENT_URL = "https://x.com/dango/status/2080707796144705625";

interface AnnouncementBannerProps {
  onDismiss: () => void;
}

export function AnnouncementBanner({ onDismiss }: AnnouncementBannerProps) {
  return (
    <div
      role="alert"
      className="relative z-10 flex w-full items-center justify-center bg-account-card-blue px-4 py-3 shadow-account-card"
    >
      <p className="diatype-sm-medium max-w-[76rem] pr-8 text-center text-ink-secondary-700">
        {m["announcement.windingDown"]()}{" "}
        <a
          className="diatype-sm-heavy break-all underline underline-offset-[3px] hover:text-ink-primary-900"
          href={ANNOUNCEMENT_URL}
          target="_blank"
          rel="noopener noreferrer"
        >
          {m["announcement.readMore"]()}
        </a>
        .
      </p>
      <button
        type="button"
        aria-label={m["common.dismiss"]()}
        className="absolute right-3 top-3 flex h-6 w-6 items-center justify-center rounded-full text-ink-tertiary-500 hover:text-ink-primary-900"
        onClick={onDismiss}
      >
        <IconClose className="h-6 w-6" />
      </button>
    </div>
  );
}

export function AnnouncementBannerRender() {
  const [isVisible, setIsVisible] = useState(true);
  const { isLg } = useMediaQuery();
  const desktopContainer = usePortalTarget("#announcement-banner");
  const mobileContainer = usePortalTarget("#announcement-banner-mobile");
  const container = isLg ? desktopContainer : mobileContainer;

  if (!isVisible || !container) return null;

  return createPortal(<AnnouncementBanner onDismiss={() => setIsVisible(false)} />, container);
}
