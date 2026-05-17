import { Marquee, useMediaQuery } from "@left-curve/applets-kit";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import type React from "react";

const BannerBody: React.FC = () => (
  <>
    <span>{m["geoblock.bannerLead"]()}</span>{" "}
    <span className="diatype-xs-heavy">{m["geoblock.bannerEmphasis"]()}</span>{" "}
    <span>{m["geoblock.bannerTail"]()}</span>
  </>
);

export const GeoblockBanner: React.FC = () => {
  const { isXl } = useMediaQuery();

  return (
    <div
      role="alert"
      aria-live="polite"
      className="min-h-8 w-full bg-primitives-error-600 text-primitives-rice-light-25 diatype-xs-medium flex items-center"
    >
      {isXl ? (
        <div className="w-full text-center px-4 py-1.5">
          <BannerBody />
        </div>
      ) : (
        <Marquee
          className="py-1.5"
          speed={60}
          item={
            <span className="inline-flex items-center gap-2 pr-10">
              <BannerBody />
            </span>
          }
        />
      )}
    </div>
  );
};
