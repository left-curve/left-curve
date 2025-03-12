import {
  Button,
  IconBell,
  IconGear,
  ProfileIcon,
  twMerge,
  useMediaQuery,
} from "@left-curve/applets-kit";

import { useAccount } from "@left-curve/store-react";
import { Link, useNavigate } from "@tanstack/react-router";
import { useRef } from "react";
import { useApp } from "~/hooks/useApp";
import { m } from "~/paraglide/messages";
import { HamburgerMenu } from "./HamburguerMenu";
import { NotificationsMenu } from "./NotificationsMenu";
import { AccountDesktopMenu } from "./menu/AccountDesktopMenu";
import { AccountMobileMenu } from "./menu/AccountMobileMenu";
import { SearchMenu } from "./menu/SearchMenu";

interface HeaderProps {
  isScrolled: boolean;
}

export const Header: React.FC<HeaderProps> = ({ isScrolled }) => {
  const { account, isConnected } = useAccount();

  const {
    setSidebarVisibility,
    setNotificationMenuVisibility,
    isNotificationMenuVisible,
    isSearchBarVisible,
  } = useApp();
  const navigate = useNavigate();
  const isLg = useMediaQuery("lg");
  const buttonNotificationsRef = useRef<HTMLButtonElement>(null);

  return (
    <header
      className={twMerge(
        "fixed lg:sticky bottom-0 lg:top-0 left-0 bg-transparent z-50 w-full transition-all",
        isScrolled ? "lg:bg-white-100 lg:shadow-card-shadow" : "bg-transparent shadow-none",
      )}
    >
      <div className="gap-4 flex flex-wrap lg:flex-nowrap items-center justify-center xl:grid xl:grid-cols-4 max-w-[76rem] mx-auto p-4">
        <Link to="/" className="w-fit">
          <img
            src="/favicon.svg"
            alt="dango logo"
            className="h-11 order-1 cursor-pointer hidden lg:flex rounded-full shadow-btn-shadow-gradient"
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
          <SearchMenu />
          {!isSearchBarVisible ? <HamburgerMenu /> : null}
        </div>
        <div className="hidden lg:flex gap-2 items-center justify-end order-2 lg:order-3">
          <Button as={Link} variant="utility" size="lg" to="/settings">
            <IconGear className="w-6 h-6 text-rice-700" />
          </Button>

          {isConnected ? (
            <Button
              ref={buttonNotificationsRef}
              variant="utility"
              size="lg"
              onClick={() => setNotificationMenuVisibility(!isNotificationMenuVisible)}
            >
              <IconBell className="w-6 h-6 text-rice-700" />
            </Button>
          ) : null}
          <Button
            variant="utility"
            size="lg"
            onClick={() => (isConnected ? setSidebarVisibility(true) : navigate({ to: "/login" }))}
          >
            {isConnected ? (
              <>
                <ProfileIcon className="w-6 h-6" />
                <span className="italic font-exposure font-bold capitalize">
                  {account?.type} #{account?.index}
                </span>
              </>
            ) : (
              <span>{m["common.signin"]()}</span>
            )}
          </Button>
        </div>
      </div>
      <NotificationsMenu buttonRef={buttonNotificationsRef} />
      {isLg ? <AccountDesktopMenu /> : <AccountMobileMenu />}
    </header>
  );
};
