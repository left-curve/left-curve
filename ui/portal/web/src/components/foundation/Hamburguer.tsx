import { twMerge, useClickAway } from "@left-curve/applets-kit";
import { useAccount } from "@left-curve/store";
import { useNavigate } from "@tanstack/react-router";
import { useRef, useState } from "react";
import { useApp } from "~/hooks/useApp";

import { capitalize } from "@left-curve/dango/utils";

import {
  IconBell,
  IconButton,
  IconDangoDots,
  IconGear,
  IconProfile,
} from "@left-curve/applets-kit";
import { motion } from "framer-motion";
import { TxIndicator } from "./TxIndicator";

export const Hamburger: React.FC = () => {
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
            <IconProfile className="h-6 w-6 " />
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
            <IconBell className="h-6 w-6 " />
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
            <IconDangoDots className="h-6 w-6 " />
          </IconButton>
        </div>

        <TxIndicator>
          <HamburgerButton isOpen={showOptions} onClick={() => setShowOptions(!showOptions)} />
        </TxIndicator>
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
};

type HamburgerButtonProps = {
  isOpen: boolean;
  className?: string;
  onClick: () => void;
};

const HamburgerButton: React.FC<HamburgerButtonProps> = ({ isOpen, className, onClick }) => {
  return (
    <IconButton
      variant="utility"
      size="lg"
      className={twMerge("relative group", className)}
      type="button"
      onClick={onClick}
    >
      <div className="relative flex overflow-hidden items-center justify-center transform transition-all duration-200">
        <div
          className={twMerge(
            "flex flex-col justify-between transform transition-all duration-200 origin-center overflow-hidden",
            isOpen ? "gap-2" : "gap-1",
          )}
        >
          <div
            className={twMerge(
              "bg-rice-700 h-[2px] w-4 rounded-xl transform transition-all duration-200 origin-left",
              { "translate-x-10": isOpen },
            )}
          />
          <div
            className={twMerge(
              "bg-rice-700 h-[2px] w-4 rounded-xl transform transition-all duration-200 delay-75",
              { "translate-x-10": isOpen },
            )}
          />
          <div
            className={twMerge(
              "bg-rice-700 h-[2px] w-4 rounded-xl transform transition-all duration-200 origin-left delay-150",
              { "translate-x-10": isOpen },
            )}
          />

          <div
            className={twMerge(
              "absolute items-center justify-between transform transition-all duration-300 top-2.5 -translate-x-10 flex w-0",
              { "translate-x-[-1.5px] translate-y-[1px] w-12": isOpen },
            )}
          >
            <div
              className={twMerge(
                "absolute bg-rice-700 h-[2px] w-5 rounded-full transform transition-all duration-300 rotate-0 delay-200",
                { "rotate-45": isOpen },
              )}
            />
            <div
              className={twMerge(
                "absolute bg-rice-700 h-[2px] w-5 rounded-full transform transition-all duration-300 -rotate-0 delay-200",
                { "-rotate-45": isOpen },
              )}
            />
          </div>
        </div>
      </div>
    </IconButton>
  );
};
