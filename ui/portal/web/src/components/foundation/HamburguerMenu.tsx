import {
  BellIcon,
  DangoDotsIcon,
  Hamburger,
  IconButton,
  IconGear,
  ProfileIcon,
  type VisibleRef,
  twMerge,
  useClickAway,
} from "@left-curve/applets-kit";
import { capitalize } from "@left-curve/dango/utils";
import { useAccount } from "@left-curve/store";
import { useNavigate } from "@tanstack/react-router";
import { motion } from "framer-motion";
import { forwardRef, useRef, useState } from "react";
import { useApp } from "~/hooks/useApp";

export const HamburgerMenu = forwardRef<VisibleRef>((_props, ref) => {
  const { isConnected, account } = useAccount();
  const { setSidebarVisibility } = useApp();
  const [showOptions, setShowOptions] = useState(false);
  const navigate = useNavigate();
  const menuRef = useRef<HTMLDivElement>(null);

  useClickAway(menuRef, () => setShowOptions(false));

  return (
    <>
      <div
        ref={menuRef}
        className={twMerge("flex flex-col lg:hidden h-11 w-11 z-[80] transition-all duration-300", {
          "bottom-[2.75rem]": window.matchMedia("(display-mode: standalone)").matches,
        })}
      >
        <div
          className={twMerge("absolute flex w-fit right-4 gap-2 items-center transition-all", {
            "translate-y-[-15.25rem]": showOptions && isConnected,
            "translate-y-[-11.25rem]": showOptions && !isConnected,
          })}
          onClick={() => [
            setShowOptions(!showOptions),
            isConnected ? setSidebarVisibility(true) : navigate({ to: "/signin" }),
          ]}
        >
          <span className={twMerge("hidden exposure-m-italic text-white", { block: showOptions })}>
            {isConnected ? `${capitalize(account?.type as string)} #${account?.index}` : "Connect"}
          </span>
          <IconButton
            variant="utility"
            size="lg"
            type="button"
            className={twMerge("shadow-btn-shadow-gradient", { "shadow-none": !showOptions })}
          >
            <ProfileIcon className="h-6 w-6 " />
          </IconButton>
        </div>

        <div
          className={twMerge("absolute flex w-fit right-4 gap-2 items-center transition-all", {
            "translate-y-[-11.25rem]": showOptions && isConnected,
            hidden: showOptions && !isConnected,
          })}
          onClick={() => [setShowOptions(!showOptions), navigate({ to: "/notifications" })]}
        >
          <span className={twMerge("hidden exposure-m-italic text-white", { block: showOptions })}>
            Notifications
          </span>
          <IconButton
            variant="utility"
            size="lg"
            type="button"
            className={twMerge("shadow-btn-shadow-gradient", { "shadow-none": !showOptions })}
          >
            <BellIcon className="h-6 w-6 " />
          </IconButton>
        </div>

        <div
          className={twMerge("absolute flex w-fit right-4 gap-2 items-center transition-all", {
            "translate-y-[-7.5rem]": showOptions,
          })}
          onClick={() => [setShowOptions(!showOptions), navigate({ to: "/settings" })]}
        >
          <span className={twMerge("hidden exposure-m-italic text-white", { block: showOptions })}>
            Settings
          </span>
          <IconButton
            variant="utility"
            size="lg"
            type="button"
            className={twMerge("shadow-btn-shadow-gradient", { "shadow-none": !showOptions })}
          >
            <IconGear className="h-6 w-6 " />
          </IconButton>
        </div>

        <div
          className={twMerge("absolute flex w-fit right-4 gap-2 items-center transition-all", {
            "translate-y-[-3.75rem]": showOptions,
          })}
          onClick={() => [setShowOptions(!showOptions), navigate({ to: "/" })]}
        >
          <span className={twMerge("hidden exposure-m-italic text-white", { block: showOptions })}>
            Home
          </span>
          <IconButton
            variant="utility"
            size="lg"
            type="button"
            className={twMerge("shadow-btn-shadow-gradient", { "shadow-none": !showOptions })}
          >
            <DangoDotsIcon className="h-6 w-6 " />
          </IconButton>
        </div>

        <Hamburger isOpen={showOptions} onClick={() => setShowOptions(!showOptions)} />
      </div>
      {showOptions && (
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          className="fixed w-screen h-screen z-60 bg-gray-900/50 bottom-0 right-0"
        />
      )}
    </>
  );
});
