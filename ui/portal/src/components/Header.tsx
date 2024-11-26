import {
  CommandBar,
  MenuAccounts,
  MenuConnections,
  MenuNotifications,
  type VisibleRef,
  twMerge,
} from "@dango/shared";

import { AccountType } from "@leftcurve/types";

import { useRef } from "react";
import { Link, useNavigate } from "react-router-dom";
import { applets } from "../../applets";
import { HamburgerMenu } from "./HamburguerMenu";

export const Header: React.FC = () => {
  const navigate = useNavigate();

  const commandBarRef = useRef<VisibleRef>(null);
  const hamburgerRef = useRef<VisibleRef>(null);
  const menuAccountsRef = useRef<VisibleRef>(null);
  const menuConnectionsRef = useRef<VisibleRef>(null);
  const menuNotificationsRef = useRef<VisibleRef>(null);

  return (
    <>
      <header className="sticky bottom-0 left-0 bg-transparent gap-4 z-50 flex flex-wrap lg:flex-nowrap items-center justify-center  w-full p-4 xl:grid xl:grid-cols-4">
        <Link to="/" className="w-fit">
          <img src="/images/dango.svg" alt="dango logo" className="h-8 order-1 cursor-pointer" />
        </Link>
        <div
          className={twMerge(
            "xl:col-span-2 z-50 min-w-full lg:min-w-0 flex-1 order-3 lg:order-2 flex items-end justify-center gap-2 fixed lg:relative lg:bottom-auto bottom-0 left-0 transition-all p-4 lg:p-0",
            { "p-0 ": commandBarRef.current?.isVisible },
          )}
        >
          <CommandBar
            applets={applets}
            action={({ path }) => navigate(path)}
            ref={commandBarRef}
            hamburgerRef={hamburgerRef}
          />
          <HamburgerMenu
            ref={hamburgerRef}
            commandBarRef={commandBarRef}
            menuAccountsRef={menuAccountsRef}
            menuConnectionsRef={menuConnectionsRef}
            menuNotificationsRef={menuNotificationsRef}
          />
        </div>
        <div className="flex gap-2 items-center justify-end order-2 lg:order-3">
          <MenuNotifications ref={menuNotificationsRef} />
          <MenuAccounts
            ref={menuAccountsRef}
            manageAction={(account) => navigate(`/accounts?address=${account.address}`)}
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
