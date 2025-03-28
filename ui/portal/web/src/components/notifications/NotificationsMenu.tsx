import { Button, twMerge, useClickAway } from "@left-curve/applets-kit";
import { useRef } from "react";
import { useApp } from "~/hooks/useApp";

import { useNavigate } from "@tanstack/react-router";
import { m } from "~/paraglide/messages";
import { NotificationsList } from "./NotificationsList";

import type React from "react";

interface Props {
  buttonRef: React.RefObject<HTMLButtonElement>;
}

export const NotificationsMenu: React.FC<Props> = ({ buttonRef }) => {
  const { isNotificationMenuVisible, setNotificationMenuVisibility } = useApp();
  const menuRef = useRef<HTMLDivElement>(null);

  const navigate = useNavigate();

  useClickAway(menuRef, (e) => {
    if (buttonRef.current?.contains(e.target as Node)) return;
    setNotificationMenuVisibility(false);
  });

  return (
    <div
      ref={menuRef}
      className={twMerge(
        "hidden lg:block transition-all absolute top-[75px] bg-rice-50 shadow-card-shadow z-50 right-0 rounded-3xl w-[27rem] duration-200",
        isNotificationMenuVisible
          ? "scale-1 translate-y-0 translate-x-0"
          : "scale-0 -translate-y-1/2 translate-x-16",
      )}
    >
      <div className="p-4 flex items-center justify-between border-b border-b-gray-100">
        <h2 className="diatype-m-heavy">{m["notifications.title"]()}</h2>
        <Button
          variant="link"
          className="py-0 h-fit"
          onClick={() => [navigate({ to: "/notifications" }), setNotificationMenuVisibility(false)]}
        >
          {m["common.viewAll"]()}
        </Button>
      </div>
      <NotificationsList
        notifications={[]}
        className="max-h-[41rem] overflow-y-scroll scrollbar-none"
      />
    </div>
  );
};
