import {
  Button,
  IconBell,
  IconGear,
  ProfileIcon,
  twMerge,
  useClickAway,
  useMediaQuery,
} from "@left-curve/applets-kit";

import { IconSearch } from "@left-curve/applets-kit";
import { useAccount } from "@left-curve/store-react";
import { Link, useNavigate } from "@tanstack/react-router";
import { useRef, useState } from "react";
import { useApp } from "~/hooks/useApp";
import { HamburgerMenu } from "./HamburguerMenu";
import { NotificationsMenu } from "./NotificationsMenu";
import { AccountDesktopMenu } from "./menu/AccountDesktopMenu";
import { AccountMobileMenu } from "./menu/AccountMobileMenu";
import { SearchMenuBody } from "./menu/SearchBody";
import { SearchMobileMenu } from "./menu/SearchMobileMenu";

import { AnimatePresence, motion } from "framer-motion";

export const Header: React.FC = () => {
  const { account, isConnected } = useAccount();
  const { setSidebarVisibility, setNotificationMenuVisibility, isNotificationMenuVisible } =
    useApp();
  const [searchVisible, setSearchVisible] = useState(false);
  const navigate = useNavigate();
  const isMd = useMediaQuery("md");
  const menuRef = useRef<HTMLDivElement>(null);

  useClickAway(menuRef, (e) => {
    if (!isMd) return;
    setSearchVisible(false);
  });

  const buttonNotificationsRef = useRef<HTMLButtonElement>(null);

  return (
    <>
      <header className="fixed  max-w-[76rem] mx-auto lg:sticky bottom-0 lg:top-0 left-0 pbg-transparent z-50 w-full p-4 backdrop-blur-sm">
        <div className="gap-4 flex flex-wrap lg:flex-nowrap items-center justify-center xl:grid xl:grid-cols-4 max-w-[76rem] mx-auto">
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
            <motion.div
              className="flex-col bg-rice-25 [box-shadow:0px_2px_6px_0px_#C7C2B666] rounded-md w-full flex items-center lg:absolute relative lg:-top-5"
              ref={menuRef}
            >
              <motion.div className="w-full flex items-center gap-2 px-3 py-2 rounded-md">
                <IconSearch className="w-5 h-5 text-gray-500" />
                <input className="bg-rice-25 pt-[4px] w-full outline-none focus:outline-none placeholder:text-gray-500" />
              </motion.div>
              {!searchVisible && (
                <AnimatePresence mode="wait" custom={searchVisible}>
                  <motion.button
                    type="button"
                    initial={{ opacity: 0 }}
                    animate={{ opacity: 1 }}
                    exit={{
                      opacity: 0,
                      transition: { duration: 0.2 },
                    }}
                    transition={{ duration: 1 }}
                    className="flex absolute w-full h-full bg-transparent left-0 rounded-md cursor-text gap-1 items-center pl-9 pt-1 diatype-m-regular"
                    onClick={() => setSearchVisible(!searchVisible)}
                  >
                    <span>Search for</span>{" "}
                    <span className="exposure-m-italic text-rice-800">apps</span>
                  </motion.button>
                </AnimatePresence>
              )}

              <AnimatePresence mode="wait" custom={searchVisible}>
                {isMd && searchVisible && (
                  <motion.div
                    layout
                    initial={{ height: 0 }}
                    animate={{ height: "auto" }}
                    exit={{ height: 0 }}
                    transition={{ duration: 0.3 }}
                    className="menu w-full overflow-hidden"
                  >
                    <SearchMenuBody />
                  </motion.div>
                )}
              </AnimatePresence>
            </motion.div>

            <HamburgerMenu />
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
      {isMd ? <AccountDesktopMenu /> : <AccountMobileMenu />}
      {isMd ? null : (
        <SearchMobileMenu isVisible={searchVisible} setVisibility={setSearchVisible} />
      )}
      <NotificationsMenu buttonRef={buttonNotificationsRef} />
    </>
  );
};
