import {
  CommandBar,
  MenuAccounts,
  MenuConnections,
  MenuNotifications,
  type VisibleRef,
  twMerge,
} from "@left-curve/applets-kit";

import { AccountType } from "@left-curve/types";

import { useEffect, useRef, useState } from "react";

import { Link, useNavigate, useSearch } from "@tanstack/react-router";
import { applets } from "../../applets";
import { HamburgerMenu } from "./HamburguerMenu";

export const Header: React.FC = () => {
  const navigate = useNavigate({ from: "." });
  const [showCommandBar, setShowCommandBar] = useState(false);

  const hamburgerRef = useRef<VisibleRef>(null);
  const menuAccountsRef = useRef<VisibleRef>(null);
  const menuConnectionsRef = useRef<VisibleRef>(null);
  const menuNotificationsRef = useRef<VisibleRef>(null);

  const search = useSearch({ strict: false });
  const { showAccounts } = search;

  useEffect(() => {
    if (showAccounts) {
      menuAccountsRef.current?.changeVisibility(true);
      navigate({ search: () => ({}) });
    }
  }, [showAccounts]);

  return (
    <>
      <header className="sticky bottom-0 left-0 bg-transparent gap-4 z-50 flex flex-wrap lg:flex-nowrap items-center justify-center  w-full p-4 xl:grid xl:grid-cols-4">
        <Link to="/" className="w-fit">
          <img src="/images/dango.svg" alt="dango logo" className="h-8 order-1 cursor-pointer" />
        </Link>
        <div
          className={twMerge(
            "xl:col-span-2 z-50 min-w-full lg:min-w-0 flex-1 order-3 lg:order-2 flex items-end justify-center gap-2 fixed lg:relative bottom-0 lg:bottom-auto left-0 transition-all p-4 lg:p-0",
            { "p-0 ": showCommandBar },
            {
              "bottom-6":
                window.matchMedia("(display-mode: standalone)").matches && !showCommandBar,
            },
          )}
        >
          <CommandBar
            applets={applets}
            action={({ path }) => navigate({ to: path, from: "/" })}
            changeVisibility={setShowCommandBar}
            isVisible={showCommandBar}
            hamburgerRef={hamburgerRef}
          />
          <HamburgerMenu
            ref={hamburgerRef}
            isOpen={showCommandBar}
            onClose={() => setShowCommandBar(false)}
            menuAccountsRef={menuAccountsRef}
            menuConnectionsRef={menuConnectionsRef}
            menuNotificationsRef={menuNotificationsRef}
          />
        </div>
        <div className="flex gap-2 items-center justify-end order-2 lg:order-3">
          <MenuNotifications ref={menuNotificationsRef} />
          <MenuAccounts
            ref={menuAccountsRef}
            manageAction={(account) => navigate({ to: `/accounts?address=${account.address}` })}
            images={{
              [AccountType.Spot]: "/images/avatars/spot.svg",
              [AccountType.Margin]: "/images/avatars/margin.svg",
              [AccountType.Safe]: "/images/avatars/safe.svg",
            }}
          />
          <MenuConnections ref={menuConnectionsRef} />
        </div>
      </header>
    </>
  );
};
