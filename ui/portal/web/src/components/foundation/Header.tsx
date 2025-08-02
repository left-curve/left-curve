import {
  Button,
  IconBell,
  IconGear,
  IconUser,
  twMerge,
  useMediaQuery,
} from "@left-curve/applets-kit";

import { useAccount } from "@left-curve/store";
import { Link, useNavigate, useRouterState } from "@tanstack/react-router";
import { useRef } from "react";
import { useApp } from "~/hooks/useApp";
import { m } from "~/paraglide/messages";
import { TradeButtons } from "../dex/TradeButtons";
import { NotificationsMenu } from "../notifications/NotificationsMenu";
import { AccountMenu } from "./AccountMenu";
import { Hamburger } from "./Hamburguer";
import { SearchMenu } from "./SearchMenu";
import { TxIndicator } from "./TxIndicator";

interface HeaderProps {
  isScrolled: boolean;
}

export const Header: React.FC<HeaderProps> = ({ isScrolled }) => {
  const { account, isConnected } = useAccount();

  const {
    setSidebarVisibility,
    setNotificationMenuVisibility,
    isNotificationMenuVisible,
    isSidebarVisible,
  } = useApp();
  const { location } = useRouterState();
  const navigate = useNavigate();
  const { isLg } = useMediaQuery();
  const buttonNotificationsRef = useRef<HTMLButtonElement>(null);

  const linkStatus = (path: string) => (location.pathname.startsWith(path) ? "active" : "");
  const isProSwap = location.pathname.includes("trade");

  const hideSearchBar = isProSwap && !isLg || location.pathname === "/" && isLg;

  return (
    <header
      className={twMerge(
        "fixed lg:sticky bottom-0 lg:top-0 left-0 bg-transparent z-50 w-full transition-all",
        isScrolled
          ? "lg:bg-surface-primary-rice lg:shadow-account-card"
          : "bg-transparent shadow-none",
      )}
    >
      <div className="gap-4 relative flex flex-wrap lg:flex-nowrap items-center justify-center xl:grid xl:grid-cols-4 max-w-[76rem] mx-auto p-4">
        <Link to="/" className="w-fit">
          <img
            src="/favicon.svg"
            alt="dango logo"
            className="h-11 order-1 cursor-pointer hidden lg:flex rounded-full shadow-account-card"
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
          {!hideSearchBar ?  (
            <SearchMenu />
          ) : null}
          { isProSwap && !isLg ? <div id="trade-buttons" className="flex gap-2 items-center justify-center w-full" /> : null }
          <Hamburger />
        </div>
        <div className="hidden lg:flex gap-2 items-center justify-end order-2 lg:order-3">
          <Button
            as={Link}
            variant="utility"
            size="lg"
            to="/settings"
            className=""
            data-status={linkStatus("/settings")}
          >
            <IconGear className="w-6 h-6" />
          </Button>

          {isConnected ? (
            <TxIndicator
              as={Button}
              variant="utility"
              size="lg"
              data-status={linkStatus("/notifications")}
              onClick={() => setNotificationMenuVisibility(!isNotificationMenuVisible)}
            >
              <Button
                ref={buttonNotificationsRef}
                variant="utility"
                size="lg"
                data-status={linkStatus("/notifications")}
                onClick={() => setNotificationMenuVisibility(!isNotificationMenuVisible)}
              >
                <IconBell className="w-6 h-6" />
              </Button>
            </TxIndicator>
          ) : null}
          <Button
            dng-connect-button="true"
            variant="utility"
            size="lg"
            onClick={() =>
              isConnected ? setSidebarVisibility(!isSidebarVisible) : navigate({ to: "/signin" })
            }
          >
            {isConnected ? (
              <>
                <IconUser className="w-6 h-6" />
                <span className="italic font-exposure font-bold">{account?.username}</span>
              </>
            ) : (
              <span>{m["common.signin"]()}</span>
            )}
          </Button>
        </div>
        <NotificationsMenu buttonRef={buttonNotificationsRef} />
      </div>
      {isLg ? <AccountMenu.Desktop /> : <AccountMenu.Mobile />}
    </header>
  );
};
