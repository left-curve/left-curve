import {
  IconBell,
  IconGear,
  IconUser,
  ProfileIcon,
  type VisibleRef,
  twMerge,
} from "@left-curve/applets-kit";

import { useRef, useState } from "react";

import { IconSearch } from "@left-curve/applets-kit";
import { Link } from "@tanstack/react-router";
import { AccountMenu } from "./AccountMenu";
import { HamburgerMenu } from "./HamburguerMenu";

export const Header: React.FC = () => {
  const [showCommandBar, setShowCommandBar] = useState(false);

  const [showAccountMenu, setShowAccountMenu] = useState(false);

  const hamburgerRef = useRef<VisibleRef>(null);
  const menuNotificationsRef = useRef<VisibleRef>(null);

  return (
    <>
      <header className="sticky bottom-0 lg:top-0 left-0 bg-transparent z-50 w-full p-4 backdrop-blur-sm">
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
              { "p-0 ": showCommandBar },
              {
                "bottom-6":
                  window.matchMedia("(display-mode: standalone)").matches && !showCommandBar,
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

            <HamburgerMenu
              ref={hamburgerRef}
              isOpen={showCommandBar}
              onClose={() => setShowCommandBar(false)}
              openAccountMenu={() => setShowAccountMenu(true)}
              menuNotificationsRef={menuNotificationsRef}
            />
          </div>
          <div className="hidden lg:flex gap-2 items-center justify-end order-2 lg:order-3">
            <Link
              type="button"
              className="[box-shadow:0px_0px_8px_-2px_#FFFFFFA3_inset,_0px_3px_6px_-2px_#FFFFFFA3_inset,_0px_4px_6px_0px_#0000000A,_0px_4px_6px_0px_#0000000A] bg-rice-100 text-rice-700 border-[1px] border-solid [border-image-source:linear-gradient(180deg,_rgba(46,_37,_33,_0.06)_8%,_rgba(46,_37,_33,_0.12)_100%)] p-[10px] rounded-[14px]"
              to="/settings"
            >
              <IconGear className="w-6 h-6 text-rice-700" />
            </Link>
            <button
              type="button"
              className="[box-shadow:0px_0px_8px_-2px_#FFFFFFA3_inset,_0px_3px_6px_-2px_#FFFFFFA3_inset,_0px_4px_6px_0px_#0000000A,_0px_4px_6px_0px_#0000000A] bg-rice-100 text-rice-700 border-[1px] border-solid [border-image-source:linear-gradient(180deg,_rgba(46,_37,_33,_0.06)_8%,_rgba(46,_37,_33,_0.12)_100%)] p-[10px] rounded-[14px]"
            >
              <IconBell className="w-6 h-6 text-rice-700" />
            </button>
            <button
              type="button"
              onClick={() => setShowAccountMenu(!showAccountMenu)}
              className="[box-shadow:0px_0px_8px_-2px_#FFFFFFA3_inset,_0px_3px_6px_-2px_#FFFFFFA3_inset,_0px_4px_6px_0px_#0000000A,_0px_4px_6px_0px_#0000000A] bg-rice-100 text-rice-700 border-[1px] border-solid [border-image-source:linear-gradient(180deg,_rgba(46,_37,_33,_0.06)_8%,_rgba(46,_37,_33,_0.12)_100%)] p-[10px] pr-4 rounded-[14px] flex gap-2"
            >
              <ProfileIcon className="w-6 h-6" />
              <span className="italic font-exposure font-bold">Spot #1</span>
            </button>
            <AccountMenu
              showAccountMenu={showAccountMenu}
              setShowAccountMenu={setShowAccountMenu}
            />
            {/* <MenuNotifications ref={menuNotificationsRef} />*/}
          </div>
        </div>
      </header>
    </>
  );
};
