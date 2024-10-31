"use client";

import { Fragment, useRef, useState } from "react";
import { twMerge } from "../../utils";

import { useClickAway } from "react-use";
import { BellIcon, Button, CloseIcon, DangoButton } from "../";
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

export const MenuNotifications: React.FC = () => {
  const [showMenu, setShowMenu] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);
  const buttonRef = useRef<HTMLButtonElement>(null);

  useClickAway(menuRef, (e) => {
    if (buttonRef.current?.contains(e.target as Node)) return;
    setShowMenu(false);
  });

  return (
    <>
      <DangoButton
        ref={buttonRef}
        onClick={() => setShowMenu(!showMenu)}
        color="gray"
        radius="lg"
        isIconOnly
        className="font-bold"
      >
        <BellIcon className="h-6 w-6" />
      </DangoButton>

      <div
        ref={menuRef}
        className={twMerge(
          "transition-all bg-gray-200 backdrop-blur-3xl w-full md:w-[19.5rem] fixed top-0 md:top-[72px] md:rounded-3xl p-4 md:p-2 md:py-4 flex flex-col gap-4 h-[100vh] md:h-fit md:max-h-[calc(100vh-78px)] z-50",
          showMenu ? "right-0 md:right-4" : "right-[-100vh]",
        )}
      >
        <div className={twMerge("flex items-center justify-between md:hidden")}>
          <p className="text-2xl font-bold font-diatype-rounded mx-2 tracking-widest flex-1">
            Notifications
          </p>
          <div className="flex gap-2">
            <DangoButton isIconOnly radius="lg" onClick={() => setShowMenu(false)}>
              <CloseIcon className="h-6 w-6" />
            </DangoButton>
          </div>
        </div>
        <div className="flex flex-col gap-3 relative flex-1 scrollbar-none">
          {mockNotifications.map((notification, i) => (
            <Fragment key={notification.txHash}>
              <NotificationCard notification={notification} />
              <span className="last:hidden bg-black h-[1px] w-full" />
            </Fragment>
          ))}
        </div>
      </div>
    </>
  );
};
