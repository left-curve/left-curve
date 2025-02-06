import {
  BellIcon,
  ConnectorIcon,
  Hamburger,
  ProfileIcon,
  type VisibleRef,
  twMerge,
  useClickAway,
} from "@left-curve/applets-kit";
import type React from "react";
import { forwardRef, useImperativeHandle, useRef, useState } from "react";

interface Props {
  isOpen?: boolean;
  onClose?: () => void;
  menuAccountsRef: React.RefObject<VisibleRef>;
  menuNotificationsRef: React.RefObject<VisibleRef>;
  menuConnectionsRef: React.RefObject<VisibleRef>;
}

export const HamburgerMenu = forwardRef<VisibleRef, Props>(
  ({ isOpen, menuAccountsRef, menuNotificationsRef, menuConnectionsRef, onClose }, ref) => {
    const [showOptions, setShowOptions] = useState(false);
    const menuRef = useRef<HTMLDivElement>(null);

    useImperativeHandle(ref, () => ({
      isVisible: showOptions,
      changeVisibility: (v) => setShowOptions(v),
    }));

    useClickAway(menuRef, () => setShowOptions(false));
    // className=""
    return (
      <div
        ref={menuRef}
        className={twMerge(
          "flex flex-col lg:hidden h-11 w-11 z-60 transition-all duration-300",
          { "bottom-4": isOpen },
          {
            "bottom-[2.75rem]": window.matchMedia("(display-mode: standalone)").matches && !isOpen,
          },
        )}
      >
        <div
          className={twMerge(
            "absolute flex flex-col items-center justify-center p-1 gap-1 h-11 w-11 cursor-pointer transition-all transform duration-300 rounded-[14px] bg-rice-100 text-rice-700",
            {
              "[box-shadow:0px_0px_8px_-2px_#FFFFFFA3_inset,_0px_3px_6px_-2px_#FFFFFFA3_inset,_0px_4px_6px_0px_#0000000A,_0px_4px_6px_0px_#0000000A]  border-[1px] border-solid [border-image-source:linear-gradient(180deg,_rgba(46,_37,_33,_0.06)_8%,_rgba(46,_37,_33,_0.12)_100%)] translate-y-[-11.25rem]":
                showOptions,
            },
          )}
          onClick={() => [
            setShowOptions(!showOptions),
            menuAccountsRef.current?.changeVisibility(true),
          ]}
        >
          <ProfileIcon className="h-6 w-6 text-surface-green-400" />
        </div>
        <div
          className={twMerge(
            "absolute flex flex-col items-center justify-center p-1 gap-1 h-11 w-11 cursor-pointer transition-all transform duration-300 rounded-[14px] bg-rice-100 text-rice-700",
            {
              "[box-shadow:0px_0px_8px_-2px_#FFFFFFA3_inset,_0px_3px_6px_-2px_#FFFFFFA3_inset,_0px_4px_6px_0px_#0000000A,_0px_4px_6px_0px_#0000000A]  border-[1px] border-solid [border-image-source:linear-gradient(180deg,_rgba(46,_37,_33,_0.06)_8%,_rgba(46,_37,_33,_0.12)_100%)] translate-y-[-7.5rem]":
                showOptions,
            },
          )}
          onClick={() => [
            setShowOptions(!showOptions),
            menuNotificationsRef.current?.changeVisibility(true),
          ]}
        >
          <BellIcon className="h-5 w-5 text-surface-green-400" />
        </div>
        <div
          className={twMerge(
            "absolute flex flex-col items-center justify-center p-1 gap-1 h-11 w-11 cursor-pointer transition-all transform duration-300 rounded-[14px] bg-rice-100 text-rice-700",
            {
              "[box-shadow:0px_0px_8px_-2px_#FFFFFFA3_inset,_0px_3px_6px_-2px_#FFFFFFA3_inset,_0px_4px_6px_0px_#0000000A,_0px_4px_6px_0px_#0000000A]  border-[1px] border-solid [border-image-source:linear-gradient(180deg,_rgba(46,_37,_33,_0.06)_8%,_rgba(46,_37,_33,_0.12)_100%)] translate-y-[-3.75rem]":
                showOptions,
            },
          )}
          onClick={() => [
            setShowOptions(!showOptions),
            menuConnectionsRef.current?.changeVisibility(true),
          ]}
        >
          <ConnectorIcon className="h-6 w-6 text-surface-green-400" />
        </div>
        {/* //[box-shadow:0px_0px_8px_-2px_#FFFFFFA3_inset,_0px_3px_6px_-2px_#FFFFFFA3_inset,_0px_4px_6px_0px_#0000000A,_0px_4px_6px_0px_#0000000A]
        bg-rice-100 text-rice-700 border-[1px] border-solid
        [border-image-source:linear-gradient(180deg,_rgba(46,_37,_33,_0.06)_8%,_rgba(46,_37,_33,_0.12)_100%)]
        p-[10px] rounded-[14px] */}
        <div
          className="flex flex-col items-center justify-center p-1 gap-1 h-11 w-11 z-[60] cursor-pointer rounded-[14px] border-[1px] border-solid
        [border-image-source:linear-gradient(180deg,_rgba(46,_37,_33,_0.06)_8%,_rgba(46,_37,_33,_0.12)_100%)] [box-shadow:0px_0px_8px_-2px_#FFFFFFA3_inset,_0px_3px_6px_-2px_#FFFFFFA3_inset,_0px_4px_6px_0px_#0000000A,_0px_4px_6px_0px_#0000000A]
        bg-rice-100 text-rice-700"
          hamburger-element="true"
          onClick={() => {
            if (isOpen) {
              onClose?.();
            } else {
              setShowOptions(!showOptions);
            }
          }}
        >
          <Hamburger isOpen={showOptions || Boolean(isOpen)} />
        </div>
      </div>
    );
  },
);
