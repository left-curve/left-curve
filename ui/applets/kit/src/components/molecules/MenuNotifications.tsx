"use client";

import { Fragment, forwardRef, useImperativeHandle, useRef, useState } from "react";
import { twMerge } from "../../utils";

import { useClickAway } from "react-use";
import { BellIcon, Button, CloseIcon } from "../";
import type { VisibleRef } from "../../types";
import { CrossIcon } from "../icons/Cross";
import { NotificationCard } from "./NotificationCard";

const mockNotifications = [
  {
    title: "Tx Broadcasted",
    description: "Your transaction has been broadcasted to the Ethereum Mainnet.",
    txHash: "0x1234536",
  },
  {
    title: "Tx Confirmation",
    description: "Your transaction has been succesfully confirmed on the Ethereum Mainnet.",
    txHash: "0x123456",
  },
  {
    title: "Funds Reveived",
    description: "You have received funds on the Ethereum Mainnet.",
    txHash: "0x1234526",
  },
  {
    title: "Position Liquidated",
    description: "Description",
    txHash: "0x1234556",
  },
  {
    title: "Tx Failed",
    description: "Your transaction has failed on the Ethereum Mainnet.",
    txHash: "0x1234656",
  },
];

export const MenuNotifications = forwardRef<VisibleRef>((props, ref) => {
  const menuRef = useRef<HTMLDivElement>(null);
  const buttonRef = useRef<HTMLButtonElement>(null);
  const [showNotifications, setShowNotifications] = useState(false);

  useImperativeHandle(ref, () => ({
    isVisible: showNotifications,
    changeVisibility: (v) => setShowNotifications(v),
  }));

  useClickAway(menuRef, (e) => {
    if (buttonRef?.current?.contains(e.target as Node)) return;
    setShowNotifications(false);
  });

  return (
    <>
      <Button
        ref={buttonRef}
        aria-description="Notifications"
        onClick={() => setShowNotifications((prev) => !prev)}
        color="gray"
        className="p-4 hidden lg:flex rounded-[20px]"
      >
        <BellIcon className="h-5 w-5" />
      </Button>

      <div
        ref={menuRef}
        className={twMerge(
          "transition-all bg-surface-green-200 backdrop-blur-3xl w-full md:w-[19.5rem] fixed top-0 md:top-[72px] md:rounded-3xl p-4 md:p-2 md:py-4 flex flex-col gap-4 h-[100vh] md:h-fit md:max-h-[calc(100vh-78px)] z-50 duration-300 delay-100",
          showNotifications ? "right-0 md:right-4" : "right-[-100vh]",
        )}
      >
        <div className={twMerge("flex items-center justify-between md:hidden")}>
          <p className="text-2xl font-bold font-diatype-rounded mx-2 tracking-widest flex-1 text-typography-green-500">
            Notifications
          </p>
          <p
            className="p-2 bg-surface-green-300 rounded-xl text-typography-black-300 lg:hidden"
            onClick={() => setShowNotifications(false)}
          >
            <CrossIcon className="w-4 h-4" />
          </p>
        </div>
        <div className="flex flex-col gap-3 relative flex-1 scrollbar-none">
          {mockNotifications.map((notification, i) => (
            <Fragment key={notification.txHash}>
              <NotificationCard notification={notification} />
              <span className="last:hidden bg-surface-green-400 h-[1px] w-full" />
            </Fragment>
          ))}
        </div>
      </div>
    </>
  );
});
