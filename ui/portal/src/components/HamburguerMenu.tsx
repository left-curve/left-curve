import {
  BellIcon,
  ConnectorIcon,
  Hamburger,
  ProfileIcon,
  type VisibleRef,
  twMerge,
  useClickAway,
} from "@dango/shared";
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

    return (
      <div
        ref={menuRef}
        className={twMerge(
          "flex flex-col lg:hidden h-10 w-10 z-[60] transition-all fixed right-5 bottom-[1.25rem] duration-300",
          {
            "bottom-4": isOpen,
          },
        )}
      >
        <div
          className={twMerge(
            "absolute flex flex-col items-center justify-center p-1 gap-1 h-10 w-10 bg-surface-green-200 cursor-pointer rounded-xl transition-all transform duration-300",
            { "shadow-sm translate-y-[-11.25rem]": showOptions },
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
            "absolute flex flex-col items-center justify-center p-1 gap-1 h-10 w-10 bg-surface-green-200 cursor-pointer rounded-xl transition-all transform duration-300",
            { "shadow-sm translate-y-[-7.5rem]": showOptions },
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
            "absolute flex flex-col items-center justify-center p-1 gap-1 h-10 w-10 bg-surface-green-200 cursor-pointer rounded-xl transition-all transform duration-300",
            { "shadow-sm translate-y-[-3.75rem]": showOptions },
          )}
          onClick={() => [
            setShowOptions(!showOptions),
            menuConnectionsRef.current?.changeVisibility(true),
          ]}
        >
          <ConnectorIcon className="h-6 w-6 text-surface-green-400" />
        </div>
        <div
          className="flex flex-col items-center justify-center p-1 gap-1 h-10 w-10 bg-surface-green-300 rounded-xl z-[60] cursor-pointer"
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
