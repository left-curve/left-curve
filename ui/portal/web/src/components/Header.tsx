import { Button, IconBell, IconGear, ProfileIcon, twMerge } from "@left-curve/applets-kit";

import { IconSearch } from "@left-curve/applets-kit";
import { useAccount } from "@left-curve/store-react";
import { Link, useNavigate } from "@tanstack/react-router";
import { useApp } from "~/hooks/useApp";
import { HamburgerMenu } from "./HamburguerMenu";
import { AccountMenu } from "./menu/AccountMenu";

export const Header: React.FC = () => {
  const { account, isConnected } = useAccount();
  const { setSidebarVisibility } = useApp();
  const navigate = useNavigate();

  return (
    <>
      <header className="fixed lg:sticky bottom-0 lg:top-0 left-0 bg-transparent z-50 w-full p-4 backdrop-blur-sm">
        <div className="gap-4 flex flex-wrap lg:flex-nowrap items-center justify-center xl:grid xl:grid-cols-4 max-w-[76rem] mx-auto">
          <Link to="/" className="w-fit">
            <img
              src="/images/dango.svg"
              alt="dango logo"
              className="h-8 order-1 cursor-pointer hidden lg:flex"
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
            <div className="bg-rice-25 [box-shadow:0px_2px_6px_0px_#C7C2B666] rounded-md w-full px-5 py-2 flex items-center gap-1">
              <IconSearch />
              <input
                placeholder="Search for apps"
                className="bg-rice-25 pt-[4px] w-full outline-none focus:outline-none placeholder:text-gray-500"
              />
            </div>

            <HamburgerMenu />
          </div>
          <div className="hidden lg:flex gap-2 items-center justify-end order-2 lg:order-3">
            <Button as={Link} variant="utility" size="lg" to="/settings">
              <IconGear className="w-6 h-6 text-rice-700" />
            </Button>

            {isConnected ? (
              <Button variant="utility" size="lg">
                <IconBell className="w-6 h-6 text-rice-700" />
              </Button>
            ) : null}
            <Button
              variant="utility"
              size="lg"
              onClick={() =>
                isConnected ? setSidebarVisibility(true) : navigate({ to: "/login" })
              }
            >
              {isConnected ? (
                <>
                  <ProfileIcon className="w-6 h-6" />
                  <span className="italic font-exposure font-bold capitalize">
                    {account?.type} #{account?.index}
                  </span>
                </>
              ) : (
                <span>Connect</span>
              )}
            </Button>

            {/* <MenuNotifications ref={menuNotificationsRef} />*/}
          </div>
        </div>
      </header>
      <AccountMenu />
    </>
  );
};
