import { useAccount } from "@left-curve/store";
import { useNavigate, useRouterState } from "@tanstack/react-router";
import { useApp, useMediaQuery } from "@left-curve/applets-kit";

import { Button, IconButton, IconWallet, twMerge } from "@left-curve/applets-kit";
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
  const { account, isConnected } = useAccount();

  const { setSidebarVisibility, isSidebarVisible, isSearchBarVisible } = useApp();
  const { location } = useRouterState();
  const navigate = useNavigate();
  const { isLg } = useMediaQuery();

  const isProSwap = location.pathname.includes("trade");

  const hideSearchBar = (isProSwap && !isLg) || (location.pathname === "/" && isLg);

  return (
    <header
      className={twMerge(
        "fixed bottom-0 lg:top-0 left-0 right-0 bg-transparent z-50 transition-all",
        isScrolled
          ? "lg:bg-surface-primary-rice lg:shadow-account-card"
          : "bg-transparent shadow-none",
        location.pathname === "/" ? "lg:fixed h-fit " : "lg:sticky flex flex-col items-center",
      )}
    >
      {isLg ? <div id="quest-banner" /> : null}
      {isLg ? <TestnetBanner /> : null}

      <div className="w-full gap-4 relative flex flex-wrap lg:flex-nowrap items-center justify-center xl:grid xl:grid-cols-4 max-w-[76rem] mx-auto p-4">
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
          {!hideSearchBar ? <SearchMenu /> : null}
          {isProSwap ? (
            <div
              id="trade-buttons"
              className="flex gap-2 items-center justify-center w-full lg:hidden"
            />
          ) : null}
          {!isSearchBarVisible ? (
            <IconButton
              onClick={() =>
                isConnected ? setSidebarVisibility(true) : navigate({ to: "/signin" })
              }
              variant="utility"
              size="lg"
              type="button"
              className="shadow-account-card lg:hidden"
            >
              <TxIndicator icon={<IconWallet className="w-6 h-6" />} />
            </IconButton>
          ) : null}
        </div>
        <div className="hidden lg:flex gap-2 items-center justify-end order-2 lg:order-3">
          <Button
            dng-connect-button="true"
            variant="utility"
            size="lg"
            onClick={() =>
              isConnected ? setSidebarVisibility(!isSidebarVisible) : navigate({ to: "/signin" })
            }
          >
            {isConnected ? (
              <div className="flex items-center justify-center gap-2">
                <TxIndicator icon={<IconWallet className="w-6 h-6" />} />
                <span className="italic font-exposure font-bold capitalize">
                  {account?.type} # {account?.index}
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
