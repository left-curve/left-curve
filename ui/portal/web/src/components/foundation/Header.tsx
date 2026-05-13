import { useAccount, useConfig } from "@left-curve/store";
import { useRouterState } from "@tanstack/react-router";
import {
  IconGift,
  IconWalletWithCross,
  Modals,
  useApp,
  useCountdown,
  useMediaQuery,
  Tooltip,
} from "@left-curve/applets-kit";

import { Button, IconButton, twMerge } from "@left-curve/applets-kit";
import { Link } from "@tanstack/react-router";
import { useMemo } from "react";
import { AccountMenu } from "./AccountMenu";
import { SearchMenu } from "./SearchMenu";
import { TxIndicator } from "./TxIndicator";

import { m } from "@left-curve/foundation/paraglide/messages.js";
import { TestnetBanner } from "./TestnetBanner";

interface HeaderProps {
  isScrolled: boolean;
}

export const Header: React.FC<HeaderProps> = ({ isScrolled }) => {
  const { account, isConnected, isUserActive } = useAccount();
  const { chain } = useConfig();

  const { showModal, setSidebarVisibility, isSidebarVisible, isSearchBarVisible } = useApp();
  const { location } = useRouterState();
  const { isLg } = useMediaQuery();

  const isMainnet = !["Devnet", "Testnet"].includes(chain.name);

  // TODO: Re-enable once the points service is ready.
  // const pointsUrl = window.dango.urls.pointsUrl;
  // const { isStarted, startsAt } = useCurrentEpoch({ pointsUrl, enabled: isMainnet });

  // Fixed countdown target: 2026-04-15 12:00:00 UTC
  const startDate = new Date("2026-04-15T12:00:00Z");
  const isStarted = new Date() >= startDate;

  const countdown = useCountdown({ date: startDate ?? undefined });

  const campaignStartLabel = useMemo(() => {
    if (!startDate) return null;
    const dateLabel = startDate.toLocaleDateString(undefined, {
      month: "long",
      day: "numeric",
    });

    const days = Number(countdown.days);
    const hours = Number(countdown.hours);
    const minutes = Number(countdown.minutes);
    const seconds = Number(countdown.seconds);

    let remaining: string | null = null;
    if (Number.isFinite(days + hours + minutes + seconds)) {
      if (days > 0) remaining = `${days}d ${hours}h ${minutes}m`;
      else if (hours > 0) remaining = `${hours}h ${minutes}m ${seconds}s`;
      else if (minutes > 0) remaining = `${minutes}m ${seconds}s`;
      else if (seconds > 0) remaining = `${seconds}s`;
    }

    return remaining ? `${dateLabel} · ${remaining}` : dateLabel;
  }, [startDate, countdown]);

  const isCampaignLocked = isMainnet && !isStarted;
  const isProSwap = location.pathname.includes("trade");
  const hideSearchBar = (isProSwap && !isLg) || (location.pathname === "/" && isLg);

  return (
    <header
      className={twMerge(
        "fixed bottom-0 lg:top-0 left-0 right-0 bg-transparent z-50 transition-[background,box-shadow] w-full",
        isScrolled
          ? "lg:bg-surface-primary-rice lg:shadow-account-card"
          : "bg-transparent shadow-none",
        location.pathname === "/" ? "lg:fixed h-fit" : "lg:sticky flex flex-col items-center",
      )}
    >
      {isLg ? <div id="quest-banner" className="w-full" /> : null}
      {isLg ? <TestnetBanner /> : null}

      <div className="w-full gap-4 relative flex flex-wrap lg:flex-nowrap items-center justify-center xl:grid xl:grid-cols-4 max-w-[76rem] mx-auto p-4">
        <Link to="/" className="w-fit drag-none">
          <img
            src="/dango-logo.svg"
            alt="dango logo"
            className="h-11 order-1 cursor-pointer drag-none hidden lg:flex rounded-full shadow-account-card select-none bg-surface-secondary-rice"
          />
        </Link>
        <div
          className={twMerge(
            "xl:col-span-2 z-50 min-w-full lg:min-w-0 flex-1 order-3 lg:order-2 flex items-end justify-center gap-2 fixed lg:relative bottom-0 lg:bottom-auto left-0 transition-all p-4 lg:p-0",
            {
              "bottom-6": window.matchMedia("(display-mode: standalone)").matches,
            },
          )}
        >
          {!hideSearchBar ? <SearchMenu /> : null}
          {isProSwap ? (
            <div
              id="trade-buttons"
              className="flex gap-2 items-center justify-center w-full lg:hidden"
            />
          ) : null}
          {!isSearchBarVisible ? (
            <div className="flex gap-2 lg:hidden">
              {isCampaignLocked ? (
                <Tooltip
                  content={
                    campaignStartLabel
                      ? m["points.campaignStartsOn"]({ date: campaignStartLabel })
                      : m["points.campaign"]()
                  }
                  placement="top"
                >
                  <IconButton
                    size="lg"
                    type="button"
                    className="rounded-lg shadow-account-card"
                    isDisabled
                  >
                    <IconGift />
                  </IconButton>
                </Tooltip>
              ) : (
                <IconButton
                  as={Link}
                  to="/points"
                  size="lg"
                  type="button"
                  className="rounded-lg shadow-account-card"
                >
                  <IconGift />
                </IconButton>
              )}
              <IconButton
                onClick={() =>
                  isConnected ? setSidebarVisibility(true) : showModal(Modals.Authenticate)
                }
                variant="utility"
                size="lg"
                type="button"
                className="shadow-account-card lg:hidden"
              >
                <TxIndicator icon={<IconWalletWithCross isCrossVisible={!isUserActive} />} />
              </IconButton>
            </div>
          ) : null}
        </div>
        <div className="hidden lg:flex gap-4 items-center justify-end order-2 lg:order-3">
          {isCampaignLocked ? (
            <Tooltip
              content={
                campaignStartLabel
                  ? m["points.campaignStartsOn"]({ date: campaignStartLabel })
                  : m["points.campaign"]()
              }
              placement="bottom"
            >
              <Button size="lg" className="rounded-lg" isDisabled>
                {m["points.campaign"]()}
              </Button>
            </Tooltip>
          ) : (
            <Button as={Link} to="/points" size="lg" className="rounded-lg">
              {m["points.campaign"]()}
            </Button>
          )}
          <Button
            dng-connect-button="true"
            variant="utility"
            size="lg"
            onClick={() =>
              isConnected ? setSidebarVisibility(!isSidebarVisible) : showModal(Modals.Authenticate)
            }
          >
            {isConnected ? (
              <div className="flex items-center justify-center gap-2">
                <TxIndicator icon={<IconWalletWithCross isCrossVisible={!isUserActive} />} />
                <span
                  className={twMerge("italic font-exposure font-bold capitalize", {
                    "text-ink-placeholder-400": !isUserActive,
                  })}
                >
                  {m["common.account"]()} #{account?.index}
                </span>
              </div>
            ) : (
              <span>{m["common.signin"]()}</span>
            )}
          </Button>
        </div>
      </div>
      <AccountMenu />
    </header>
  );
};
