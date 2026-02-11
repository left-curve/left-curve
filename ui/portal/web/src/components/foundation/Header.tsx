import { useAccount } from "@left-curve/store";
import { useRouterState } from "@tanstack/react-router";
import {
  IconGift,
  IconWalletWithCross,
  Modals,
  useApp,
  useMediaQuery,
} from "@left-curve/applets-kit";

import { Button, IconButton, twMerge } from "@left-curve/applets-kit";
import { Link } from "@tanstack/react-router";
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

  const { showModal, setSidebarVisibility, isSidebarVisible, isSearchBarVisible } = useApp();
  const { location } = useRouterState();
  const { isLg } = useMediaQuery();

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
        <Link to="/" className="w-fit">
          <img
            src="/dango-logo.svg"
            alt="dango logo"
            className="h-11 order-1 cursor-pointer hidden lg:flex rounded-full shadow-account-card select-none bg-surface-secondary-rice"
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
              <IconButton
                as={Link}
                to="/points"
                size="lg"
                type="button"
                className="rounded-lg shadow-account-card"
              >
                <IconGift />
              </IconButton>
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
          <Button as={Link} to="/points" size="lg" className="rounded-lg">
            {m["points.campaign"]()}
          </Button>
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
